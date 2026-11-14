//! Mediciones de línea base del refactor de arquitectura.
//!
//! Se ejecutan a mano contra una bóveda real, antes de empezar y al final de cada etapa:
//!
//! ```bash
//! SYNAPSE_BENCH_VAULT=/ruta/a/la/boveda cargo test --release --test baseline_measurements -- --ignored --nocapture
//! ```
//!
//! Miden tres operaciones: abrir la bóveda (escaneo y reconciliación en frío), búsqueda rápida
//! y guardar una nota grande. La nota de prueba se escribe en un directorio temporal, nunca en
//! la bóveda.
use app_lib::domain::note::note::Note;
use app_lib::domain::note::note_path::NoteRelativePath;
use app_lib::domain::note::note_repository::NoteRepository;
use app_lib::infrastructure::navigation::engine::NavigationEngine;
use app_lib::infrastructure::repository::file::note_repository_impl::NoteRepositoryImpl;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn median(mut v: Vec<Duration>) -> Duration {
    v.sort();
    v[v.len() / 2]
}

#[test]
#[ignore]
fn baseline_measurements() {
    let vault =
        PathBuf::from(std::env::var("SYNAPSE_BENCH_VAULT").expect("definir SYNAPSE_BENCH_VAULT"));
    let tmp = std::env::temp_dir().join(format!("synapse_bench_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    // 1. Abrir la bóveda en frío (sin caché previa).
    let mut open_times = Vec::new();
    let mut engine = None;
    for i in 0..3 {
        let e = NavigationEngine::new(vault.clone(), tmp.join(format!("cache_{i}.bin")));
        let t = Instant::now();
        e.reconcile_sync();
        open_times.push(t.elapsed());
        engine = Some(e);
    }
    let engine = engine.unwrap();
    let notes = engine.notes.len();

    // 2. Búsqueda rápida (Quick Open).
    let queries = [
        "a", "no", "nota", "proy", "arq ceb", "2026", "readme", "xyz123",
    ];
    let mut search_times = Vec::new();
    for _ in 0..20 {
        for q in queries {
            let t = Instant::now();
            let _ = engine.search(q, 50);
            search_times.push(t.elapsed());
        }
    }

    // 3. Guardar una nota grande (~2 MB) fuera de la bóveda.
    let repo = NoteRepositoryImpl::new();
    let content = "Línea de prueba con [[enlace]] y #etiqueta.\n".repeat(50_000);
    let mut save_times = Vec::new();
    for _ in 0..10 {
        let note = Note::new(
            NoteRelativePath::new("bench/grande.md").unwrap(),
            tmp.join("bench/grande.md").to_string_lossy().to_string(),
            "Grande".into(),
            content.clone(),
        );
        let t = Instant::now();
        repo.save_note(&tmp, &note).unwrap();
        save_times.push(t.elapsed());
    }

    println!("notas indexadas:            {notes}");
    println!("abrir bóveda (mediana):     {:?}", median(open_times));
    println!("búsqueda rápida (mediana):  {:?}", median(search_times));
    println!("guardar nota 2 MB (mediana): {:?}", median(save_times));
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Costo de las dos listas que pide el front al abrir la bóveda: la de notas (explorador) y la de
/// archivos (fachada de Obsidian). Mide tamaño del JSON y tiempo de serialización en Rust; el
/// webview además tiene que parsearlo.
#[test]
#[ignore]
fn vault_open_payloads() {
    use app_lib::domain::note::{Note, NoteRelativePath};
    use app_lib::infrastructure::entrypoint::tauri::dto::VaultFilesResponse;
    use std::sync::Arc;

    let vault =
        PathBuf::from(std::env::var("SYNAPSE_BENCH_VAULT").expect("definir SYNAPSE_BENCH_VAULT"));
    let tmp = std::env::temp_dir().join(format!("synapse_payload_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let engine = Arc::new(NavigationEngine::new(vault.clone(), tmp.join("cache.bin")));
    engine.reconcile_sync();

    // Lista del explorador, como la arma activate_vault
    let mut notes = Vec::new();
    for item in engine.notes.iter() {
        let meta = item.value();
        let rel = meta.path.to_string();
        if let Ok(rel_path) = NoteRelativePath::new(&rel) {
            notes.push(Note::new(
                rel_path,
                vault.join(&rel).to_string_lossy().to_string(),
                meta.title.to_string(),
                String::new(),
            ));
        }
    }
    let files = VaultFilesResponse {
        engine: Some(Arc::clone(&engine)),
    };

    let mut t_notes = Vec::new();
    let mut t_files = Vec::new();
    let (mut b_notes, mut b_files) = (0, 0);
    for _ in 0..10 {
        let t = Instant::now();
        b_notes = serde_json::to_string(&notes).unwrap().len();
        t_notes.push(t.elapsed());
        let t = Instant::now();
        b_files = serde_json::to_string(&files).unwrap().len();
        t_files.push(t.elapsed());
    }
    if let Ok(dir) = std::env::var("SYNAPSE_DUMP_DIR") {
        std::fs::write(
            format!("{dir}/notes.json"),
            serde_json::to_string(&notes).unwrap(),
        )
        .unwrap();
        std::fs::write(
            format!("{dir}/files.json"),
            serde_json::to_string(&files).unwrap(),
        )
        .unwrap();
    }
    println!("elementos:                 {}", notes.len());
    println!(
        "lista explorador:          {:.2} MB, serializar {:?}",
        b_notes as f64 / 1048576.0,
        median(t_notes)
    );
    println!(
        "lista fachada Obsidian:    {:.2} MB, serializar {:?}",
        b_files as f64 / 1048576.0,
        median(t_files)
    );
    let _ = std::fs::remove_dir_all(&tmp);
}
