//! Guardián de capas de la arquitectura cebolla (`domain` ← `application` ← `infrastructure`).
//!
//! Dependencias externas permitidas en `domain` y `application`:
//! - `std`, `core`, `alloc`: biblioteca estándar (sin `std::fs` ni `std::process`).
//! - `thiserror`: definición de errores tipados sin acoplamiento a infraestructura.
//! - `tracing`: únicamente macros de trazabilidad (`tracing::info!`, `#[tracing::instrument]`).
//! - `uuid`: identificadores de negocio (`TenantId`, `version_id`).
//! - `blake3`: únicamente para el cálculo Merkle y el tipo `ContentHash`.
//! - `async_trait`: únicamente si un trait de puerto se usa como `dyn`.

use std::fs;
use std::path::{Path, PathBuf};

const CRATES_PROHIBIDOS: &[&str] = &[
    "serde",
    "sqlx",
    "tonic",
    "prost",
    "object_store",
    "reqwest",
    "jsonwebtoken",
    "axum",
    "figment",
    "clap",
    "tower",
    "tower_http",
    "humantime_serde",
    "anyhow",
];

const RUTAS_IO_PROHIBIDAS: &[&str] = &["tokio::fs", "std::fs", "std::process"];

fn archivos_rs_en(dir: &Path) -> Vec<PathBuf> {
    let mut resultados = Vec::new();
    let Ok(entradas) = fs::read_dir(dir) else {
        return resultados;
    };
    for entrada in entradas.flatten() {
        let ruta = entrada.path();
        if ruta.is_dir() {
            resultados.extend(archivos_rs_en(&ruta));
        } else if ruta.extension().and_then(|e| e.to_str()) == Some("rs") {
            resultados.push(ruta);
        }
    }
    resultados
}

fn contiene_token_de_ruta(linea: &str, patron: &str) -> bool {
    let mut inicio = 0;
    while let Some(pos) = linea[inicio..].find(patron) {
        let abs = inicio + pos;
        let antes = linea[..abs].chars().next_back();
        let despues = linea[abs + patron.len()..].chars().next();
        let borde_izq = antes.is_none_or(|c| !c.is_alphanumeric() && c != '_');
        let borde_der = despues.is_none_or(|c| !c.is_alphanumeric() && c != '_');
        if borde_izq && borde_der {
            return true;
        }
        inicio = abs + patron.len();
    }
    false
}

fn verificar_capa(subcarpeta: &str, capas_prohibidas: &[&str]) {
    let raiz = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(subcarpeta);
    let archivos = archivos_rs_en(&raiz);
    assert!(
        !archivos.is_empty(),
        "no se encontraron archivos .rs en {}",
        raiz.display()
    );

    let mut infracciones = Vec::new();

    for archivo in archivos {
        let contenido = fs::read_to_string(&archivo)
            .unwrap_or_else(|e| panic!("no se pudo leer {}: {e}", archivo.display()));

        for (num_linea, linea) in contenido.lines().enumerate() {
            let codigo = linea.split("//").next().unwrap_or("");
            if codigo.trim().is_empty() {
                continue;
            }

            for capa in capas_prohibidas {
                if contiene_token_de_ruta(codigo, capa) {
                    infracciones.push(format!(
                        "{}:{} importa capa prohibida '{}': {}",
                        archivo.display(),
                        num_linea + 1,
                        capa,
                        codigo.trim()
                    ));
                }
            }

            for ruta_io in RUTAS_IO_PROHIBIDAS {
                if contiene_token_de_ruta(codigo, ruta_io) {
                    infracciones.push(format!(
                        "{}:{} usa I/O prohibido '{}': {}",
                        archivo.display(),
                        num_linea + 1,
                        ruta_io,
                        codigo.trim()
                    ));
                }
            }

            for crate_prohibido in CRATES_PROHIBIDOS {
                let patron_use = format!("use {crate_prohibido}");
                let patron_ruta = format!("{crate_prohibido}::");
                if contiene_token_de_ruta(codigo, &patron_use)
                    || contiene_token_de_ruta(codigo, &patron_ruta)
                {
                    infracciones.push(format!(
                        "{}:{} importa crate prohibido '{}': {}",
                        archivo.display(),
                        num_linea + 1,
                        crate_prohibido,
                        codigo.trim()
                    ));
                }
            }
        }
    }

    assert!(
        infracciones.is_empty(),
        "infracciones de arquitectura en '{subcarpeta}':\n{}",
        infracciones.join("\n")
    );
}

#[test]
fn dominio_no_conoce_el_exterior() {
    verificar_capa("domain", &["crate::application", "crate::infrastructure"]);
}

#[test]
fn aplicacion_solo_conoce_el_dominio() {
    verificar_capa("application", &["crate::infrastructure"]);
}
