//! Texto de un diagrama BPMN (`.bpmn`) para el índice full-text.
//!
//! Lo que se busca en un proceso son sus pasos: el nombre de cada tarea, evento, compuerta, carril o
//! subproceso con su tipo, la documentación y las anotaciones. El dibujo (`bpmndi:*`: posiciones y
//! tamaños) y los ids se descartan.

use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;

/// Nombre local de la etiqueta, sin prefijo de espacio de nombres (`bpmn:userTask` → `userTask`).
fn local(tag: &str) -> String {
    tag.rsplit(':').next().unwrap_or(tag).to_string()
}

fn attr(e: &BytesStart, key: &str) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.as_ref() == key)
        .and_then(|a| quick_xml::escape::unescape(&a.value).ok().map(|v| v.into_owned()))
}

/// Líneas del proceso: `Validar beneficiario (userTask)`, documentación y anotaciones.
pub fn bpmn_text(xml: &str) -> String {
    let mut reader = Reader::from_str(xml);
    let mut out: Vec<String> = Vec::new();
    // Dentro de `documentation` o del `text` de una anotación: se junta su texto
    let mut capturing = false;
    let mut buf = String::new();
    // Profundidad dentro de `bpmndi:*` (el dibujo): se ignora entero
    let mut in_diagram = 0usize;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.name();
                if name.as_ref().starts_with("bpmndi:") || in_diagram > 0 {
                    in_diagram += 1;
                    continue;
                }
                let tag = local(name.as_ref());
                if tag == "documentation" || tag == "text" {
                    capturing = true;
                    buf.clear();
                    continue;
                }
                push_named(&e, &tag, &mut out);
            }
            Ok(Event::Empty(e)) => {
                if in_diagram > 0 || e.name().as_ref().starts_with("bpmndi:") {
                    continue;
                }
                let tag = local(e.name().as_ref());
                push_named(&e, &tag, &mut out);
            }
            Ok(Event::Text(t)) if capturing => {
                let s: &str = &t;
                buf.push_str(s);
            }
            Ok(Event::CData(t)) if capturing => {
                let s: &str = &t;
                buf.push_str(s);
            }
            Ok(Event::GeneralRef(r)) if capturing => {
                let name: &str = &r;
                buf.push(match name {
                    "amp" => '&',
                    "lt" => '<',
                    "gt" => '>',
                    "quot" => '"',
                    "apos" => '\'',
                    _ => r.resolve_char_ref().ok().flatten().unwrap_or(' '),
                });
            }
            Ok(Event::End(e)) => {
                if in_diagram > 0 {
                    in_diagram -= 1;
                    continue;
                }
                let tag = local(e.name().as_ref());
                if capturing && (tag == "documentation" || tag == "text") {
                    capturing = false;
                    let text = buf.split_whitespace().collect::<Vec<_>>().join(" ");
                    if !text.is_empty() {
                        out.push(text);
                    }
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    out.join("\n")
}

/// Un elemento con nombre: `Nombre (tipo)`. Las definiciones y los flujos sin nombre no aportan.
fn push_named(e: &BytesStart, tag: &str, out: &mut Vec<String>) {
    if let Some(name) = attr(e, "name") {
        let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
        if !name.is_empty() {
            out.push(format!("{name} ({tag})"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexa_pasos_documentacion_y_anotaciones_sin_el_dibujo() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL" xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI" id="Defs_1">
  <bpmn:process id="Process_1" name="Alta de transferencia">
    <bpmn:startEvent id="Start_1" name="Solicitud recibida"/>
    <bpmn:userTask id="Task_1" name="Validar   beneficiario">
      <bpmn:documentation>Se consulta el maestro &amp; la lista negra</bpmn:documentation>
    </bpmn:userTask>
    <bpmn:exclusiveGateway id="Gw_1" name="¿Monto alto?"/>
    <bpmn:sequenceFlow id="Flow_1" sourceRef="Start_1" targetRef="Task_1"/>
    <bpmn:textAnnotation id="Ann_1"><bpmn:text>Revisar con riesgos</bpmn:text></bpmn:textAnnotation>
  </bpmn:process>
  <bpmndi:BPMNDiagram id="Diagram_1" name="Vista">
    <bpmndi:BPMNPlane id="Plane_1" bpmnElement="Process_1">
      <bpmndi:BPMNShape id="Shape_1" bpmnElement="Task_1"><dc:Bounds x="10" y="20" width="100" height="80"/></bpmndi:BPMNShape>
    </bpmndi:BPMNPlane>
  </bpmndi:BPMNDiagram>
</bpmn:definitions>"#;
        let text = bpmn_text(xml);
        assert_eq!(
            text.lines().collect::<Vec<_>>(),
            vec![
                "Alta de transferencia (process)",
                "Solicitud recibida (startEvent)",
                "Validar beneficiario (userTask)",
                "Se consulta el maestro & la lista negra",
                "¿Monto alto? (exclusiveGateway)",
                "Revisar con riesgos",
            ]
        );
    }

    #[test]
    fn sin_prefijo_de_espacio_de_nombres_tambien_funciona() {
        let xml = r#"<definitions><process><task name="Aprobar"/></process></definitions>"#;
        assert_eq!(bpmn_text(xml), "Aprobar (task)");
    }
}
