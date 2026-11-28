//! Árbol XML mínimo, con límites: el documento puede venir de cualquier sitio.
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;

const MAX_DEPTH: usize = 200;
const MAX_NODES: usize = 1_500_000;

#[derive(Debug, Default)]
pub struct Node {
    /// Nombre con prefijo (`w:p`).
    name: String,
    attrs: Vec<(String, String)>,
    /// Solo los elementos de texto (`w:t`, `w:instrText`…) lo usan.
    pub text: String,
    pub children: Vec<Node>,
}

fn local(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

impl Node {
    /// Nombre sin prefijo: el de los espacios de nombres no es fijo.
    pub fn local(&self) -> &str {
        local(&self.name)
    }

    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| local(k) == name)
            .map(|(_, v)| v.as_str())
    }

    /// Como `attr`, pero solo con prefijo (`r:id`, no `id`).
    pub fn prefixed_attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k.contains(':') && local(k) == name)
            .map(|(_, v)| v.as_str())
    }

    pub fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.local() == name)
    }

    /// Valor de un interruptor de Word: `<w:b/>` es verdadero, `<w:b w:val="0"/>` no.
    pub fn flag(&self, name: &str) -> bool {
        self.child(name)
            .is_some_and(|c| !matches!(c.attr("val"), Some("0" | "false" | "off" | "none")))
    }
}

fn node_of(e: &BytesStart) -> Node {
    let name: &str = &e.name().as_ref().to_string();
    let attrs = e
        .attributes()
        .flatten()
        .map(|a| {
            let key: &str = a.key.as_ref();
            let value = quick_xml::escape::unescape(&a.value)
                .map(|v| v.into_owned())
                .unwrap_or_default();
            (key.to_string(), value)
        })
        .collect();
    Node {
        name: name.to_string(),
        attrs,
        ..Node::default()
    }
}

pub fn parse(xml: &str) -> Result<Node, String> {
    // Sin `trim_text`: los espacios al borde de un run cuentan
    let mut reader = Reader::from_str(xml);
    let mut stack: Vec<Node> = vec![Node::default()];
    let mut nodes = 0usize;
    let mut count = || {
        nodes += 1;
        if nodes > MAX_NODES {
            return Err("El documento es demasiado grande para compararlo".to_string());
        }
        Ok(())
    };
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                count()?;
                if stack.len() > MAX_DEPTH {
                    return Err("Document has too many nested levels".into());
                }
                stack.push(node_of(&e));
            }
            Ok(Event::Empty(e)) => {
                count()?;
                let node = node_of(&e);
                if let Some(top) = stack.last_mut() {
                    top.children.push(node);
                }
            }
            Ok(Event::End(_)) => {
                if stack.len() < 2 {
                    return Err("Document XML is corrupted".into());
                }
                let node = stack.pop().unwrap_or_default();
                if let Some(top) = stack.last_mut() {
                    top.children.push(node);
                }
            }
            Ok(Event::Text(t)) => {
                let s: &str = &t;
                push_text(&mut stack, s);
            }
            Ok(Event::CData(t)) => {
                let s: &str = &t;
                push_text(&mut stack, s);
            }
            // Las entidades (`&amp;`, `&#241;`…) son eventos aparte
            Ok(Event::GeneralRef(r)) => {
                let name: &str = &r;
                let c = match name {
                    "amp" => '&',
                    "lt" => '<',
                    "gt" => '>',
                    "quot" => '"',
                    "apos" => '\'',
                    _ => r.resolve_char_ref().ok().flatten().unwrap_or(' '),
                };
                push_text(&mut stack, c.encode_utf8(&mut [0; 4]));
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("Document XML is corrupted: {e}")),
            _ => {}
        }
    }
    if stack.len() != 1 {
        return Err("Document XML is truncated".into());
    }
    let mut root = stack.pop().unwrap_or_default();
    root.children
        .pop()
        .ok_or_else(|| "Document has no content".to_string())
}

fn push_text(stack: &mut [Node], s: &str) {
    if let Some(top) = stack.last_mut()
        && matches!(top.local(), "t" | "instrText")
    {
        top.text.push_str(s);
    }
}
