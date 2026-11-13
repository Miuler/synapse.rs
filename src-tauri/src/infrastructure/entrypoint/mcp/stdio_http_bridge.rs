//! Puente stdio → HTTP hacia el MCP de la instancia de Synapse que está corriendo.
//!
//! Los agentes que solo hablan MCP por stdio lanzan `synapsers --mcp-proxy <url>`: cada línea
//! JSON-RPC de `stdin` se reenvía por POST al servidor MCP de la app abierta y la respuesta se
//! escribe en `stdout`. Así todos los agentes usan el mismo índice y estado que la ventana.

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

/// Envía un mensaje JSON-RPC y devuelve el cuerpo de la respuesta (vacío para notificaciones).
pub fn post(url: &str, body: &str) -> Result<String, String> {
    let (addr, path) = split_url(url)?;
    let mut stream =
        TcpStream::connect(&addr).map_err(|e| format!("Synapse no responde en {addr}: {e}"))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(300)));
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\n\
         Accept: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
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

/// Bucle del puente: una línea de `stdin`, un POST, una línea a `stdout`.
pub fn run(url: &str, input: impl BufRead, mut output: impl Write) -> Result<(), String> {
    for line in input.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        match post(url, line.trim()) {
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
