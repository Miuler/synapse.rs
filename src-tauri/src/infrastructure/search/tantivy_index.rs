use crate::domain::models::full_text::{
    DocumentKind, FullTextDocument, FullTextHit, FullTextQuery, HighlightSegment,
};
use crate::domain::services::full_text_index::FullTextIndex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use tantivy::collector::{Count, TopDocs};
use tantivy::directory::MmapDirectory;
use tantivy::query::{BooleanQuery, Occur, Query, QueryParser, RegexQuery};
use tantivy::schema::{
    Field, IndexRecordOption, Schema, TextFieldIndexing, TextOptions, Value, FAST, INDEXED,
    STORED, STRING, TEXT,
};
use tantivy::snippet::SnippetGenerator;
use tantivy::tokenizer::{
    AsciiFoldingFilter, LowerCaser, RemoveLongFilter, SimpleTokenizer, Stemmer, TextAnalyzer,
};
use tantivy::{doc, DocAddress, Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument, Term};

pub const SCHEMA_VERSION: &str = "1";

#[derive(Clone, Copy)]
pub struct TantivyFields {
    pub path_id: Field,
    pub path: Field,
    pub title: Field,
    pub body: Field,
    pub kind: Field,
    pub mtime: Field,
}

pub struct TantivyFullTextIndex {
    index: Index,
    reader: IndexReader,
    writer: Mutex<IndexWriter>,
    fields: TantivyFields,
    in_memory: bool,
}

impl TantivyFullTextIndex {
    pub fn is_in_memory(&self) -> bool {
        self.in_memory
    }

    /// Construye el esquema tantivy y registra los tokenizers personalizados.
    pub fn build_schema() -> (Schema, TantivyFields) {
        let mut builder = Schema::builder();

        // 1. Clave primaria única para reemplazos y eliminaciones
        let path_id = builder.add_text_field("path_id", STRING | STORED);

        // 2. Ruta indexada con tokenizer estándar para búsquedas tipo `path:carpeta`
        let path = builder.add_text_field("path", TEXT | STORED);

        // 3. Título y cuerpo indexados con el tokenizer español plegado a ASCII
        let text_indexing = TextFieldIndexing::default()
            .set_tokenizer("es_fold")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);
        let es_text_options = TextOptions::default()
            .set_indexing_options(text_indexing)
            .set_stored();

        let title = builder.add_text_field("title", es_text_options.clone());
        let body = builder.add_text_field("body", es_text_options);

        // 4. Tipo de documento para filtros `kind:code`, etc.
        let kind = builder.add_text_field("kind", STRING | STORED);

        // 5. Fecha de modificación para sincronización incremental
        let mtime = builder.add_u64_field("mtime", INDEXED | STORED | FAST);

        let schema = builder.build();
        let fields = TantivyFields {
            path_id,
            path,
            title,
            body,
            kind,
            mtime,
        };

        (schema, fields)
    }

    fn register_tokenizers(index: &Index) {
        let es_fold = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(RemoveLongFilter::limit(40))
            .filter(LowerCaser)
            .filter(Stemmer::new(tantivy::tokenizer::Language::Spanish))
            .filter(AsciiFoldingFilter)
            .build();

        index.tokenizers().register("es_fold", es_fold);
    }

    /// Abre o crea un índice persistente en el disco utilizando `MmapDirectory`.
    /// Si la versión del esquema no coincide o el directorio está corrupto, lo recrea limpiamente.
    pub fn open_or_create(dir: &Path) -> Result<Self, String> {
        let schema_file = dir.join("SCHEMA_VERSION");
        let should_recreate = if schema_file.exists() {
            match fs::read_to_string(&schema_file) {
                Ok(v) => v.trim() != SCHEMA_VERSION,
                Err(_) => true,
            }
        } else {
            // Si el directorio tiene contenido pero no SCHEMA_VERSION, limpiar
            dir.exists() && fs::read_dir(dir).map(|mut d| d.next().is_some()).unwrap_or(false)
        };

        if should_recreate {
            let _ = fs::remove_dir_all(dir);
        }

        fs::create_dir_all(dir).map_err(|e| format!("Error al crear directorio fts {:?}: {}", dir, e))?;

        let (schema, fields) = Self::build_schema();

        let mmap_dir = MmapDirectory::open(dir)
            .map_err(|e| format!("Error abriendo MmapDirectory en {:?}: {}", dir, e))?;

        let index = Index::open_or_create(mmap_dir, schema.clone())
            .map_err(|e| format!("Error creando/abriendo índice tantivy: {}", e))?;

        Self::register_tokenizers(&index);

        let writer = index
            .writer(50_000_000)
            .map_err(|e| format!("Error al obtener IndexWriter: {}", e))?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()
            .map_err(|e| format!("Error al crear IndexReader: {}", e))?;

        let _ = fs::write(&schema_file, SCHEMA_VERSION);

        Ok(Self {
            index,
            reader,
            writer: Mutex::new(writer),
            fields,
            in_memory: false,
        })
    }

    /// Crea un índice en memoria RAM (ideal para tests o como fallback si el disco está bloqueado).
    pub fn in_memory() -> Result<Self, String> {
        let (schema, fields) = Self::build_schema();
        let index = Index::create_in_ram(schema);
        Self::register_tokenizers(&index);

        let writer = index
            .writer(25_000_000)
            .map_err(|e| format!("Error al obtener IndexWriter en RAM: {}", e))?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()
            .map_err(|e| format!("Error al crear IndexReader en RAM: {}", e))?;

        Ok(Self {
            index,
            reader,
            writer: Mutex::new(writer),
            fields,
            in_memory: true,
        })
    }

    /// Construye la consulta de tantivy combinando `QueryParser` con soporte de prefijo en el último término.
    fn build_query(&self, query: &FullTextQuery) -> Result<Box<dyn Query>, String> {
        let trimmed = query.text.trim();
        if trimmed.is_empty() {
            return Err("Consulta vacía".to_string());
        }

        let mut query_parser = QueryParser::for_index(
            &self.index,
            vec![self.fields.title, self.fields.body, self.fields.path],
        );
        query_parser.set_conjunction_by_default();
        query_parser.set_field_boost(self.fields.title, 2.5);

        // Parseo con el QueryParser estándar
        let (parsed_query, _) = query_parser.parse_query_lenient(trimmed);

        // Si no se requiere prefijo o el texto termina en espacio, usar la consulta parseada directamente
        if !query.prefix_last_term || query.text.ends_with(' ') {
            return Ok(parsed_query);
        }

        // Buscar el último token
        let last_word = trimmed.split_whitespace().last().unwrap_or("");
        // Verificar que el token sea simple y apto para búsqueda por prefijo (>= 2 caracteres alfanuméricos)
        let is_simple_prefix = last_word.len() >= 2
            && !last_word.contains(':')
            && !last_word.contains('"')
            && !last_word.contains('*')
            && !last_word.starts_with('-')
            && !last_word.starts_with('+');

        if !is_simple_prefix {
            return Ok(parsed_query);
        }

        // Crear una consulta regex de prefijo case-insensitive plegada
        let clean_token: String = last_word
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect();

        if clean_token.is_empty() {
            return Ok(parsed_query);
        }

        let pattern = format!("{}.*", clean_token.to_lowercase());

        let title_regex = RegexQuery::from_pattern(&pattern, self.fields.title).ok();
        let body_regex = RegexQuery::from_pattern(&pattern, self.fields.body).ok();

        match (title_regex, body_regex) {
            (Some(tr), Some(br)) => {
                let prefix_clause = Box::new(BooleanQuery::new(vec![
                    (Occur::Should, Box::new(tr) as Box<dyn Query>),
                    (Occur::Should, Box::new(br) as Box<dyn Query>),
                ]));

                // Si hay palabras anteriores, combinamos con AND (Must)
                let preceding: Vec<&str> = trimmed.split_whitespace().collect();
                if preceding.len() > 1 {
                    let preceding_text = preceding[..preceding.len() - 1].join(" ");
                    let (prec_query, _) = query_parser.parse_query_lenient(&preceding_text);
                    let combined = BooleanQuery::new(vec![
                        (Occur::Must, prec_query),
                        (Occur::Must, prefix_clause),
                    ]);
                    Ok(Box::new(combined))
                } else {
                    // Si es solo una palabra, unimos con Should la consulta normal y el prefijo
                    let combined = BooleanQuery::new(vec![
                        (Occur::Should, parsed_query),
                        (Occur::Should, prefix_clause),
                    ]);
                    Ok(Box::new(combined))
                }
            }
            _ => Ok(parsed_query),
        }
    }
}

impl FullTextIndex for TantivyFullTextIndex {
    fn upsert(&self, docs: Vec<FullTextDocument>) -> Result<(), String> {
        let writer = self.writer.lock().map_err(|e| e.to_string())?;

        for doc in docs {
            let term = Term::from_field_text(self.fields.path_id, &doc.path);
            writer.delete_term(term);

            let tantivy_doc = doc!(
                self.fields.path_id => doc.path.as_str(),
                self.fields.path => doc.path.as_str(),
                self.fields.title => doc.title.as_str(),
                self.fields.body => doc.body.as_str(),
                self.fields.kind => doc.kind.as_str(),
                self.fields.mtime => doc.mtime_nanos,
            );

            writer
                .add_document(tantivy_doc)
                .map_err(|e| format!("Error agregando documento {:?}: {}", doc.path, e))?;
        }

        Ok(())
    }

    fn delete(&self, paths: &[String]) -> Result<(), String> {
        let writer = self.writer.lock().map_err(|e| e.to_string())?;
        for path in paths {
            let term = Term::from_field_text(self.fields.path_id, path);
            writer.delete_term(term);
        }
        Ok(())
    }

    fn delete_all(&self) -> Result<(), String> {
        let writer = self.writer.lock().map_err(|e| e.to_string())?;
        writer
            .delete_all_documents()
            .map_err(|e| format!("Error eliminando documentos: {}", e))?;
        Ok(())
    }

    fn commit(&self) -> Result<(), String> {
        let mut writer = self.writer.lock().map_err(|e| e.to_string())?;
        writer
            .commit()
            .map_err(|e| format!("Error en commit de tantivy: {}", e))?;
        self.reader
            .reload()
            .map_err(|e| format!("Error recargando index reader: {}", e))?;
        Ok(())
    }

    fn search(&self, query: &FullTextQuery) -> Result<(Vec<FullTextHit>, usize), String> {
        let searcher = self.reader.searcher();
        let tantivy_query = self.build_query(query)?;

        let (top_docs, total_hits) = searcher
            .search(
                &*tantivy_query,
                &(TopDocs::with_limit(query.limit).order_by_score(), Count),
            )
            .map_err(|e| format!("Error en búsqueda tantivy: {}", e))?;

        let mut body_snippet_gen =
            SnippetGenerator::create(&searcher, &*tantivy_query, self.fields.body)
                .map_err(|e| format!("Error creando body snippet generator: {}", e))?;
        body_snippet_gen.set_max_num_chars(160);

        let mut title_snippet_gen =
            SnippetGenerator::create(&searcher, &*tantivy_query, self.fields.title)
                .map_err(|e| format!("Error creando title snippet generator: {}", e))?;
        title_snippet_gen.set_max_num_chars(200);

        let mut hits = Vec::with_capacity(top_docs.len());

        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| format!("Error leyendo doc {:?}: {}", doc_address, e))?;

            let path = doc
                .get_first(self.fields.path_id)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let title_str = doc
                .get_first(self.fields.title)
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            let body_str = doc
                .get_first(self.fields.body)
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            let kind_str = doc
                .get_first(self.fields.kind)
                .and_then(|v| v.as_str())
                .unwrap_or("markdown");
            let kind = DocumentKind::parse(kind_str).unwrap_or(DocumentKind::Markdown);

            // Generar fragmentos y resaltado para título
            let title_snippet = title_snippet_gen.snippet(title_str);
            let title_segments = if title_snippet.highlighted().is_empty() {
                vec![HighlightSegment {
                    text: title_str.to_string(),
                    highlighted: false,
                }]
            } else {
                snippet_to_segments(title_snippet.fragment(), title_snippet.highlighted())
            };

            // Generar fragmentos y resaltado para cuerpo
            let body_snippet = body_snippet_gen.snippet(body_str);
            let snippet_segments = if body_snippet.fragment().is_empty() {
                let fallback_preview = if body_str.chars().count() > 160 {
                    let truncated: String = body_str.chars().take(160).collect();
                    format!("{}...", truncated)
                } else {
                    body_str.to_string()
                };
                vec![HighlightSegment {
                    text: fallback_preview,
                    highlighted: false,
                }]
            } else {
                snippet_to_segments(body_snippet.fragment(), body_snippet.highlighted())
            };

            // Extraer términos que coincidieron para scroll en el editor
            let mut matched_terms = Vec::new();
            for seg in title_segments.iter().chain(snippet_segments.iter()) {
                if seg.highlighted {
                    let term = seg.text.trim();
                    if !term.is_empty()
                        && !matched_terms.iter().any(|t: &String| t.eq_ignore_ascii_case(term))
                    {
                        matched_terms.push(term.to_string());
                    }
                }
            }

            hits.push(FullTextHit {
                note_path: path,
                title: title_segments,
                snippet: snippet_segments,
                matched_terms,
                kind,
                score,
            });
        }

        Ok((hits, total_hits))
    }

    fn indexed_versions(&self) -> Result<HashMap<String, u64>, String> {
        let searcher = self.reader.searcher();
        let mut versions = HashMap::new();

        for (segment_ord, segment_reader) in searcher.segment_readers().iter().enumerate() {
            for doc_id in segment_reader.doc_ids_alive() {
                if let Ok(doc) = searcher.doc::<TantivyDocument>(DocAddress::new(segment_ord as u32, doc_id)) {
                    if let Some(path) = doc.get_first(self.fields.path_id).and_then(|v| v.as_str()) {
                        let mtime = doc.get_first(self.fields.mtime).and_then(|v| v.as_u64()).unwrap_or(0);
                        versions.insert(path.to_string(), mtime);
                    }
                }
            }
        }

        Ok(versions)
    }

    fn num_docs(&self) -> u64 {
        self.reader.searcher().num_docs()
    }
}

/// Convierte un fragmento de texto y sus rangos de resaltado en segmentos ordenados.
pub fn snippet_to_segments(
    fragment: &str,
    ranges: &[std::ops::Range<usize>],
) -> Vec<HighlightSegment> {
    if ranges.is_empty() {
        return vec![HighlightSegment {
            text: fragment.to_string(),
            highlighted: false,
        }];
    }

    let mut segments = Vec::new();
    let mut current_idx = 0;

    for r in ranges {
        let start = r.start.min(fragment.len());
        let end = r.end.min(fragment.len());

        if start > current_idx && current_idx < fragment.len() {
            if let Some(slice) = fragment.get(current_idx..start) {
                if !slice.is_empty() {
                    segments.push(HighlightSegment {
                        text: slice.to_string(),
                        highlighted: false,
                    });
                }
            }
        }

        if start < end {
            if let Some(slice) = fragment.get(start..end) {
                if !slice.is_empty() {
                    segments.push(HighlightSegment {
                        text: slice.to_string(),
                        highlighted: true,
                    });
                }
            }
        }

        current_idx = end.max(current_idx);
    }

    if current_idx < fragment.len() {
        if let Some(slice) = fragment.get(current_idx..) {
            if !slice.is_empty() {
                segments.push(HighlightSegment {
                    text: slice.to_string(),
                    highlighted: false,
                });
            }
        }
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tantivy_index_upsert_and_search() {
        let index = TantivyFullTextIndex::in_memory().unwrap();

        let doc1 = FullTextDocument {
            path: "nota1.md".to_string(),
            title: "Arquitectura Cebolla".to_string(),
            body: "Synapse implementa arquitectura cebolla en Rust con tantivy.".to_string(),
            kind: DocumentKind::Markdown,
            mtime_nanos: 1000,
        };

        let doc2 = FullTextDocument {
            path: "canciones.md".to_string(),
            title: "Lista de Canciones".to_string(),
            body: "Esta canción es hermosa y melódica.".to_string(),
            kind: DocumentKind::Markdown,
            mtime_nanos: 2000,
        };

        index.upsert(vec![doc1, doc2]).unwrap();
        index.commit().unwrap();

        assert_eq!(index.num_docs(), 2);

        // Búsqueda simple
        let q = FullTextQuery {
            text: "cebolla".to_string(),
            limit: 10,
            prefix_last_term: true,
        };
        let (hits, count) = index.search(&q).unwrap();
        assert_eq!(count, 1);
        assert_eq!(hits[0].note_path, "nota1.md");

        // Stemming y acentos en español: "cancion" busca "canciones" y "canción"
        let q_stem = FullTextQuery {
            text: "cancion".to_string(),
            limit: 10,
            prefix_last_term: false,
        };
        let (hits_stem, count_stem) = index.search(&q_stem).unwrap();
        assert_eq!(count_stem, 1);
        assert_eq!(hits_stem[0].note_path, "canciones.md");

        // Búsqueda por prefijo mientras se escribe
        let q_prefix = FullTextQuery {
            text: "arqui".to_string(),
            limit: 10,
            prefix_last_term: true,
        };
        let (hits_pre, count_pre) = index.search(&q_prefix).unwrap();
        assert_eq!(count_pre, 1);
        assert_eq!(hits_pre[0].note_path, "nota1.md");

        // Re-upsert (idempotencia)
        let doc1_updated = FullTextDocument {
            path: "nota1.md".to_string(),
            title: "Arquitectura Cebolla Actualizada".to_string(),
            body: "Nueva información sobre arquitectura cebolla.".to_string(),
            kind: DocumentKind::Markdown,
            mtime_nanos: 1500,
        };
        index.upsert(vec![doc1_updated]).unwrap();
        index.commit().unwrap();
        assert_eq!(index.num_docs(), 2);

        // Delete
        index.delete(&["nota1.md".to_string()]).unwrap();
        index.commit().unwrap();
        assert_eq!(index.num_docs(), 1);
    }
}
