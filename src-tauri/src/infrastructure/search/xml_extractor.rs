//! Texto de un XML cualquiera para el índice full-text (los formatos conocidos, como ArchiMate o
//! draw.io, tienen su propio extractor).
//!
//! Antes se indexaba el XML crudo, cortado al límite de caracteres: etiquetas, ids, coordenadas y
//! colores ocupaban el espacio y lo que venía después del corte se perdía. Aquí se recorre en modo
//! streaming (sin cargar el árbol) y se queda solo con lo legible: el texto de los nodos y los
//! valores de atributos que parecen palabras, sin repetir.

use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::HashSet;

pub fn xml_text(xml: &str) -> String {
    let mut reader = Reader::from_str(xml);
    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut push = |s: &str| {
        let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
        if is_readable(&s) && seen.insert(s.clone()) {
            out.push(s);
        }
    };
    // El texto de un nodo llega en trozos (las entidades `&amp;`… son eventos aparte): se junta
    let mut text = String::new();
    loop {
        let event = reader.read_event();
        if !matches!(event, Ok(Event::Text(_)) | Ok(Event::GeneralRef(_))) && !text.is_empty() {
            push(&text);
            text.clear();
        }
        match event {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                for a in e.attributes().flatten() {
                    // Atributos de espacio de nombres y de esquema: ruido
                    let key: &str = a.key.as_ref();
                    if key.starts_with("xmlns") || key.starts_with("xsi:") || key == "id" {
                        continue;
                    }
                    if let Ok(v) = quick_xml::escape::unescape(&a.value) {
                        push(&v);
                    }
                }
            }
            Ok(Event::Text(t)) => {
                let s: &str = &t;
                text.push_str(s);
            }
            Ok(Event::GeneralRef(r)) => {
                let name: &str = &r;
                text.push(match name {
                    "amp" => '&',
                    "lt" => '<',
                    "gt" => '>',
                    "quot" => '"',
                    "apos" => '\'',
                    _ => r.resolve_char_ref().ok().flatten().unwrap_or(' '),
                });
            }
            Ok(Event::CData(t)) => {
                let s: &str = &t;
                push(s);
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    out.join("\n")
}

/// ¿Vale la pena indexarlo? Descarta números, coordenadas, colores, UUID/ids, hashes y base64.
fn is_readable(s: &str) -> bool {
    if s.is_empty() || s.len() > 4_000 {
        return false;
    }
    // Sin letras (números, coordenadas «10,20», fechas…)
    if !s.chars().any(char::is_alphabetic) {
        return false;
    }
    // Una sola «palabra» larga sin espacios: id, hash, base64, ruta de estilo
    if !s.contains(' ') {
        let lower = s.to_lowercase();
        if lower.starts_with('#') && s.len() <= 9 {
            return false; // color
        }
        let hexish = s
            .chars()
            .filter(|c| c.is_ascii_hexdigit() || *c == '-')
            .count();
        if s.len() >= 16 && hexish * 10 >= s.len() * 9 {
            return false; // uuid / hash
        }
        if s.len() > 60 && !s.contains(['/', '.', '_', ':']) {
            return false; // base64 u otro blob
        }
        // estilos «a=b;c=d»
        if s.matches(';').count() >= 2 && s.contains('=') {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexa_texto_y_atributos_legibles() {
        let xml = r##"<?xml version="1.0"?>
<definitions xmlns="http://example.com/x" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <process id="a1b2c3d4-e5f6-7890-abcd-ef0123456789" name="Alta de transferencia">
    <task name="Validar beneficiario" color="#ff00aa" x="120" y="45"/>
    <task name="Validar beneficiario"/>
    <documentation>Se consulta el &amp; maestro</documentation>
    <style>fillColor=#fff;strokeColor=#000;rounded=1</style>
    <script><![CDATA[return monto > 0;]]></script>
  </process>
</definitions>"##;
        let text = xml_text(xml);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            lines,
            vec![
                "Alta de transferencia",
                "Validar beneficiario",
                "Se consulta el & maestro",
                "return monto > 0;"
            ]
        );
    }
}
