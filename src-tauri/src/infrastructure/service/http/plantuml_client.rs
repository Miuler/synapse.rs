use serde::Serialize;
use std::sync::OnceLock;
use std::time::Duration;

/// `svg` si el diagrama se renderizó; `error` y `error_line` si el servidor rechazó su sintaxis.
#[derive(Debug, Serialize)]
pub struct PlantUmlResponse {
    pub svg: Option<String>,
    pub error: Option<String>,
    pub error_line: Option<u32>,
}

fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent("Synapse-App/0.1.0 (https://github.com/Miuler/synapse.rs)")
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

/// URL `{server}/svg/{encoded}`. Solo http(s) y el alfabeto de la codificación de PlantUML, para que
/// `encoded` no pueda cambiar la ruta pedida.
fn svg_url(server: &str, encoded: &str) -> Result<String, String> {
    let server = server.trim().trim_end_matches('/');
    if !(server.starts_with("http://") || server.starts_with("https://")) {
        return Err(format!(
            "Invalid PlantUML server URL: {server} (must start with http:// or https://)"
        ));
    }
    if !encoded
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '~')
    {
        return Err("Invalid encoded diagram".to_string());
    }
    Ok(format!("{server}/svg/{encoded}"))
}

fn header(response: &reqwest::Response, name: &str) -> Option<String> {
    response
        .headers()
        .get(name)
        .map(|value| String::from_utf8_lossy(value.as_bytes()).trim().to_string())
}

/// Pide el SVG al servidor. Va por Rust y no por el webview para no depender del CORS del servidor.
pub async fn fetch_svg(server: &str, encoded: &str) -> Result<PlantUmlResponse, String> {
    let url = svg_url(server, encoded)?;
    let response = client()
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Could not connect to PlantUML server {server}: {e}"))?;

    let status = response.status();
    if let Some(error) = header(&response, "x-plantuml-diagram-error") {
        let error_line =
            header(&response, "x-plantuml-diagram-error-line").and_then(|l| l.parse().ok());
        return Ok(PlantUmlResponse {
            svg: None,
            error: Some(error),
            error_line,
        });
    }
    if status == reqwest::StatusCode::URI_TOO_LONG {
        return Err(format!(
            "Diagram is too long for PlantUML server {server}"
        ));
    }
    if !status.is_success() {
        return Err(format!("PlantUML server {server} responded with {status}"));
    }

    let svg = response
        .text()
        .await
        .map_err(|e| format!("Could not read response from PlantUML server: {e}"))?;
    if !svg.contains("<svg") {
        return Err(format!("Server {server} did not return an SVG"));
    }
    Ok(PlantUmlResponse {
        svg: Some(svg),
        error: None,
        error_line: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_url_quita_la_barra_final() {
        assert_eq!(
            svg_url("https://example.test/plantuml/", "SoWk~1_-").unwrap(),
            "https://example.test/plantuml/svg/SoWk~1_-"
        );
    }

    #[test]
    fn svg_url_rechaza_otros_esquemas() {
        assert!(svg_url("file:///etc", "abc").is_err());
    }

    #[test]
    fn svg_url_rechaza_rutas_en_el_codigo() {
        assert!(svg_url("https://example.test", "../secreto").is_err());
        assert!(svg_url("https://example.test", "a?b=c").is_err());
    }
}
