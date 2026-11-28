//! Puente stdio → HTTP hacia el MCP de la instancia de Synapse que está corriendo.
//!
//! Los agentes que solo hablan MCP por stdio lanzan `synapsers --mcp-proxy <url>`: cada línea
//! JSON-RPC de `stdin` se reenvía por POST al servidor MCP de la app abierta y la respuesta se
//! escribe en `stdout`. Así todos los agentes usan el mismo índice y estado que la ventana.

use super::server::{SESSION_ENV, SESSION_HEADER};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// Separa `http://host:puerto/ruta` en dirección y ruta.
fn split_url(url: &str) -> Result<(String, String), String> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| format!("URL MCP no soportada: {url}"))?;
    let (addr, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    Ok((addr.to_string(), path.to_string()))
}

/// Lo que se espera una respuesta del MCP.
const TIMEOUT: Duration = Duration::from_secs(300);

/// Envía un mensaje JSON-RPC, con cabeceras extra y un tiempo máximo de espera, y devuelve el cuerpo
/// de la respuesta (vacío para notificaciones).
pub fn post_with(
    url: &str,
    body: &str,
    headers: &[(&str, &str)],
    timeout: Duration,
) -> Result<String, String> {
    let (addr, path) = split_url(url)?;
    let socket = addr
        .parse::<std::net::SocketAddr>()
        .map_err(|e| format!("Dirección inválida {addr}: {e}"))?;
    let mut stream = TcpStream::connect_timeout(&socket, Duration::from_secs(3))
        .map_err(|e| format!("Synapse no responde en {addr}: {e}"))?;
    let _ = stream.set_read_timeout(Some(timeout));
    let extra: String = headers
        .iter()
        .map(|(k, v)| format!("{k}: {v}\r\n"))
        .collect();
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\n\
         Accept: application/json\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(stream);
    let mut length = 0usize;
    let mut status = String::new();
    reader.read_line(&mut status).map_err(|e| e.to_string())?;
    if !status.contains(" 200 ") && !status.contains(" 202 ") {
        return Err(format!("Respuesta MCP inesperada: {}", status.trim()));
    }
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 || line.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':')
            && k.trim().eq_ignore_ascii_case("content-length")
        {
            length = v.trim().parse().unwrap_or(0);
        }
    }
    let mut buf = vec![0u8; length];
    reader.read_exact(&mut buf).map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// Bucle del puente: una línea de `stdin`, un POST, una línea a `stdout`. El token de sesión que
/// Synapse le deja en el entorno viaja como cabecera.
pub fn run(url: &str, input: impl BufRead, output: impl Write) -> Result<(), String> {
    let token = std::env::var(SESSION_ENV).ok();
    run_as(url, token.as_deref(), input, output)
}

fn run_as(
    url: &str,
    token: Option<&str>,
    input: impl BufRead,
    mut output: impl Write,
) -> Result<(), String> {
    let headers: Vec<(&str, &str)> = token.map(|t| (SESSION_HEADER, t)).into_iter().collect();
    for line in input.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        match post_with(url, line.trim(), &headers, TIMEOUT) {
            Ok(resp) if !resp.trim().is_empty() => {
                writeln!(output, "{}", resp.trim()).map_err(|e| e.to_string())?;
                output.flush().map_err(|e| e.to_string())?;
            }
            Ok(_) => {}
            Err(e) => {
                // Responder el error para que el agente no quede esperando
                let id = serde_json::from_str::<serde_json::Value>(&line)
                    .ok()
                    .and_then(|v| v.get("id").cloned());
                if let Some(id) = id {
                    let err = serde_json::json!({"jsonrpc": "2.0", "id": id,
                        "error": {"code": -32603, "message": e}});
                    writeln!(output, "{err}").map_err(|e| e.to_string())?;
                    output.flush().map_err(|e| e.to_string())?;
                }
                eprintln!("[Synapse MCP proxy] {e}");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn el_token_de_sesion_viaja_como_cabecera() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut head = String::new();
            while !head.ends_with("\r\n\r\n") {
                reader.read_line(&mut head).unwrap();
            }
            let mut body = [0; 2];
            reader.read_exact(&mut body).unwrap();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n").unwrap();
            head
        });
        let url = format!("http://127.0.0.1:{port}/mcp");
        run_as(&url, Some("secreto"), "{}\n".as_bytes(), Vec::new()).unwrap();
        let head = server.join().unwrap();
        assert!(head.contains("x-synapse-session: secreto\r\n"), "{head}");
    }

    #[test]
    fn reenvia_por_http_y_devuelve_la_respuesta() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for stream in listener.incoming().take(2) {
                let mut stream = stream.unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut len = 0;
                loop {
                    let mut l = String::new();
                    reader.read_line(&mut l).unwrap();
                    if l.trim().is_empty() {
                        break;
                    }
                    if let Some(v) = l.strip_prefix("Content-Length: ") {
                        len = v.trim().parse().unwrap();
                    }
                }
                let mut body = vec![0; len];
                reader.read_exact(&mut body).unwrap();
                let req: serde_json::Value = serde_json::from_slice(&body).unwrap();
                // Las notificaciones (sin id) no tienen respuesta
                let resp = match req.get("id") {
                    Some(id) => {
                        serde_json::json!({"jsonrpc": "2.0", "id": id, "result": {"ok": true}})
                            .to_string()
                    }
                    None => String::new(),
                };
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{resp}",
                    resp.len()
                )
                .unwrap();
            }
        });
        let input = "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"tools/list\"}\n";
        let mut out = Vec::new();
        run(
            &format!("http://127.0.0.1:{port}/mcp"),
            input.as_bytes(),
            &mut out,
        )
        .unwrap();
        let out = String::from_utf8(out).unwrap();
        assert_eq!(out.lines().count(), 1);
        assert!(out.contains("\"id\":7"));
    }
}
