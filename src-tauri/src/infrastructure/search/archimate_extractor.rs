//! Texto de un modelo ArchiMate (`.archimate`, de Archi) para el índice full-text.
//!
//! El XML de un modelo pesa megas, casi todo posiciones y estilos de los diagramas; indexado crudo
//! se cortaba al límite de caracteres y se perdían los nombres. Aquí se indexa solo lo que se busca:
//! el nombre de cada elemento con su tipo, su documentación, sus propiedades (`clave: valor`) y los
//! nombres de las carpetas y las vistas.

use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;

/// Líneas de texto del modelo: `Nombre (ApplicationComponent)`, documentación, `CLAVE: valor`.
pub fn archimate_text(xml: &str) -> String {
    let mut reader = Reader::from_str(xml);
    let mut out: Vec<String> = Vec::new();
    let mut in_documentation = false;
    let mut doc = String::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let tag = e.name();
                let tag = tag.as_ref();
                if tag == "documentation" || tag == "purpose" {
                    in_documentation = true;
                    doc.clear();
                    continue;
                }
                if tag == "property" {
                    if let (Some(k), Some(v)) = (attr(&e, "key"), attr(&e, "value"))
                        && !v.trim().is_empty()
                    {
                        out.push(format!("{k}: {v}"));
                    }
                    continue;
                }
                if let Some(name) = attr(&e, "name").filter(|n| !n.trim().is_empty()) {
                    match attr(&e, "xsi:type") {
                        Some(t) => {
                            out.push(format!("{name} ({})", t.trim_start_matches("archimate:")))
                        }
                        None => out.push(name),
                    }
                }
            }
            Ok(Event::Text(t)) if in_documentation => {
                let s: &str = &t;
                doc.push_str(s);
            }
            // Las entidades (`&amp;`, `&#10;`…) llegan como un evento aparte
            Ok(Event::GeneralRef(r)) if in_documentation => {
                let name: &str = &r;
                match name {
                    "amp" => doc.push('&'),
                    "lt" => doc.push('<'),
                    "gt" => doc.push('>'),
                    "quot" => doc.push('"'),
                    "apos" => doc.push('\''),
                    _ => match r.resolve_char_ref() {
                        Ok(Some(c)) => doc.push(c),
                        _ => doc.push(' '),
                    },
                }
            }
            Ok(Event::End(e)) if matches!(e.name().as_ref(), "documentation" | "purpose") => {
                in_documentation = false;
                let text = doc.trim();
                if !text.is_empty() {
                    out.push(text.to_string());
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    out.join("\n")
}

/// Entrada del índice de un modelo: carpeta (con su profundidad) o vista, y su línea.
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct ArchimateOutlineEntry {
    pub level: usize,
    pub kind: &'static str,
    pub text: String,
    pub line: usize,
}

/// Índice de un modelo: carpetas (`<folder name>`) y vistas (`ArchimateDiagramModel`), con línea.
pub fn archimate_outline(xml: &str) -> Vec<ArchimateOutlineEntry> {
    let mut out = Vec::new();
    let mut reader = Reader::from_str(xml);
    let mut depth = 0usize;
    let mut lines = LineCounter::default();
    loop {
        let pos = reader.buffer_position() as usize;
        match reader.read_event() {
            Ok(ev @ Event::Start(_)) | Ok(ev @ Event::Empty(_)) => {
                let is_start = matches!(ev, Event::Start(_));
                let (Event::Start(e) | Event::Empty(e)) = ev else {
                    continue;
                };
                let tag = e.name();
                if tag.as_ref() == "folder" {
                    out.push(ArchimateOutlineEntry {
                        level: depth,
                        kind: "folder",
                        text: attr(&e, "name").unwrap_or_default(),
                        line: lines.line_at(xml, pos),
                    });
                    if is_start {
                        depth += 1;
                    }
                } else if tag.as_ref() == "element"
                    && attr(&e, "xsi:type").as_deref() == Some("archimate:ArchimateDiagramModel")
                {
                    out.push(ArchimateOutlineEntry {
                        level: depth,
                        kind: "view",
                        text: attr(&e, "name").unwrap_or_default(),
                        line: lines.line_at(xml, pos),
                    });
                }
            }
            Ok(Event::End(e)) if e.name().as_ref() == "folder" => depth = depth.saturating_sub(1),
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    out
}

/// Elemento conectado por una relación.
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct ArchimateRelation {
    pub relation: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub relation_name: String,
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
}

/// Todo lo de un elemento de un modelo ArchiMate, resuelto en una sola respuesta.
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct ArchimateElementReport {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub line: usize,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub documentation: String,
    pub properties: Vec<(String, String)>,
    /// Relaciones donde el elemento es el origen.
    pub outgoing: Vec<ArchimateRelation>,
    /// Relaciones donde el elemento es el destino.
    pub incoming: Vec<ArchimateRelation>,
    /// Vistas donde aparece dibujado.
    pub views: Vec<String>,
}

#[derive(Default, Clone)]
struct Element {
    id: String,
    name: String,
    kind: String,
    line: usize,
    documentation: String,
    properties: Vec<(String, String)>,
    source: String,
    target: String,
}

/// Elementos que coinciden con `query` (su id exacto, o parte de su nombre sin distinguir
/// mayúsculas), hasta `limit`, con sus relaciones y vistas resueltas.
pub fn inspect_archimate(xml: &str, query: &str, limit: usize) -> Vec<ArchimateElementReport> {
    parse_archimate(xml).inspect(query, limit)
}

/// Modelo ya recorrido: elementos, dónde aparece cada uno e índice. Recorrer el XML es lo costoso;
/// consultarlo después es barato, así que es lo que se guarda en la caché del MCP.
pub struct ArchimateModel {
    elements: Vec<Element>,
    /// (vista, id del elemento dibujado en ella)
    appearances: Vec<(String, String)>,
    outline: Vec<ArchimateOutlineEntry>,
    lines: usize,
}

/// Recorre el modelo una vez.
pub fn parse_archimate(xml: &str) -> ArchimateModel {
    let mut elements: Vec<Element> = Vec::new();
    // vista → ids de los elementos dibujados en ella
    let mut appearances: Vec<(String, String)> = Vec::new();
    let mut reader = Reader::from_str(xml);
    let mut lines = LineCounter::default();
    // pila de elementos abiertos (para documentación/propiedades) y vista abierta
    let mut open: Vec<Option<usize>> = Vec::new();
    let mut view: Option<String> = None;
    let mut view_depth = 0usize;
    let mut in_doc = false;
    let mut doc = String::new();
    loop {
        let pos = reader.buffer_position() as usize;
        match reader.read_event() {
            Ok(ev @ Event::Start(_)) | Ok(ev @ Event::Empty(_)) => {
                let is_start = matches!(ev, Event::Start(_));
                let (Event::Start(e) | Event::Empty(e)) = ev else {
                    continue;
                };
                let tag = e.name();
                match tag.as_ref() {
                    "element" => {
                        let kind = attr(&e, "xsi:type")
                            .unwrap_or_default()
                            .trim_start_matches("archimate:")
                            .to_string();
                        let el = Element {
                            id: attr(&e, "id").unwrap_or_default(),
                            name: attr(&e, "name").unwrap_or_default(),
                            line: lines.line_at(xml, pos),
                            source: attr(&e, "source").unwrap_or_default(),
                            target: attr(&e, "target").unwrap_or_default(),
                            kind,
                            ..Default::default()
                        };
                        if el.kind == "ArchimateDiagramModel" && is_start {
                            view = Some(el.name.clone());
                            view_depth = open.len();
                        }
                        elements.push(el);
                        if is_start {
                            open.push(Some(elements.len() - 1));
                        }
                    }
                    "property" => {
                        if let (Some(Some(i)), Some(k)) = (open.last(), attr(&e, "key")) {
                            elements[*i]
                                .properties
                                .push((k, attr(&e, "value").unwrap_or_default()));
                        }
                    }
                    "documentation" => {
                        in_doc = true;
                        doc.clear();
                    }
                    "child" => {
                        if let (Some(v), Some(target)) = (&view, attr(&e, "archimateElement")) {
                            appearances.push((v.clone(), target));
                        }
                        if is_start {
                            open.push(None);
                        }
                    }
                    _ if is_start => open.push(None),
                    _ => {}
                }
            }
            Ok(Event::Text(t)) if in_doc => {
                let s: &str = &t;
                doc.push_str(s);
            }
            Ok(Event::GeneralRef(r)) if in_doc => {
                let name: &str = &r;
                doc.push(match name {
                    "amp" => '&',
                    "lt" => '<',
                    "gt" => '>',
                    "quot" => '"',
                    "apos" => '\'',
                    _ => r.resolve_char_ref().ok().flatten().unwrap_or(' '),
                });
            }
            Ok(Event::End(e)) => {
                if e.name().as_ref() == "documentation" {
                    in_doc = false;
                    if let Some(Some(i)) = open.last() {
                        elements[*i].documentation = doc.trim().to_string();
                    }
                    continue;
                }
                open.pop();
                if view.is_some() && open.len() <= view_depth {
                    view = None;
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }

    ArchimateModel {
        elements,
        appearances,
        outline: archimate_outline(xml),
        lines: xml.lines().count(),
    }
}

impl ArchimateModel {
    pub fn outline(&self) -> &[ArchimateOutlineEntry] {
        &self.outline
    }

    /// Líneas del archivo.
    pub fn lines(&self) -> usize {
        self.lines
    }

    /// Memoria aproximada que ocupa (para el límite de la caché).
    pub fn approx_bytes(&self) -> usize {
        let el: usize = self
            .elements
            .iter()
            .map(|e| {
                64 + e.id.len()
                    + e.name.len()
                    + e.kind.len()
                    + e.documentation.len()
                    + e.source.len()
                    + e.target.len()
                    + e.properties
                        .iter()
                        .map(|(k, v)| 48 + k.len() + v.len())
                        .sum::<usize>()
            })
            .sum();
        let ap: usize = self
            .appearances
            .iter()
            .map(|(v, id)| 48 + v.len() + id.len())
            .sum();
        let ol: usize = self.outline.iter().map(|o| 48 + o.text.len()).sum();
        el + ap + ol
    }

    /// Elementos que coinciden con `query` (id exacto o parte del nombre), hasta `limit`.
    pub fn inspect(&self, query: &str, limit: usize) -> Vec<ArchimateElementReport> {
        let elements = &self.elements;
        let appearances = &self.appearances;
        let q = query.trim().to_lowercase();
        let by_id: std::collections::HashMap<&str, &Element> =
            elements.iter().map(|e| (e.id.as_str(), e)).collect();
        let related = |rel: &Element, other_id: &str| -> ArchimateRelation {
            let other = by_id.get(other_id);
            ArchimateRelation {
                relation: rel.kind.clone(),
                relation_name: rel.name.clone(),
                id: other_id.to_string(),
                name: other.map(|o| o.name.clone()).unwrap_or_default(),
                kind: other.map(|o| o.kind.clone()).unwrap_or_default(),
            }
        };
        elements
            .iter()
            .filter(|e| e.source.is_empty() && e.kind != "ArchimateDiagramModel")
            .filter(|e| {
                e.id.to_lowercase() == q || (!q.is_empty() && e.name.to_lowercase().contains(&q))
            })
            .take(limit)
            .map(|e| {
                let mut views: Vec<String> = appearances
                    .iter()
                    .filter(|(_, id)| *id == e.id)
                    .map(|(v, _)| v.clone())
                    .collect();
                views.dedup();
                ArchimateElementReport {
                    id: e.id.clone(),
                    name: e.name.clone(),
                    kind: e.kind.clone(),
                    line: e.line,
                    documentation: e.documentation.clone(),
                    properties: e.properties.clone(),
                    outgoing: elements
                        .iter()
                        .filter(|r| r.source == e.id)
                        .map(|r| related(r, &r.target))
                        .collect(),
                    incoming: elements
                        .iter()
                        .filter(|r| r.target == e.id)
                        .map(|r| related(r, &r.source))
                        .collect(),
                    views,
                }
            })
            .collect()
    }
}

/// Número de línea de una posición, contando de forma incremental (las posiciones crecen).
#[derive(Default)]
struct LineCounter {
    pos: usize,
    line: usize,
}

impl LineCounter {
    fn line_at(&mut self, text: &str, pos: usize) -> usize {
        let pos = pos.min(text.len());
        if pos < self.pos {
            self.pos = 0;
            self.line = 0;
        }
        // La etiqueta empieza después de los espacios/saltos que el lector consumió antes
        let start = text[pos..].find('<').map_or(pos, |o| pos + o);
        self.line += text.as_bytes()[self.pos..start]
            .iter()
            .filter(|b| **b == b'\n')
            .count();
        self.pos = start;
        self.line + 1
    }
}

fn attr(e: &BytesStart, key: &str) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.as_ref() == key)
        .and_then(|a| {
            quick_xml::escape::unescape(&a.value)
                .ok()
                .map(|v| v.into_owned())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODEL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<archimate:model xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:archimate="http://www.archimatetool.com/archimate" name="ACH Operations" id="id-1">
  <folder name="Application" id="id-2" type="application">
    <element xsi:type="archimate:ApplicationComponent" name="ACH Operation Maintenance Outbound V1" id="id-3">
      <documentation>Mantiene las operaciones &amp; reintentos</documentation>
      <property key="UNIQUECODE" value="CUD-0042"/>
    </element>
  </folder>
  <folder name="Views" id="id-4" type="diagrams">
    <element xsi:type="archimate:ArchimateDiagramModel" name="Vista general" id="id-5">
      <child xsi:type="archimate:DiagramObject" id="id-6" archimateElement="id-3">
        <bounds x="10" y="20" width="120" height="55"/>
      </child>
    </element>
  </folder>
</archimate:model>"#;

    #[test]
    fn indice_de_carpetas_y_vistas() {
        let outline = archimate_outline(MODEL);
        let texts: Vec<_> = outline
            .iter()
            .map(|e| (e.level, e.kind, e.text.as_str(), e.line))
            .collect();
        assert_eq!(
            texts,
            vec![
                (0, "folder", "Application", 3),
                (0, "folder", "Views", 9),
                (1, "view", "Vista general", 10)
            ]
        );
    }

    #[test]
    fn inspecciona_un_elemento_con_relaciones_y_vistas() {
        let model = MODEL.replace(
            "</archimate:model>",
            r#"<folder name="Relations" id="id-7" type="relations">
    <element xsi:type="archimate:ServingRelationship" id="id-8" source="id-3" target="id-9"/>
    <element xsi:type="archimate:AssignmentRelationship" id="id-10" source="id-11" target="id-3"/>
  </folder>
  <element xsi:type="archimate:ApplicationService" name="Servicio de pagos" id="id-9"/>
  <element xsi:type="archimate:Node" name="Servidor" id="id-11"/>
</archimate:model>"#,
        );
        let found = inspect_archimate(&model, "maintenance outbound", 5);
        assert_eq!(found.len(), 1);
        let e = &found[0];
        assert_eq!(
            (e.id.as_str(), e.kind.as_str(), e.line),
            ("id-3", "ApplicationComponent", 4)
        );
        assert_eq!(e.documentation, "Mantiene las operaciones & reintentos");
        assert_eq!(
            e.properties,
            vec![("UNIQUECODE".to_string(), "CUD-0042".to_string())]
        );
        assert_eq!(e.outgoing[0].name, "Servicio de pagos");
        assert_eq!(e.outgoing[0].relation, "ServingRelationship");
        assert_eq!(e.incoming[0].name, "Servidor");
        assert_eq!(e.views, vec!["Vista general".to_string()]);
        // Por id exacto también
        assert_eq!(inspect_archimate(&model, "id-3", 5).len(), 1);
    }

    #[test]
    fn indexa_nombres_documentacion_y_propiedades() {
        let text = archimate_text(MODEL);
        assert!(text.contains("ACH Operation Maintenance Outbound V1 (ApplicationComponent)"));
        assert!(text.contains("Mantiene las operaciones & reintentos"));
        assert!(text.contains("UNIQUECODE: CUD-0042"));
        assert!(text.contains("Vista general (ArchimateDiagramModel)"));
        assert!(text.contains("Application"));
        // Las posiciones de los diagramas no entran
        assert!(!text.contains("120"));
    }
}
