//! El texto de terceros (descripciones y comentarios de un gestor de issues) como Markdown saneado:
//! sin código, sin HTML y sin nada que se cargue solo de la red al mostrarlo.

use dom_query::Document;
use pulldown_cmark::{Event, Options, Parser, Tag, html};

/// Elementos que no aportan texto.
const SKIPPED: [&str; 7] = [
    "script", "style", "meta", "head", "iframe", "object", "embed",
];

/// Una dirección que se puede escribir en un enlace Markdown; `None` si no es web ni de correo.
fn safe_link(url: &str) -> Option<String> {
    let url = url.trim();
    let lower = url.to_ascii_lowercase();
    ["https://", "http://", "mailto:"]
        .iter()
        .any(|scheme| lower.starts_with(scheme))
        .then(|| {
            url.chars()
                .map(|c| match c {
                    ' ' => "%20".into(),
                    '(' => "%28".into(),
                    ')' => "%29".into(),
                    '<' => "%3C".into(),
                    '>' => "%3E".into(),
                    c if c.is_control() => String::new(),
                    c => c.to_string(),
                })
                .collect()
        })
}

fn attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn from_html(html: &str) -> String {
    let document = Document::from(html);
    // Una imagen se cargaría sola al mostrarla: queda como enlace
    for image in document.select("img").nodes() {
        let alt = image.attr("alt").unwrap_or_default();
        let label = if alt.trim().is_empty() {
            "imagen"
        } else {
            alt.trim()
        };
        let replacement = match image.attr("src").and_then(|src| safe_link(&src)) {
            Some(src) => format!("<a href=\"{}\">{}</a>", attribute(&src), attribute(label)),
            None => attribute(label),
        };
        image.replace_with_html(replacement);
    }
    for link in document.select("a").nodes() {
        match link.attr("href").and_then(|href| safe_link(&href)) {
            Some(href) => link.set_attr("href", &href),
            None => link.remove_attr("href"),
        }
    }
    // Tres comillas invertidas dentro de un bloque de código lo cerrarían antes de tiempo
    for code in document.select("pre, code").nodes() {
        let text = code.text();
        if text.contains("```") {
            code.set_text(text.replace("```", "``\u{200B}`").as_str());
        }
    }
    document.md(Some(&SKIPPED)).trim().to_string()
}

/// Destinos de enlace que ejecutan o leen algo.
const UNSAFE: [&str; 4] = ["javascript:", "data:", "vbscript:", "file:"];

/// Markdown que llega ya escrito: se deja sin HTML, imágenes ni enlaces que ejecuten algo.
pub fn from_markdown(text: &str) -> String {
    let mut clean = text
        .replace("\r\n", "\n")
        .replace('<', "&lt;")
        .replace("![", "[");
    // El destino de un enlace, en línea `](…)` o por referencia `]: …`
    for opener in ["](", "]:"] {
        let mut from = 0;
        while let Some(found) = clean[from..].find(opener) {
            let after = from + found + opener.len();
            let mut start = clean.len() - clean[after..].trim_start().len();
            if clean[start..].starts_with("&lt;") {
                start += "&lt;".len();
            }
            let rest = clean[start..].to_ascii_lowercase();
            match UNSAFE.iter().find(|scheme| rest.starts_with(*scheme)) {
                // Se vuelve a mirar el mismo sitio: puede venir repetido
                Some(scheme) => clean.replace_range(start..start + scheme.len(), ""),
                None => from = after,
            }
        }
    }
    clean.trim().to_string()
}

/// Lo que sobrevive al ir a Markdown y volver.
const KEPT: [&str; 29] = [
    "html",
    "head",
    "body",
    "div",
    "p",
    "br",
    "b",
    "strong",
    "i",
    "em",
    "s",
    "del",
    "strike",
    "ul",
    "ol",
    "li",
    "a",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "code",
    "pre",
    "blockquote",
    "hr",
    "span",
    "table",
];
const KEPT_CELLS: [&str; 5] = ["thead", "tbody", "tr", "th", "td"];

/// Si un HTML tiene formato que su Markdown no conserva (imágenes, estilos, menciones…).
pub fn loses_format(html: &str) -> bool {
    let document = Document::from(html);
    document.select("*").nodes().iter().any(|node| {
        let Some(name) = node.node_name() else {
            return false;
        };
        let name = name.to_ascii_lowercase();
        !(KEPT.contains(&name.as_str()) || KEPT_CELLS.contains(&name.as_str()))
            || node
                .attrs()
                .iter()
                .any(|attribute| !(name == "a" && &*attribute.name.local == "href"))
    })
}

/// El Markdown como HTML para el proveedor. El HTML que traiga escrito va como texto y los enlaces
/// que ejecuten algo se quedan sin destino.
pub fn to_html(markdown: &str) -> String {
    let events = Parser::new_ext(
        markdown,
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH,
    )
    .map(|event| match event {
        Event::Html(raw) | Event::InlineHtml(raw) => Event::Text(raw),
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Link {
            link_type,
            dest_url: safe_link(&dest_url).unwrap_or_default().into(),
            title,
            id,
        }),
        // Una imagen del texto sería un recurso nuevo en el proveedor: va como enlace
        Event::Start(Tag::Image {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Link {
            link_type,
            dest_url: safe_link(&dest_url).unwrap_or_default().into(),
            title,
            id,
        }),
        Event::End(pulldown_cmark::TagEnd::Image) => Event::End(pulldown_cmark::TagEnd::Link),
        other => other,
    });
    let mut out = String::new();
    html::push_html(&mut out, events);
    out.trim().to_string()
}

fn looks_like_html(text: &str) -> bool {
    text.match_indices('<').any(|(at, _)| {
        let rest = &text[at + 1..];
        rest.strip_prefix('/')
            .unwrap_or(rest)
            .starts_with(|c: char| c.is_ascii_alphabetic())
            && rest.contains('>')
    })
}

/// `format` es el que declare el proveedor (`html` o `markdown`); sin él, se deduce del texto.
pub fn from_text(text: &str, format: Option<&str>) -> String {
    match format.map(str::to_ascii_lowercase).as_deref() {
        Some("html") => from_html(text),
        Some("markdown") => from_markdown(text),
        _ if looks_like_html(text) => from_html(text),
        _ => from_markdown(text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conserva_la_estructura() {
        assert_eq!(
            from_html("<h2>T</h2><div>uno <b>dos</b></div><ul><li>a</li><li>b</li></ul>"),
            "## T\n\nuno **dos**\n\n- a\n- b"
        );
        assert_eq!(
            from_html("<table><tr><th>a</th></tr><tr><td>1</td></tr></table>"),
            "| a |\n| - |\n| 1 |"
        );
    }

    #[test]
    fn el_texto_que_parece_html_no_lo_es() {
        let found = from_html("<p>a &lt;img src=x onerror=1&gt; b</p>");
        assert_eq!(found, "a \\<img src=x onerror=1\\> b");
        let fenced = from_html("<pre><code>```\n&lt;img src=x&gt;</code></pre>");
        assert_eq!(fenced.matches("```").count(), 2, "{fenced}");
    }

    #[test]
    fn quita_el_codigo_y_lo_que_se_cargaria_solo() {
        let found = from_html(
            r#"<p><a href="javascript:alert(1)">mal</a> <a href="https://e.test/a b?q=(1)">bien</a>
<img src="https://e.test/i.png" alt="foto"><img src="data:image/png;base64,AAA="></p>
<script>x()</script><style>p{}</style><iframe src="https://e.test">dentro</iframe>"#,
        );
        for gone in ["javascript", "![", "x()", "p{}", "dentro", "base64"] {
            assert!(!found.contains(gone), "{gone} en {found}");
        }
        assert!(found.contains("mal"), "{found}");
        assert!(
            found.contains("[bien](https://e.test/a%20b?q=%281%29)"),
            "{found}"
        );
        assert!(found.contains("[foto](https://e.test/i.png)"), "{found}");
    }

    #[test]
    fn el_markdown_se_deja_sin_html_ni_imagenes() {
        assert_eq!(
            from_markdown("**a** <img src=x> ![i](https://e.test/i) [m](JavaScript:x)\r\nfin"),
            "**a** &lt;img src=x> [i](https://e.test/i) [m](x)\nfin"
        );
    }

    #[test]
    fn ningun_destino_de_enlace_ejecuta_nada() {
        let found = from_markdown(
            "[a]( javascript:x) [b](<data:y>) [c](javascript:javascript:z)\n[d]: vbscript:w",
        );
        for scheme in UNSAFE {
            assert!(!found.contains(scheme), "{scheme} en {found}");
        }
    }

    #[test]
    fn distingue_el_formato_que_se_perderia() {
        assert!(!loses_format(
            r#"<div>a <b>b</b><br><a href="https://e.test">c</a></div><ul><li>d</li></ul>"#
        ));
        for lossy in [
            r#"<div style="color:red">a</div>"#,
            r#"<img src="https://e.test/i.png">"#,
            r##"<a href="#" data-vss-mention="version:2.0">@Ana</a>"##,
            "<table><tr><td colspan=\"2\">a</td></tr></table>",
            "<u>a</u>",
        ] {
            assert!(loses_format(lossy), "{lossy}");
        }
    }

    #[test]
    fn el_html_enviado_no_lleva_lo_que_venga_escrito() {
        let sent =
            to_html("**a** <script>x()</script>\n\n[m](javascript:x) ![i](https://e.test/i.png)");
        assert!(
            sent.starts_with("<p><strong>a</strong> &lt;script&gt;"),
            "{sent}"
        );
        for gone in ["<script", "javascript:", "<img"] {
            assert!(!sent.contains(gone), "{gone} en {sent}");
        }
        assert!(
            sent.contains(r#"<a href="https://e.test/i.png">i</a>"#),
            "{sent}"
        );
    }

    #[test]
    fn deduce_el_formato_si_no_se_declara() {
        assert_eq!(from_text("<div>a<br>b</div>", None), "a  \nb");
        assert_eq!(from_text("a < b\nc > d", None), "a &lt; b\nc > d");
        assert_eq!(from_text("**x**", Some("markdown")), "**x**");
    }
}
