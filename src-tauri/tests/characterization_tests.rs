//! Pruebas de caracterización (etapa 0 del refactor de arquitectura).
//!
//! Congelan el comportamiento actual del Quick Open y de la resolución de enlaces para que
//! el refactor lo conserve exactamente: mismo orden de resultados con las mismas entradas.
//! Si una de estas pruebas falla tras mover código, es una regresión, no un ajuste.
use app_lib::infrastructure::navigation::engine::NavigationEngine;
use app_lib::infrastructure::navigation::model::{NoteMeta, NoteSyncState};
use compact_str::CompactString;

const HOUR: u128 = 60 * 60 * 1_000_000_000;

fn now_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

fn note(id: u32, path: &str, title: &str, tags: &[&str], opened_hours_ago: Option<u128>) -> NoteMeta {
    NoteMeta {
        id,
        path: CompactString::new(path),
        title: CompactString::new(title),
        tags: tags.iter().map(|t| CompactString::new(*t)).collect(),
        mtime_nanos: 1,
        size_bytes: 1,
        created_nanos: None,
        last_opened_nanos: opened_hours_ago.map(|h| now_nanos() - h * HOUR),
        is_open: false,
        tab_order: None,
        is_active_tab: false,
        view_mode: None,
        sync_state: NoteSyncState::Local,
    }
}

fn engine(tag: &str) -> NavigationEngine {
    let dir = std::env::temp_dir().join(format!("synapse_charac_{}_{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let engine = NavigationEngine::new(dir.clone(), dir.join("cache.bin"));
    let notes = [
        note(1, "proyectos/BCP/TTIB/reunion-bcp.md", "Reunión BCP", &["trabajo/bcp"], Some(1)),
        note(2, "proyectos/BCP/TTIB/arquitectura.md", "Arquitectura TTIB", &["arquitectura"], None),
        note(3, "bcp.md", "Notas generales", &[], None),
        note(4, "diario/2026/bcp-semana.md", "Semana", &["diario"], Some(72)),
        note(5, "recetas/pan.md", "Pan casero", &["cocina"], Some(720)),
        note(6, "docs/arquitectura/cebolla.md", "Arquitectura cebolla", &["arquitectura", "rust"], None),
        note(7, "docs/arquitectura/fsd.md", "Feature Sliced Design", &["arquitectura", "front"], Some(48)),
        note(8, "inbox/idea-rust.md", "Idea", &["rust"], None),
        note(9, "a/notas.md", "Notas A", &[], None),
        note(10, "b/notas.md", "Notas B", &[], None),
        note(11, "a/sub/notas.md", "Notas A sub", &[], None),
        note(12, "imagenes/diagrama.png", "diagrama.png", &[], None),
    ];
    for n in notes {
        engine.notes.insert(n.id, n);
    }
    engine
}

fn search_paths(engine: &NavigationEngine, query: &str) -> Vec<String> {
    engine.search(query, 20).into_iter().map(|m| m.path.to_string()).collect()
}

#[test]
fn quick_open_order_is_preserved() {
    let e = engine("qo");
    let mut report = String::new();
    // "notas" no se incluye: hoy los empates entre archivos con el mismo nombre y puntaje
    // dependen del orden de iteración del DashMap, que cambia entre ejecuciones.
    for q in ["bcp", "arquitectura", "rust", "ttib", "pan", "cebolla", "diagrama", "arq ceb"] {
        report.push_str(&format!("{q:?} => {:?}\n", search_paths(&e, q)));
    }
    insta_like(&report, include_str!("characterization/quick_open.txt"));
}

#[test]
fn link_resolution_is_preserved() {
    let e = engine("links");
    let mut report = String::new();
    for (target, base) in [
        ("notas", None),
        ("notas", Some("a/sub/x.md")),
        ("notas", Some("a/x.md")),
        ("notas", Some("b/x.md")),
        ("notas", Some("c/x.md")),
        ("notas.md#seccion", Some("b/x.md")),
        ("pan", Some("a/x.md")),
        ("diagrama.png", None),
        ("no-existe", None),
        ("docs/arquitectura/fsd", None),
    ] {
        report.push_str(&format!("{target:?} desde {base:?} => {:?}\n", e.resolve_link_path_with_base(target, base)));
    }
    insta_like(&report, include_str!("characterization/links.txt"));
}

/// Compara contra la instantánea guardada. Con `UPDATE_CHARACTERIZATION=1` imprime el valor
/// actual para regenerarla (solo al crearla; nunca durante el refactor).
fn insta_like(actual: &str, expected: &str) {
    if std::env::var("UPDATE_CHARACTERIZATION").is_ok() {
        println!("-----\n{actual}-----");
        return;
    }
    assert_eq!(actual, expected, "el comportamiento cambió respecto a la instantánea");
}
