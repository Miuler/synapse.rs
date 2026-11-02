//! Guardián de la Arquitectura Cebolla: las capas internas no conocen a las externas.
use std::fs;
use std::path::{Path, PathBuf};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn offenders(layer: &str, forbidden: &[&str], allowed: &[&str]) -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(layer);
    let mut files = Vec::new();
    rust_files(&root, &mut files);
    let mut found = Vec::new();
    for file in files {
        let rel = file.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
        if allowed.contains(&rel.as_str()) {
            continue;
        }
        let text = fs::read_to_string(&file).unwrap();
        // Solo código de producción: se ignora el módulo de tests.
        let prod = text.split("#[cfg(test)]").next().unwrap_or(&text);
        for (i, line) in prod.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            if forbidden.iter().any(|f| line.contains(f)) {
                found.push(format!("{layer}/{rel}:{}: {}", i + 1, line.trim()));
            }
        }
    }
    found
}

#[test]
fn domain_does_not_know_outer_layers() {
    let bad = offenders(
        "domain",
        &["crate::application", "crate::infrastructure", "tauri::", "std::fs", "std::process", "tantivy", "reqwest"],
        &[],
    );
    assert!(bad.is_empty(), "El dominio rompe la regla centrípeta:\n{}", bad.join("\n"));
}

#[test]
fn application_does_not_know_infrastructure() {
    // Deuda conocida, a eliminar: mcp_use_cases y community_plugin_use_cases aún hacen I/O directo.
    let bad = offenders(
        "application",
        &["crate::infrastructure", "tauri::", "std::fs", "std::process"],
        &["use_cases/mcp_use_cases.rs", "use_cases/community_plugin_use_cases.rs"],
    );
    assert!(bad.is_empty(), "La aplicación rompe la regla centrípeta:\n{}", bad.join("\n"));
}
