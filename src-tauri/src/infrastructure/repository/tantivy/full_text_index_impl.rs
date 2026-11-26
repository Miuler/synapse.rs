use crate::domain::search::full_text::{
    DocumentKind, FullTextDocument, FullTextHit, FullTextQuery, HighlightSegment, LineOptions,
    MatchLocation, MatchedLine,
};
use crate::domain::search::full_text_index::FullTextIndex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use tantivy::collector::{Count, TopDocs};
use tantivy::directory::MmapDirectory;
use tantivy::query::{BooleanQuery, Occur, Query, QueryParser, RegexQuery};
use tantivy::schema::{
    FAST, Field, INDEXED, IndexRecordOption, STORED, STRING, Schema, TEXT, TextFieldIndexing,
    TextOptions, Value,
};
use tantivy::snippet::SnippetGenerator;
use tantivy::tokenizer::{
    AsciiFoldingFilter, LowerCaser, RemoveLongFilter, Stemmer, TextAnalyzer,
};
use tantivy::{
    DocSet, Index, IndexReader, IndexWriter, ReloadPolicy, TERMINATED, TantivyDocument, Term, doc,
};

/// Subirla obliga a reconstruir el índice (4: extractores de draw.io y ArchiMate; 5: XML genérico
/// e identificadores partidos; 6: extractor y tipo propio para BPMN; 7: palabras y entidades de Word).
pub const SCHEMA_VERSION: &str = "7";

#[derive(Clone, Copy)]
pub struct TantivyFields {
    pub path_id: Field,
    pub path: Field,
    pub title: Field,
    pub headings: Field,
    pub body: Field,
    pub kind: Field,
    pub mtime: Field,
}

pub struct FullTextIndexImpl {
    index: Index,
    reader: IndexReader,
    /// `None` en un índice de solo lectura (el de otra bóveda, que escribe su propia instancia).
    writer: Option<Mutex<IndexWriter>>,
    fields: TantivyFields,
    in_memory: bool,
}

impl FullTextIndexImpl {
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

        // Encabezados de la nota: para puntuar y filtrar (`headings:xxx`). Se guardan para saber
        // qué línea del cuerpo es el encabezado buscado y mostrarla en el fragmento.
        let headings_indexing = TextFieldIndexing::default()
            .set_tokenizer("es_fold")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);
        let headings = builder.add_text_field(
            "headings",
            TextOptions::default()
                .set_indexing_options(headings_indexing)
                .set_stored(),
        );

        // 4. Tipo de documento para filtros `kind:code`, etc.
        let kind = builder.add_text_field("kind", STRING | STORED);

        // 5. Fecha de modificación para sincronización incremental
        let mtime = builder.add_u64_field("mtime", INDEXED | STORED | FAST);

        let schema = builder.build();
        let fields = TantivyFields {
            path_id,
            path,
            title,
            headings,
            body,
            kind,
            mtime,
        };

        (schema, fields)
    }

    fn register_tokenizers(index: &Index) {
        // Identificadores (camelCase, letras+números) también se indexan por partes
        let es_fold = TextAnalyzer::builder(super::identifier_tokenizer::IdentifierTokenizer::default())
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
            dir.exists()
                && fs::read_dir(dir)
                    .map(|mut d| d.next().is_some())
                    .unwrap_or(false)
        };

        if should_recreate {
            let _ = fs::remove_dir_all(dir);
        }

        fs::create_dir_all(dir)
            .map_err(|e| format!("Error al crear directorio fts {:?}: {}", dir, e))?;

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
            writer: Some(Mutex::new(writer)),
            fields,
            in_memory: false,
        })
    }

    /// Abre el índice de otra bóveda solo para buscar: no lo crea ni lo recrea, no toma el
    /// bloqueo de escritura (lo tiene la instancia de esa bóveda) y ve sus cambios al confirmarse.
    pub fn open_read_only(dir: &Path) -> Result<Self, String> {
        if !dir.join("SCHEMA_VERSION").exists() {
            return Err(format!(
                "La bóveda no tiene índice de búsqueda en {:?}",
                dir
            ));
        }
        let (_, fields) = Self::build_schema();
        let mmap_dir = MmapDirectory::open(dir)
            .map_err(|e| format!("Error abriendo MmapDirectory en {:?}: {}", dir, e))?;
        let index =
            Index::open(mmap_dir).map_err(|e| format!("Error abriendo índice tantivy: {}", e))?;
        Self::register_tokenizers(&index);
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| format!("Error al crear IndexReader: {}", e))?;
        Ok(Self {
            index,
            reader,
            writer: None,
            fields,
            in_memory: false,
        })
    }

    fn writer(&self) -> Result<std::sync::MutexGuard<'_, IndexWriter>, String> {
        self.writer
            .as_ref()
            .ok_or_else(|| "Índice de solo lectura".to_string())?
            .lock()
            .map_err(|e| e.to_string())
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
            writer: Some(Mutex::new(writer)),
            fields,
            in_memory: true,
        })
    }

    /// `QueryParser` de los campos de texto, con AND por defecto y el peso de título y encabezados.
    fn query_parser(&self) -> QueryParser {
        let mut query_parser = QueryParser::for_index(
            &self.index,
            vec![
                self.fields.title,
                self.fields.headings,
                self.fields.body,
                self.fields.path,
            ],
        );
        query_parser.set_conjunction_by_default();
        query_parser.set_field_boost(self.fields.title, 2.5);
        query_parser.set_field_boost(self.fields.headings, 1.5);
        query_parser
    }

    /// Construye la consulta de tantivy combinando `QueryParser` con soporte de prefijo en el último término.
    fn build_query(&self, query: &FullTextQuery) -> Result<Box<dyn Query>, String> {
        self.build_query_with_prefix(query).map(|(q, _)| q)
    }

    /// Como `build_query`, y además el prefijo (ya en minúsculas) que se buscó para la última
    /// palabra, si se buscó: hace falta para saber qué líneas coinciden.
    fn build_query_with_prefix(
        &self,
        query: &FullTextQuery,
    ) -> Result<(Box<dyn Query>, Option<String>), String> {
        let expanded = normalize_operators(&expand_field_aliases(query.text.trim()));
        let trimmed = expanded.as_str();
        if trimmed.is_empty() {
            return Err("Consulta vacía".to_string());
        }

        let query_parser = self.query_parser();

        // Parseo con el QueryParser estándar
        let (parsed_query, _) = query_parser.parse_query_lenient(trimmed);

        // Sin prefijo si no se pide, si el texto termina en espacio o si lleva operadores: con
        // `a OR b` el prefijo partía la consulta en «a OR» y «b» y exigía las dos
        if !query.prefix_last_term || query.text.ends_with(' ') || has_operators(trimmed) {
            return Ok((parsed_query, None));
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
            return Ok((parsed_query, None));
        }

        // Crear una consulta regex de prefijo case-insensitive plegada
        let clean_token: String = last_word.chars().filter(|c| c.is_alphanumeric()).collect();

        if clean_token.is_empty() {
            return Ok((parsed_query, None));
        }

        let prefix = clean_token.to_lowercase();
        let pattern = format!("{}.*", prefix);

        let title_regex = RegexQuery::from_pattern(&pattern, self.fields.title).ok();
        let headings_regex = RegexQuery::from_pattern(&pattern, self.fields.headings).ok();
        let body_regex = RegexQuery::from_pattern(&pattern, self.fields.body).ok();

        match (title_regex, headings_regex, body_regex) {
            (Some(tr), Some(hr), Some(br)) => {
                let prefix_clause = Box::new(BooleanQuery::new(vec![
                    (Occur::Should, Box::new(tr) as Box<dyn Query>),
                    (Occur::Should, Box::new(hr) as Box<dyn Query>),
                    (Occur::Should, Box::new(br) as Box<dyn Query>),
                    // El índice guarda raíces ("para" → "par"), que el regex de prefijo no
                    // alcanza: la palabra completa también debe coincidir.
                    (Occur::Should, query_parser.parse_query_lenient(last_word).0),
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
                    Ok((Box::new(combined), Some(prefix)))
                } else {
                    // Si es solo una palabra, unimos con Should la consulta normal y el prefijo
                    let combined = BooleanQuery::new(vec![
                        (Occur::Should, parsed_query),
                        (Occur::Should, prefix_clause),
                    ]);
                    Ok((Box::new(combined), Some(prefix)))
                }
            }
            _ => Ok((parsed_query, None)),
        }
    }

    /// Consulta de candidatos del modo exacto: cada texto como frase, combinadas con OR.
    fn exact_candidates_query(&self, needles: &[String]) -> Result<Box<dyn Query>, String> {
        let parser = self.query_parser();
        let mut clauses: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        for needle in needles {
            if !needle.chars().any(char::is_alphanumeric) {
                return Err(format!(
                    "El modo exacto necesita al menos una palabra con letras o números en cada texto («{needle}»)"
                ));
            }
            let phrase = format!("\"{}\"", needle.replace(['\\', '"'], " "));
            clauses.push((Occur::Should, parser.parse_query_lenient(&phrase).0));
        }
        Ok(Box::new(BooleanQuery::new(clauses)))
    }

    /// Cómo decidir si una línea coincide con la consulta.
    fn line_matcher(&self, query: &FullTextQuery) -> Result<LineMatcher, String> {
        if !query.exact.is_empty() {
            return Ok(LineMatcher::Literal(
                query.exact.iter().map(|n| n.to_lowercase()).collect(),
            ));
        }
        let (tantivy_query, prefix) = self.build_query_with_prefix(query)?;
        let text_fields = [self.fields.title, self.fields.headings, self.fields.body];
        let mut terms = HashSet::new();
        tantivy_query.query_terms(&mut |term, _| {
            if text_fields.contains(&term.field())
                && let Some(text) = term.value().as_str()
            {
                terms.insert(text.to_string());
            }
        });
        // Prefijos: el de la última palabra y los `palabra*` escritos por el usuario
        let mut prefixes: Vec<String> = prefix.into_iter().collect();
        for word in normalize_operators(&query.text).split_whitespace() {
            let word = word
                .trim_start_matches(['+', '-', '('])
                .trim_end_matches(')');
            let word = word.rsplit_once(':').map_or(word, |(_, w)| w);
            if let Some(base) = word.strip_suffix('*').filter(|b| !b.is_empty())
                && let Some((token, _)) = self.analyze(base).into_iter().next()
            {
                prefixes.push(token);
            }
        }
        Ok(LineMatcher::Terms { terms, prefixes })
    }

    /// Primera coincidencia de la línea (rango en bytes), si la hay.
    fn find_in_line(&self, matcher: &LineMatcher, line: &str) -> Option<std::ops::Range<usize>> {
        match matcher {
            LineMatcher::Terms { terms, prefixes } => self
                .analyze(line)
                .into_iter()
                .find(|(token, _)| {
                    terms.contains(token) || prefixes.iter().any(|p| token.starts_with(p.as_str()))
                })
                .map(|(_, range)| range),
            LineMatcher::Literal(needles) => needles
                .iter()
                .filter_map(|n| find_case_insensitive(line, n))
                .min_by_key(|r| r.start),
        }
    }

    /// Líneas de `text` que coinciden, con la ubicación que dé `locate` (índice de línea → ubicación).
    fn collect_lines(
        &self,
        matcher: &LineMatcher,
        text: &str,
        options: LineOptions,
        locate: impl Fn(&[&str], usize) -> Option<MatchLocation>,
    ) -> (Vec<MatchedLine>, usize) {
        let lines: Vec<&str> = text.lines().collect();
        let mut out = Vec::new();
        let mut total = 0;
        for (i, line) in lines.iter().enumerate() {
            let Some(range) = self.find_in_line(matcher, line) else {
                continue;
            };
            total += 1;
            if out.len() >= options.max_per_doc {
                continue;
            }
            let from = i.saturating_sub(options.context);
            let to = (i + 1 + options.context).min(lines.len());
            out.push(MatchedLine {
                location: locate(&lines, i),
                text: clip_line(line, range.start, range.end),
                before: lines[from..i].iter().map(|l| clip_line(l, 0, 0)).collect(),
                after: lines[i + 1..to]
                    .iter()
                    .map(|l| clip_line(l, 0, 0))
                    .collect(),
            });
        }
        (out, total)
    }

    /// Búsqueda del modo exacto: candidatos por frase y comprobación literal en el texto guardado.
    fn search_exact(&self, query: &FullTextQuery) -> Result<(Vec<FullTextHit>, usize), String> {
        /// Candidatos que se comprueban como máximo; por encima, el total es un mínimo.
        const MAX_CANDIDATES: usize = 2000;
        let searcher = self.reader.searcher();
        let candidates = self.exact_candidates_query(&query.exact)?;
        let top = searcher
            .search(
                &*candidates,
                &TopDocs::with_limit(MAX_CANDIDATES).order_by_score(),
            )
            .map_err(|e| format!("Error en búsqueda tantivy: {}", e))?;
        let needles: Vec<String> = query.exact.iter().map(|n| n.to_lowercase()).collect();
        let mut hits = Vec::new();
        let mut total = 0;
        for (score, address) in top {
            let doc: TantivyDocument = searcher
                .doc(address)
                .map_err(|e| format!("Error leyendo doc {:?}: {}", address, e))?;
            let get = |f: Field| {
                doc.get_first(f)
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string()
            };
            let (body, title) = (get(self.fields.body), get(self.fields.title));
            let found_body = needles
                .iter()
                .filter_map(|n| find_case_insensitive(&body, n))
                .min_by_key(|r| r.start);
            let found_title = needles
                .iter()
                .filter_map(|n| find_case_insensitive(&title, n))
                .min_by_key(|r| r.start);
            if found_body.is_none() && found_title.is_none() {
                continue;
            }
            total += 1;
            if hits.len() >= query.limit {
                continue;
            }
            let kind =
                DocumentKind::parse(&get(self.fields.kind)).unwrap_or(DocumentKind::Markdown);
            let (snippet, matched) = match found_body.clone().or(found_title.clone()) {
                Some(r) => {
                    let source = if found_body.is_some() { &body } else { &title };
                    (exact_snippet(source, r.clone()), source[r].to_string())
                }
                None => (Vec::new(), String::new()),
            };
            let title_segments = match found_title {
                Some(r) => snippet_to_segments(&title, &[r]),
                None => vec![HighlightSegment {
                    text: title.clone(),
                    highlighted: false,
                }],
            };
            hits.push(FullTextHit {
                note_path: get(self.fields.path_id),
                title: title_segments,
                heading: Vec::new(),
                snippet,
                matched_terms: vec![matched],
                kind,
                score,
                lines: Vec::new(),
                total_line_matches: 0,
            });
        }
        Ok((hits, total))
    }
}

impl FullTextIndexImpl {
    /// Tokens de `text` tal como se indexan, con su posición en bytes.
    fn analyze(&self, text: &str) -> Vec<(String, std::ops::Range<usize>)> {
        let Ok(mut analyzer) = self.index.tokenizer_for_field(self.fields.headings) else {
            return Vec::new();
        };
        let mut stream = analyzer.token_stream(text);
        let mut tokens = Vec::new();
        while let Some(token) = stream.next() {
            tokens.push((token.text.clone(), token.offset_from..token.offset_to));
        }
        tokens
    }

    /// El encabezado con más términos buscados y, aparte, el texto que le sigue.
    /// `None` si ningún encabezado coincide o no se encuentra en el cuerpo.
    fn heading_excerpt(
        &self,
        body: &str,
        headings: &str,
        heading_terms: &HashSet<String>,
        highlight_terms: &HashSet<String>,
    ) -> Option<(Vec<HighlightSegment>, Vec<HighlightSegment>)> {
        const MAX_CHARS: usize = 160;

        let mut best: Option<(usize, &str)> = None;
        for line in headings.lines() {
            let matches = self
                .analyze(line)
                .iter()
                .filter(|(token, _)| heading_terms.contains(token))
                .count();
            if matches > 0 && best.is_none_or(|(most, _)| matches > most) {
                best = Some((matches, line));
            }
        }
        let (_, heading) = best?;

        let mut start = None;
        let mut offset = 0;
        for line in body.split_inclusive('\n') {
            if line.trim_end_matches('\n') == heading {
                start = Some(offset);
                break;
            }
            offset += line.len();
        }
        let after = &body[start? + heading.len()..];
        let text = after.trim_start();
        let end = text
            .char_indices()
            .nth(MAX_CHARS)
            .map_or(text.len(), |(index, _)| index);
        let mut following = text[..end].replace('\n', " ");
        let following_ranges = self.highlight_ranges(&following, highlight_terms);
        if end < text.len() {
            following.push_str("...");
        }

        let heading_ranges = self.highlight_ranges(heading, highlight_terms);
        Some((
            snippet_to_segments(heading, &heading_ranges),
            snippet_to_segments(&following, &following_ranges),
        ))
    }

    fn highlight_ranges(&self, text: &str, terms: &HashSet<String>) -> Vec<std::ops::Range<usize>> {
        self.analyze(text)
            .into_iter()
            .filter(|(token, _)| terms.contains(token))
            .map(|(_, range)| range)
            .collect()
    }
}

impl FullTextIndex for FullTextIndexImpl {
    fn upsert(&self, docs: Vec<FullTextDocument>) -> Result<(), String> {
        let writer = self.writer()?;

        for doc in docs {
            let term = Term::from_field_text(self.fields.path_id, &doc.path);
            writer.delete_term(term);

            let tantivy_doc = doc!(
                self.fields.path_id => doc.path.as_str(),
                self.fields.path => doc.path.as_str(),
                self.fields.title => doc.title.as_str(),
                self.fields.headings => doc.headings.as_str(),
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
        let writer = self.writer()?;
        for path in paths {
            let term = Term::from_field_text(self.fields.path_id, path);
            writer.delete_term(term);
        }
        Ok(())
    }

    fn delete_all(&self) -> Result<(), String> {
        let writer = self.writer()?;
        writer
            .delete_all_documents()
            .map_err(|e| format!("Error eliminando documentos: {}", e))?;
        Ok(())
    }

    fn commit(&self) -> Result<(), String> {
        let mut writer = self.writer()?;
        writer
            .commit()
            .map_err(|e| format!("Error en commit de tantivy: {}", e))?;
        self.reader
            .reload()
            .map_err(|e| format!("Error recargando index reader: {}", e))?;
        Ok(())
    }

    fn search(&self, query: &FullTextQuery) -> Result<(Vec<FullTextHit>, usize), String> {
        if !query.exact.is_empty() {
            return self.search_exact(query);
        }
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

        let mut query_terms = HashSet::new();
        let mut heading_terms = HashSet::new();
        tantivy_query.query_terms(&mut |term, _| {
            if let Some(text) = term.value().as_str() {
                query_terms.insert(text.to_string());
                if term.field() == self.fields.headings {
                    heading_terms.insert(text.to_string());
                }
            }
        });
        let filters_by_heading = expand_field_aliases(&query.text).contains("headings:");

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
            let headings_str = doc
                .get_first(self.fields.headings)
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let heading_excerpt = if filters_by_heading {
                self.heading_excerpt(body_str, headings_str, &heading_terms, &query_terms)
            } else {
                None
            };

            let body_snippet = body_snippet_gen.snippet(body_str);
            let (heading_segments, following_segments) = match heading_excerpt {
                Some((heading, following)) => (heading, Some(following)),
                None => (Vec::new(), None),
            };
            let snippet_segments = if let Some(segments) = following_segments {
                segments
            } else if body_snippet.fragment().is_empty() {
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
            for seg in title_segments
                .iter()
                .chain(heading_segments.iter())
                .chain(snippet_segments.iter())
            {
                if seg.highlighted {
                    let term = seg.text.trim();
                    if !term.is_empty()
                        && !matched_terms
                            .iter()
                            .any(|t: &String| t.eq_ignore_ascii_case(term))
                    {
                        matched_terms.push(term.to_string());
                    }
                }
            }

            hits.push(FullTextHit {
                note_path: path,
                title: title_segments,
                heading: heading_segments,
                snippet: snippet_segments,
                matched_terms,
                kind,
                score,
                lines: Vec::new(),
                total_line_matches: 0,
            });
        }

        Ok((hits, total_hits))
    }

    /// Ruta → mtime de cada documento vivo, sin cargar documentos: recorre los términos de
    /// `path_id` (un término por ruta) y lee `mtime` de su columna rápida. Cargar cada documento
    /// guardado (con su cuerpo) solo para estos dos campos costaba cientos de MB temporales al
    /// abrir una bóveda grande.
    fn indexed_versions(&self) -> Result<HashMap<String, u64>, String> {
        let searcher = self.reader.searcher();
        let mut versions = HashMap::new();

        for segment_reader in searcher.segment_readers() {
            let inverted = segment_reader
                .inverted_index(self.fields.path_id)
                .map_err(|e| format!("Error leyendo términos de path_id: {}", e))?;
            let mtimes = segment_reader
                .fast_fields()
                .u64("mtime")
                .map_err(|e| format!("Error leyendo la columna mtime: {}", e))?;
            let alive = segment_reader.alive_bitset();

            let mut terms = inverted
                .terms()
                .stream()
                .map_err(|e| format!("Error recorriendo términos de path_id: {}", e))?;
            while terms.advance() {
                let Ok(path) = std::str::from_utf8(terms.key()) else {
                    continue;
                };
                let mut postings = inverted
                    .read_postings_from_terminfo(terms.value(), IndexRecordOption::Basic)
                    .map_err(|e| format!("Error leyendo postings de path_id: {}", e))?;
                let mut doc_id = postings.doc();
                while doc_id != TERMINATED {
                    if alive.is_none_or(|bitset| bitset.is_alive(doc_id)) {
                        versions.insert(path.to_string(), mtimes.first(doc_id).unwrap_or(0));
                    }
                    doc_id = postings.advance();
                }
            }
        }

        Ok(versions)
    }

    fn num_docs(&self) -> u64 {
        self.reader.searcher().num_docs()
    }

    fn match_lines(&self, query: &FullTextQuery, text: &str) -> (Vec<MatchedLine>, usize) {
        let Some(options) = query.lines else {
            return (Vec::new(), 0);
        };
        match self.line_matcher(query) {
            Ok(matcher) => self.collect_lines(&matcher, text, options, |_, i| {
                Some(MatchLocation::Line { line: i + 1 })
            }),
            Err(_) => (Vec::new(), 0),
        }
    }

    fn match_document_lines(&self, query: &FullTextQuery, path: &str) -> (Vec<MatchedLine>, usize) {
        let Some(options) = query.lines else {
            return (Vec::new(), 0);
        };
        let Ok(matcher) = self.line_matcher(query) else {
            return (Vec::new(), 0);
        };
        let searcher = self.reader.searcher();
        let term = Term::from_field_text(self.fields.path_id, path);
        let by_path = tantivy::query::TermQuery::new(term, IndexRecordOption::Basic);
        let Ok(top) = searcher.search(&by_path, &TopDocs::with_limit(1).order_by_score()) else {
            return (Vec::new(), 0);
        };
        let Some(doc) = top
            .first()
            .and_then(|(_, address)| searcher.doc::<TantivyDocument>(*address).ok())
        else {
            return (Vec::new(), 0);
        };
        let get = |f: Field| {
            doc.get_first(f)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string()
        };
        let kind = DocumentKind::parse(&get(self.fields.kind)).unwrap_or(DocumentKind::Markdown);
        self.collect_lines(&matcher, &get(self.fields.body), options, |l, i| {
            locate_in_body(kind, l, i)
        })
    }
}

/// Cómo se decide si una línea coincide: por términos analizados (y prefijos) o por textos literales.
enum LineMatcher {
    Terms {
        terms: HashSet<String>,
        prefixes: Vec<String>,
    },
    /// Textos en minúsculas.
    Literal(Vec<String>),
}

/// Ubicación de la línea `i` del texto indexado de un documento, según su formato. Depende de
/// cómo escriben el texto los extractores (`infrastructure/search`).
fn locate_in_body(kind: DocumentKind, lines: &[&str], i: usize) -> Option<MatchLocation> {
    match kind {
        // `Nombre (Tipo)` es la línea del elemento; propiedades y documentación van debajo
        DocumentKind::Archimate | DocumentKind::Bpmn => lines[..=i].iter().rev().find_map(|l| {
            let l = l.trim_end();
            let inner = l.strip_suffix(')')?;
            let (name, ty) = inner.rsplit_once(" (")?;
            (!name.trim().is_empty()
                && !ty.is_empty()
                && ty.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
            .then(|| MatchLocation::Element {
                element: name.trim().to_string(),
                element_type: ty.to_string(),
            })
        }),
        DocumentKind::Excel => lines[..=i].iter().rev().find_map(|l| {
            l.strip_prefix("Hoja: ").map(|s| MatchLocation::Sheet {
                sheet: s.trim().to_string(),
            })
        }),
        // Markdown, Mermaid y código: el caso de uso pone el número de línea del archivo
        DocumentKind::Markdown | DocumentKind::Mermaid | DocumentKind::Code => {
            Some(MatchLocation::Line { line: i + 1 })
        }
        // PDF y Word se indexan en una sola línea; en draw.io y Excalidraw no se distingue la página
        _ => None,
    }
}

/// Las líneas largas (XML o PDF en una línea) se recortan a unos 300 caracteres alrededor de la coincidencia.
fn clip_line(line: &str, start: usize, end: usize) -> String {
    const MAX: usize = 300;
    let line = line.trim_end();
    if line.chars().count() <= MAX {
        return line.to_string();
    }
    let half = MAX / 2;
    let mut from = start.min(line.len()).saturating_sub(half);
    while !line.is_char_boundary(from) {
        from -= 1;
    }
    let mut to = (end.max(start) + half).min(line.len()).max(from);
    while !line.is_char_boundary(to) {
        to += 1;
    }
    let mut out = String::new();
    if from > 0 {
        out.push('…');
    }
    out.push_str(&line[from..to]);
    if to < line.len() {
        out.push('…');
    }
    out
}

/// Rango en bytes de la primera aparición de `needle` (ya en minúsculas) sin distinguir mayúsculas.
fn find_case_insensitive(haystack: &str, needle: &str) -> Option<std::ops::Range<usize>> {
    if needle.is_empty() {
        return None;
    }
    let lower = haystack.to_lowercase();
    if lower.len() == haystack.len() {
        // Mismas longitudes: los desplazamientos valen para el original
        return lower.find(needle).map(|i| i..i + needle.len());
    }
    // Algún carácter cambia de tamaño al pasar a minúsculas: se compara carácter a carácter
    haystack.char_indices().find_map(|(i, _)| {
        let rest = &haystack[i..];
        let mut taken = 0;
        let mut lowered = String::new();
        for c in rest.chars() {
            if lowered.len() >= needle.len() {
                break;
            }
            lowered.extend(c.to_lowercase());
            taken += c.len_utf8();
        }
        (lowered == needle).then(|| i..i + taken)
    })
}

/// Fragmento de ~160 caracteres alrededor de `range`, con ese texto resaltado.
fn exact_snippet(text: &str, range: std::ops::Range<usize>) -> Vec<HighlightSegment> {
    const SIDE: usize = 80;
    let mut from = range.start.saturating_sub(SIDE);
    while !text.is_char_boundary(from) {
        from -= 1;
    }
    let mut to = (range.end + SIDE).min(text.len());
    while !text.is_char_boundary(to) {
        to += 1;
    }
    let fragment = text[from..to].replace('\n', " ");
    let local = range.start - from..range.end - from;
    let mut segments = snippet_to_segments(&fragment, std::slice::from_ref(&local));
    if from > 0
        && let Some(first) = segments.first_mut()
    {
        first.text.insert_str(0, "...");
    }
    if to < text.len()
        && let Some(last) = segments.last_mut()
    {
        last.text.push_str("...");
    }
    segments
}

/// `a|b` → `a OR b` y `or`/`and` sueltos → `OR`/`AND`, fuera de comillas y de `/regex/`.
/// `\|` se queda como un espacio (Tantivy no indexa la barra).
fn normalize_operators(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                out.push(c);
            }
            '\\' if !in_quotes && chars.peek() == Some(&'|') => {
                chars.next();
                out.push(' ');
            }
            '|' if !in_quotes => out.push_str(" OR "),
            _ => out.push(c),
        }
    }
    // Palabras `or`/`and` sueltas fuera de comillas
    let mut result = String::with_capacity(out.len());
    let mut in_quotes = false;
    for (i, part) in out.split(' ').enumerate() {
        if i > 0 {
            result.push(' ');
        }
        let quotes = part.matches('"').count();
        if !in_quotes && quotes == 0 && (part == "or" || part == "and") {
            result.push_str(&part.to_uppercase());
        } else {
            result.push_str(part);
        }
        if quotes % 2 == 1 {
            in_quotes = !in_quotes;
        }
    }
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Si la consulta lleva operadores que el prefijo de la última palabra rompería.
fn has_operators(text: &str) -> bool {
    text.contains(['(', ')']) || text.split_whitespace().any(|w| w == "OR" || w == "AND")
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

        if start > current_idx && current_idx < fragment.len()
            && let Some(slice) = fragment.get(current_idx..start)
                && !slice.is_empty() {
                    segments.push(HighlightSegment {
                        text: slice.to_string(),
                        highlighted: false,
                    });
                }

        if start < end
            && let Some(slice) = fragment.get(start..end)
                && !slice.is_empty() {
                    segments.push(HighlightSegment {
                        text: slice.to_string(),
                        highlighted: true,
                    });
                }

        current_idx = end.max(current_idx);
    }

    if current_idx < fragment.len()
        && let Some(slice) = fragment.get(current_idx..)
            && !slice.is_empty() {
                segments.push(HighlightSegment {
                    text: slice.to_string(),
                    highlighted: false,
                });
            }

    segments
}

/// Sustituye `hs:` por `headings:` al inicio de un término (admite `-`, `+` o `(` delante).
/// No toca el texto entre comillas, donde `hs:` es texto literal.
fn expand_field_aliases(text: &str) -> String {
    const ALIAS: &str = "hs:";
    let mut out = String::with_capacity(text.len());
    let mut in_quotes = false;
    let mut at_term_start = true;
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if at_term_start && !in_quotes && rest.starts_with(ALIAS) {
            out.push_str("headings:");
            rest = &rest[ALIAS.len()..];
            at_term_start = false;
            continue;
        }
        if c == '"' {
            in_quotes = !in_quotes;
        }
        at_term_start = c.is_whitespace() || (at_term_start && matches!(c, '-' | '+' | '('));
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identificadores_se_encuentran_enteros_y_por_partes() {
        let index = FullTextIndexImpl::in_memory().unwrap();
        index
            .upsert(vec![FullTextDocument {
                path: "servicio.java".to_string(),
                title: "servicio.java".to_string(),
                body: "public class AchOperationCrService { void sendTransferRequest() {} }".to_string(),
                headings: String::new(),
                kind: DocumentKind::Code,
                mtime_nanos: 1,
            }])
            .unwrap();
        index.commit().unwrap();
        let found = |text: &str| {
            let q = FullTextQuery { text: text.to_string(), limit: 5, prefix_last_term: false, ..Default::default() };
            index.search(&q).unwrap().0.iter().any(|h| h.note_path == "servicio.java")
        };
        assert!(found("AchOperationCrService"), "el identificador entero");
        assert!(found("operation"), "una parte");
        assert!(found("transfer request"), "dos partes de otro identificador");
        assert!(!found("pagos"));
    }

    #[test]
    fn test_tantivy_index_upsert_and_search() {
        let index = FullTextIndexImpl::in_memory().unwrap();

        let doc1 = FullTextDocument {
            path: "nota1.md".to_string(),
            title: "Arquitectura Cebolla".to_string(),
            body: "Synapse implementa arquitectura cebolla en Rust con tantivy.".to_string(),
            headings: String::new(),
            kind: DocumentKind::Markdown,
            mtime_nanos: 1000,
        };

        let doc2 = FullTextDocument {
            path: "canciones.md".to_string(),
            title: "Lista de Canciones".to_string(),
            body: "Esta canción es hermosa y melódica.".to_string(),
            headings: String::new(),
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
            ..Default::default()
        };
        let (hits, count) = index.search(&q).unwrap();
        assert_eq!(count, 1);
        assert_eq!(hits[0].note_path, "nota1.md");

        // Stemming y acentos en español: "cancion" busca "canciones" y "canción"
        let q_stem = FullTextQuery {
            text: "cancion".to_string(),
            limit: 10,
            prefix_last_term: false,
            ..Default::default()
        };
        let (hits_stem, count_stem) = index.search(&q_stem).unwrap();
        assert_eq!(count_stem, 1);
        assert_eq!(hits_stem[0].note_path, "canciones.md");

        // Búsqueda por prefijo mientras se escribe
        let q_prefix = FullTextQuery {
            text: "arqui".to_string(),
            limit: 10,
            prefix_last_term: true,
            ..Default::default()
        };
        let (hits_pre, count_pre) = index.search(&q_prefix).unwrap();
        assert_eq!(count_pre, 1);
        assert_eq!(hits_pre[0].note_path, "nota1.md");

        // Re-upsert (idempotencia)
        let doc1_updated = FullTextDocument {
            path: "nota1.md".to_string(),
            title: "Arquitectura Cebolla Actualizada".to_string(),
            body: "Nueva información sobre arquitectura cebolla.".to_string(),
            headings: String::new(),
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

    #[test]
    fn test_heading_filter_snippet_starts_at_heading() {
        let dir = tempfile::tempdir().unwrap();
        let index = FullTextIndexImpl::open_or_create(dir.path()).unwrap();
        index
            .upsert(vec![FullTextDocument {
                path: "a.md".to_string(),
                title: "Nota".to_string(),
                body: "Introducción que menciona otra cosa.\nCierre\nHerramientas gate\naisix\n"
                    .to_string(),
                headings: "Cierre\nHerramientas gate\n".to_string(),
                kind: DocumentKind::Markdown,
                mtime_nanos: 1,
            }])
            .unwrap();
        index.commit().unwrap();

        let (hits, _) = index
            .search(&FullTextQuery {
                text: "hs:\"herramientas gate\"".to_string(),
                limit: 10,
                prefix_last_term: false,
                ..Default::default()
            })
            .unwrap();
        let text: String = hits[0].snippet.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(text, "aisix ");
        let marked: Vec<_> = hits[0]
            .heading
            .iter()
            .filter(|s| s.highlighted)
            .map(|s| s.text.as_str())
            .collect();
        assert_eq!(marked, ["Herramientas", "gate"]);
    }

    #[test]
    fn test_heading_snippet_with_real_markdown_and_prefix() {
        let md = "---\ntitle: Nota\n---\n\n# Nota\n\nollama:\n\n- `~/.ollama/models/`\n\n---\n\n# Prompts para Gemini\n\n**Opción 1:** texto\n\n<div>x</div>\n\n# Otro *título* con [enlace](http://a.b)\n\nfin\n";
        let (body, headings) =
            crate::infrastructure::search::content_extractor::extract_markdown_text_and_headings(
                md,
            );
        let dir = tempfile::tempdir().unwrap();
        let index = FullTextIndexImpl::open_or_create(dir.path()).unwrap();
        index
            .upsert(vec![FullTextDocument {
                path: "a.md".to_string(),
                title: "Nota".to_string(),
                body,
                headings,
                kind: DocumentKind::Markdown,
                mtime_nanos: 1,
            }])
            .unwrap();
        index.commit().unwrap();

        for (query, expected) in [
            ("hs:Prompts para", "Prompts para Gemini"),
            ("hs:Otro", "Otro título"),
        ] {
            let (hits, _) = index
                .search(&FullTextQuery {
                    text: query.to_string(),
                    limit: 10,
                    prefix_last_term: true,
                    ..Default::default()
                })
                .unwrap();
            let text: String = hits[0].heading.iter().map(|s| s.text.as_str()).collect();
            assert!(text.starts_with(expected), "{query}: {text:?}");
        }
    }

    #[test]
    fn test_expand_field_aliases() {
        assert_eq!(expand_field_aliases("hs:plan"), "headings:plan");
        assert_eq!(
            expand_field_aliases("uno -hs:dos (hs:tres"),
            "uno -headings:dos (headings:tres"
        );
        assert_eq!(
            expand_field_aliases("\"hs:plan\" ths:x"),
            "\"hs:plan\" ths:x"
        );
    }

    #[test]
    fn test_headings_boost_ranking_and_filter() {
        let index = FullTextIndexImpl::in_memory().unwrap();

        let sin_encabezado = FullTextDocument {
            path: "a.md".to_string(),
            title: "Nota A".to_string(),
            body: "Resumen del presupuesto anual de la oficina.".to_string(),
            headings: String::new(),
            kind: DocumentKind::Markdown,
            mtime_nanos: 1,
        };
        let con_encabezado = FullTextDocument {
            path: "b.md".to_string(),
            title: "Nota B".to_string(),
            body: "Resumen del presupuesto anual de la oficina.".to_string(),
            headings: "Presupuesto anual".to_string(),
            kind: DocumentKind::Markdown,
            mtime_nanos: 2,
        };
        index.upsert(vec![sin_encabezado, con_encabezado]).unwrap();
        index.commit().unwrap();

        // Ambas notas contienen el término en el cuerpo, pero la que lo tiene en un encabezado va primero
        let q = FullTextQuery {
            text: "presupuesto".to_string(),
            limit: 10,
            prefix_last_term: false,
            ..Default::default()
        };
        let (hits, count) = index.search(&q).unwrap();
        assert_eq!(count, 2);
        assert_eq!(hits[0].note_path, "b.md");

        // El prefijo mientras se escribe también mira los encabezados
        let q_prefix = FullTextQuery {
            text: "presup".to_string(),
            limit: 10,
            prefix_last_term: true,
            ..Default::default()
        };
        let (hits_pre, _) = index.search(&q_prefix).unwrap();
        assert_eq!(hits_pre[0].note_path, "b.md");

        // Filtro explícito por encabezado
        let q_filter = FullTextQuery {
            text: "headings:presupuesto".to_string(),
            limit: 10,
            prefix_last_term: false,
            ..Default::default()
        };
        let (hits_filter, count_filter) = index.search(&q_filter).unwrap();
        assert_eq!(count_filter, 1);
        assert_eq!(hits_filter[0].note_path, "b.md");

        // `hs:` es abreviatura de `headings:`
        let q_alias = FullTextQuery {
            text: "hs:presupuesto".to_string(),
            limit: 10,
            prefix_last_term: false,
            ..Default::default()
        };
        let (hits_alias, count_alias) = index.search(&q_alias).unwrap();
        assert_eq!(count_alias, 1);
        assert_eq!(hits_alias[0].note_path, "b.md");
    }

    fn indice(docs: Vec<(&str, DocumentKind, &str)>) -> FullTextIndexImpl {
        let index = FullTextIndexImpl::in_memory().unwrap();
        index
            .upsert(
                docs.into_iter()
                    .map(|(path, kind, body)| FullTextDocument {
                        path: path.into(),
                        title: path.into(),
                        body: body.into(),
                        headings: String::new(),
                        kind,
                        mtime_nanos: 1,
                    })
                    .collect(),
            )
            .unwrap();
        index.commit().unwrap();
        index
    }

    fn rutas(index: &FullTextIndexImpl, q: FullTextQuery) -> Vec<String> {
        let mut paths: Vec<String> = index.search(&q).unwrap().0.into_iter().map(|h| h.note_path).collect();
        paths.sort();
        paths
    }

    fn tres_documentos() -> FullTextIndexImpl {
        indice(vec![
            ("a.md", DocumentKind::Markdown, "programa TIEL emisor"),
            ("b.md", DocumentKind::Markdown, "interfaz QTI1 receptor"),
            ("c.md", DocumentKind::Markdown, "otro QTI3 BCTITPE8"),
        ])
    }

    #[test]
    fn alternativas_con_or_y_barra_tambien_con_prefijo() {
        let index = tres_documentos();
        for prefix_last_term in [false, true] {
            for text in ["TIEL OR QTI1 OR QTI3", "TIEL|QTI1|QTI3", "TIEL | QTI1 | QTI3", "tiel or qti1 or qti3"] {
                let q = FullTextQuery { text: text.into(), limit: 10, prefix_last_term, ..Default::default() };
                assert_eq!(rutas(&index, q), ["a.md", "b.md", "c.md"], "{text} (prefijo: {prefix_last_term})");
            }
            let q = FullTextQuery { text: "(TIEL|QTI1) emisor".into(), limit: 10, prefix_last_term, ..Default::default() };
            assert_eq!(rutas(&index, q), ["a.md"]);
        }
        // Sin operadores el prefijo de la última palabra sigue funcionando
        let q = FullTextQuery { text: "interfaz rece".into(), limit: 10, prefix_last_term: true, ..Default::default() };
        assert_eq!(rutas(&index, q), ["b.md"]);
        let q = FullTextQuery { text: "\"interfaz QTI1\" rece".into(), limit: 10, prefix_last_term: true, ..Default::default() };
        assert_eq!(rutas(&index, q), ["b.md"], "con una frase entre comillas, también");
    }

    #[test]
    fn normaliza_operadores_fuera_de_comillas() {
        assert_eq!(normalize_operators("a|b"), "a OR b");
        assert_eq!(normalize_operators("(a | b) c"), "(a OR b) c");
        assert_eq!(normalize_operators("a or b and c"), "a OR b AND c");
        assert_eq!(normalize_operators("\"a|b or c\" d"), "\"a|b or c\" d");
        assert_eq!(normalize_operators(r"a\|b"), "a b");
        assert!(has_operators("a OR b") && has_operators("(a) b"));
        assert!(!has_operators("kind:archimate maint") && !has_operators("-a b") && !has_operators("\"a b\" c"));
    }

    fn con_lineas(text: &str) -> FullTextQuery {
        FullTextQuery {
            text: text.into(),
            limit: 10,
            prefix_last_term: true,
            lines: Some(LineOptions { context: 1, max_per_doc: 20 }),
            ..Default::default()
        }
    }

    #[test]
    fn lineas_de_un_texto_con_su_numero_real() {
        let index = tres_documentos();
        let raw = "---\ntitle: Flujo\n---\n# Flujo\n\nEl MS llama a TIEL.\nOtra línea.\nQTI1 responde.\n";
        let (lines, total) = index.match_lines(&con_lineas("TIEL|QTI1"), raw);
        assert_eq!(total, 2);
        assert_eq!(lines[0].location, Some(MatchLocation::Line { line: 6 }));
        assert_eq!(lines[0].text, "El MS llama a TIEL.");
        assert_eq!(lines[0].before, [""]);
        assert_eq!(lines[0].after, ["Otra línea."]);
        assert_eq!(lines[1].location, Some(MatchLocation::Line { line: 8 }));
        // Prefijo escrito por el usuario y prefijo de la última palabra
        let (lines, _) = index.match_lines(&con_lineas("BCTITP*"), "x\nMS->>TIEL: BCTITPE8\n");
        assert_eq!(lines[0].location, Some(MatchLocation::Line { line: 2 }));
        let (lines, _) = index.match_lines(&con_lineas("respon"), raw);
        assert_eq!(lines.len(), 1, "«respon» como prefijo de «responde»");
    }

    #[test]
    fn lineas_de_un_documento_con_la_ubicacion_de_su_formato() {
        let index = indice(vec![
            (
                "m.archimate",
                DocumentKind::Archimate,
                "Aplicaciones\nACH Operation Maintenance Outbound V1 (ApplicationComponent)\nUNIQUECODE: CUD-024684\nDocumentación del componente",
            ),
            ("p.xlsx", DocumentKind::Excel, "Hoja: Resumen\na | b\nHoja: Mapping\nachOperationId | uuid | TIEL\n"),
            ("d.pdf", DocumentKind::Pdf, &format!("{} TIEL {}", "palabra ".repeat(100), "otra ".repeat(100))),
        ]);
        let (lines, _) = index.match_document_lines(&con_lineas("024684"), "m.archimate");
        assert_eq!(
            lines[0].location,
            Some(MatchLocation::Element {
                element: "ACH Operation Maintenance Outbound V1".into(),
                element_type: "ApplicationComponent".into()
            })
        );
        let (lines, _) = index.match_document_lines(&con_lineas("TIEL"), "p.xlsx");
        assert_eq!(lines[0].location, Some(MatchLocation::Sheet { sheet: "Mapping".into() }));
        let (lines, _) = index.match_document_lines(&con_lineas("TIEL"), "d.pdf");
        assert_eq!(lines[0].location, None);
        assert!(lines[0].text.contains("TIEL") && lines[0].text.starts_with('…') && lines[0].text.chars().count() < 320);
        assert_eq!(index.match_document_lines(&con_lineas("TIEL"), "no-existe.md"), (vec![], 0));
    }

    fn exacta(textos: &[&str]) -> FullTextQuery {
        FullTextQuery { limit: 10, exact: textos.iter().map(|t| t.to_string()).collect(), ..Default::default() }
    }

    #[test]
    fn modo_exacto_solo_con_el_texto_literal() {
        let index = indice(vec![
            ("api.md", DocumentKind::Markdown, "POST /ach-operations/{id}/Outbounds-ACH/initiate"),
            ("otro.md", DocumentKind::Markdown, "an outbound for ach operations"),
            ("apim.mmd", DocumentKind::Mermaid, "participant APIM\nAPIM: llama"),
        ]);
        let (hits, total) = index.search(&exacta(&["outbounds-ach"])).unwrap();
        assert_eq!(total, 1);
        assert_eq!(hits[0].note_path, "api.md");
        assert_eq!(hits[0].matched_terms, ["Outbounds-ACH"]);
        assert!(hits[0].snippet.iter().any(|s| s.highlighted && s.text == "Outbounds-ACH"));
        let (hits, _) = index.search(&exacta(&["outbounds-ach", "apim:"])).unwrap();
        assert_eq!(hits.len(), 2);
        assert!(index.search(&exacta(&[":"])).is_err(), "solo puntuación");
        // Líneas en modo exacto
        let q = FullTextQuery { lines: Some(LineOptions { context: 0, max_per_doc: 5 }), ..exacta(&["apim:"]) };
        let (lines, total) = index.match_document_lines(&q, "apim.mmd");
        assert_eq!(total, 1);
        assert_eq!(lines[0].location, Some(MatchLocation::Line { line: 2 }));
    }

}
