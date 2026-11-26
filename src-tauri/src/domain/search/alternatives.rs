/// Parte una consulta en alternativas separadas por `|`, como `rg "a|b"`: `a|b` y `a | b` dan
/// `["a", "b"]` (sin los espacios de alrededor). Si al partir queda alguna parte vacía
/// (`| campo |`, `a||b`, `|` al principio o al final), la consulta entera es un único texto
/// literal: así se pueden buscar filas de tablas Markdown. `\|` es siempre un `|` literal.
///
/// Sin `|` devuelve la consulta tal cual (sin recortar), para no cambiar las búsquedas de siempre.
pub fn split_alternatives(query: &str) -> Vec<String> {
    let mut parts = vec![String::new()];
    let mut chars = query.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => {
                chars.next();
                parts.last_mut().unwrap().push('|');
            }
            '|' => parts.push(String::new()),
            _ => parts.last_mut().unwrap().push(c),
        }
    }
    if parts.len() == 1 {
        return parts;
    }
    if parts.iter().any(|p| p.trim().is_empty()) {
        return vec![query.replace("\\|", "|")];
    }
    let mut out: Vec<String> = Vec::with_capacity(parts.len());
    for p in parts {
        let p = p.trim().to_string();
        if !out.iter().any(|o| o.eq_ignore_ascii_case(&p)) {
            out.push(p);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parte_por_barras_y_recorta() {
        assert_eq!(split_alternatives("TIEL|QTI1|QTI3"), ["TIEL", "QTI1", "QTI3"]);
        assert_eq!(
            split_alternatives("id_ach_operation | Límite BD | no almacena"),
            ["id_ach_operation", "Límite BD", "no almacena"]
        );
    }

    #[test]
    fn sin_barras_queda_igual_sin_recortar() {
        assert_eq!(split_alternatives(" participant "), [" participant "]);
        assert_eq!(split_alternatives(""), [""]);
    }

    #[test]
    fn partes_vacias_hacen_la_consulta_literal() {
        assert_eq!(split_alternatives("| campo |"), ["| campo |"]);
        assert_eq!(split_alternatives("a||b"), ["a||b"]);
        assert_eq!(split_alternatives("|a"), ["|a"]);
        assert_eq!(split_alternatives("a|"), ["a|"]);
        assert_eq!(split_alternatives("a| |b"), ["a| |b"]);
    }

    #[test]
    fn barra_escapada_es_literal() {
        assert_eq!(split_alternatives(r"a\|b"), ["a|b"]);
        assert_eq!(split_alternatives(r"x \| y|z"), ["x | y", "z"]);
        assert_eq!(split_alternatives(r"\| campo \|"), ["| campo |"]);
    }

    #[test]
    fn quita_alternativas_repetidas() {
        assert_eq!(split_alternatives("TIEL|tiel|QTI1"), ["TIEL", "QTI1"]);
    }
}
