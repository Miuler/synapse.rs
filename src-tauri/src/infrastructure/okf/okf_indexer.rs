//! Mantiene un `index.md` (OKF v0.3) en cada carpeta de la bóveda que tenga archivos indexados.
//!
//! - Al arrancar regenera todas; después, con cada cambio de la bóveda, solo la carpeta afectada y
//!   su carpeta padre (sus conteos cambian), agrupando los cambios unos instantes.
//! - Solo escribe si el contenido cambia, para no ensuciar Git ni despertar al watcher.
//! - Un `index.md` que no generó Synapse se renombra a `index.deprecado.md` y se genera el nuevo.
//! - Si una carpeta queda sin archivos, su `index.md` generado se borra.
use crate::domain::note::frontmatter::NoteFrontmatter;
use crate::domain::okf::{frontmatter_block, is_generated, FolderIndex, IndexEntry, INDEX_FILE, LOG_FILE};
use crate::infrastructure::navigation::model::{NoteId, NoteMeta};
use dashmap::DashMap;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Espera para agrupar cambios seguidos antes de regenerar
const DEBOUNCE: Duration = Duration::from_millis(1_500);

pub struct OkfIndexer {
    vault: PathBuf,
    notes: Arc<DashMap<NoteId, NoteMeta>>,
    pending: Mutex<HashSet<String>>,
    scheduled: AtomicBool,
}

/// Carpeta de una ruta relativa (`""` = raíz)
fn parent_of(path: &str) -> &str {
    path.rfind('/').map(|i| &path[..i]).unwrap_or("")
}

fn name_of(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Archivos que no entran en el índice: el propio índice, el log, lo oculto y las carpetas de
/// tablero (`Nombre.board/`), que solo escribe su tablero
fn skipped(path: &str) -> bool {
    let name = name_of(path);
    name == INDEX_FILE
        || name == LOG_FILE
        || path.split('/').any(|seg| seg.starts_with('.'))
        || path.split('/').rev().skip(1).any(|folder| folder.ends_with(".board"))
}

impl OkfIndexer {
    pub fn start(vault: PathBuf, notes: Arc<DashMap<NoteId, NoteMeta>>) -> Arc<Self> {
        let me = Arc::new(Self { vault, notes, pending: Mutex::new(HashSet::new()), scheduled: AtomicBool::new(false) });
        // Pasada completa al arrancar (tras dar tiempo a la primera reconciliación)
        let full = Arc::clone(&me);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(3));
            full.regenerate_all();
        });
        me
    }

    /// Cambios de la bóveda: se regeneran (agrupados) las carpetas afectadas y sus padres
    pub fn on_change(self: &Arc<Self>, upserted: &[String], removed: &[String]) {
        let mut folders = HashSet::new();
        for p in upserted.iter().chain(removed.iter()) {
            if name_of(p) == INDEX_FILE {
                continue; // lo escribimos nosotros
            }
            let dir = parent_of(p).to_string();
            if !dir.is_empty() {
                folders.insert(parent_of(&dir).to_string());
            }
            folders.insert(dir);
        }
        if folders.is_empty() {
            return;
        }
        if let Ok(mut pending) = self.pending.lock() {
            pending.extend(folders);
        }
        if self.scheduled.swap(true, Ordering::SeqCst) {
            return;
        }
        let me = Arc::clone(self);
        std::thread::spawn(move || {
            std::thread::sleep(DEBOUNCE);
            me.scheduled.store(false, Ordering::SeqCst);
            let folders: Vec<String> = me.pending.lock().map(|mut p| p.drain().collect()).unwrap_or_default();
            let tree = me.tree();
            for f in folders {
                me.write_folder(&f, &tree);
            }
        });
    }

    /// Carpeta → archivos directos, de todos los archivos indexados (y sus carpetas antecesoras)
    fn tree(&self) -> BTreeMap<String, BTreeSet<String>> {
        let mut tree: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        tree.entry(String::new()).or_default();
        for item in self.notes.iter() {
            let path = item.value().path.as_str();
            if skipped(path) {
                continue;
            }
            tree.entry(parent_of(path).to_string()).or_default().insert(name_of(path).to_string());
            let mut dir = parent_of(path);
            while !dir.is_empty() {
                let parent = parent_of(dir);
                tree.entry(dir.to_string()).or_default();
                tree.entry(parent.to_string()).or_default();
                dir = parent;
            }
        }
        tree
    }

    pub fn regenerate_all(&self) {
        let tree = self.tree();
        for folder in tree.keys() {
            self.write_folder(folder, &tree);
        }
    }

    fn abs(&self, rel: &str) -> PathBuf {
        if rel.is_empty() {
            self.vault.clone()
        } else {
            self.vault.join(rel)
        }
    }

    /// Índice de una carpeta a partir del árbol y del frontmatter de sus notas
    fn build(&self, folder: &str, tree: &BTreeMap<String, BTreeSet<String>>) -> FolderIndex {
        let mut idx = FolderIndex::default();
        let prefix = if folder.is_empty() { String::new() } else { format!("{folder}/") };
        for (dir, files) in tree.range(prefix.clone()..) {
            if !dir.starts_with(&prefix) || dir == folder {
                if !dir.starts_with(&prefix) {
                    break;
                }
                continue;
            }
            let rest = &dir[prefix.len()..];
            if rest.is_empty() || rest.contains('/') {
                continue; // solo subcarpetas directas
            }
            let subdirs = tree.keys().filter(|k| parent_of(k) == dir.as_str() && !k.is_empty()).count();
            let mut parts = Vec::new();
            if subdirs > 0 {
                parts.push(format!("{subdirs} carpeta{}", if subdirs == 1 { "" } else { "s" }));
            }
            if !files.is_empty() {
                parts.push(format!("{} archivo{}", files.len(), if files.len() == 1 { "" } else { "s" }));
            }
            idx.subfolders.push(IndexEntry { name: rest.to_string(), title: rest.to_string(), description: Some(parts.join(", ")) });
        }
        for name in tree.get(folder).into_iter().flatten() {
            let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name).to_string();
            if name.to_lowercase().ends_with(".md") {
                let content = std::fs::read_to_string(self.abs(&format!("{prefix}{name}"))).unwrap_or_default();
                let (fm, body) = NoteFrontmatter::parse(&content);
                let fm = fm.unwrap_or_default();
                // Título: el del frontmatter, si no el primer `# …`, si no el nombre del archivo
                let h1 = body.lines().find_map(|l| l.strip_prefix("# ")).map(|t| t.trim().to_string()).filter(|t| !t.is_empty());
                let title = fm.title.clone().filter(|t| !t.trim().is_empty()).or(h1).unwrap_or(stem);
                let description = fm.extra_properties.get("description").and_then(|v| v.as_str()).map(String::from);
                let entry = IndexEntry { name: name.clone(), title, description };
                // Concepto OKF: `.md` con `type`
                if fm.node_type.as_deref().is_some_and(|t| !t.trim().is_empty()) {
                    idx.concepts.push(entry);
                } else {
                    idx.files.push(entry);
                }
            } else {
                idx.files.push(IndexEntry { name: name.clone(), title: name.clone(), description: None });
            }
        }
        idx
    }

    fn write_folder(&self, folder: &str, tree: &BTreeMap<String, BTreeSet<String>>) {
        let dir = self.abs(folder);
        let path = dir.join(INDEX_FILE);
        let existing = std::fs::read_to_string(&path).ok();
        // Carpeta sin archivos: se quita el índice generado
        if !tree.contains_key(folder) {
            if existing.as_deref().is_some_and(is_generated) {
                let _ = std::fs::remove_file(&path);
            }
            return;
        }
        if !dir.is_dir() {
            return;
        }
        // Un `index.md` del usuario se conserva como `index.deprecado.md`
        if let Some(text) = existing.as_deref()
            && !is_generated(text) {
                let target = Self::free_name(&dir, "index.deprecado", "md");
                if let Err(e) = std::fs::rename(&path, &target) {
                    log::warn!("No se pudo renombrar {:?}: {}", path, e);
                    return;
                }
                log::info!("index.md propio renombrado a {:?}", target);
            }
        let previous_root_fm = if folder.is_empty() {
            existing.as_deref().filter(|t| is_generated(t)).and_then(frontmatter_block).map(String::from)
        } else {
            None
        };
        let content = self.build(folder, tree).render(folder.is_empty(), previous_root_fm.as_deref());
        if existing.as_deref() == Some(content.as_str()) {
            return;
        }
        if let Err(e) = std::fs::write(&path, content) {
            log::warn!("No se pudo escribir {:?}: {}", path, e);
        }
    }

    fn free_name(dir: &Path, stem: &str, ext: &str) -> PathBuf {
        let first = dir.join(format!("{stem}.{ext}"));
        if !first.exists() {
            return first;
        }
        (2..).map(|i| dir.join(format!("{stem} ({i}).{ext}"))).find(|p| !p.exists()).unwrap_or(first)
    }

    /// `index.md` de la raíz y de la carpeta de `relative_path` (para devolverlos con una nota)
    pub fn indexes_for(vault: &Path, relative_path: &str) -> (Option<String>, Option<String>) {
        let read = |rel: &str| {
            let p = if rel.is_empty() { vault.join(INDEX_FILE) } else { vault.join(rel).join(INDEX_FILE) };
            std::fs::read_to_string(p).ok()
        };
        let folder = parent_of(relative_path);
        let root = read("");
        let own = if folder.is_empty() { None } else { read(folder) };
        (root, own)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use compact_str::CompactString;

    fn meta(id: u32, path: &str) -> NoteMeta {
        NoteMeta {
            id,
            path: CompactString::new(path),
            title: CompactString::new(""),
            tags: Vec::new(),
            mtime_nanos: 0,
            size_bytes: 0,
            created_nanos: None,
            last_opened_nanos: None,
            is_open: false,
            tab_order: None,
            is_active_tab: false,
            view_mode: None,
            sync_state: Default::default(),
            is_drawio: false,
        }
    }

    #[test]
    fn genera_indices_conserva_el_propio_como_deprecado_y_no_reescribe_sin_cambios() {
        let dir = tempfile::tempdir().unwrap();
        let v = dir.path();
        std::fs::create_dir_all(v.join("proyecto/sub")).unwrap();
        std::fs::write(v.join("proyecto/plan.md"), "---\ntype: note\ndescription: El plan\n---\n# Plan\n").unwrap();
        std::fs::write(v.join("proyecto/libre.md"), "# Libre\n").unwrap();
        std::fs::write(v.join("proyecto/Glosario BCP.docx"), b"x").unwrap();
        std::fs::write(v.join("proyecto/sub/a.md"), "# A\n").unwrap();
        std::fs::write(v.join("proyecto/index.md"), "# Mi índice a mano\n").unwrap();
        let notes = Arc::new(DashMap::new());
        for (i, p) in ["proyecto/plan.md", "proyecto/libre.md", "proyecto/Glosario BCP.docx", "proyecto/sub/a.md", "proyecto/index.md"].iter().enumerate() {
            notes.insert(i as u32, meta(i as u32, p));
        }
        let ix = OkfIndexer { vault: v.to_path_buf(), notes, pending: Mutex::new(HashSet::new()), scheduled: AtomicBool::new(false) };
        ix.regenerate_all();

        let proyecto = std::fs::read_to_string(v.join("proyecto/index.md")).unwrap();
        assert!(is_generated(&proyecto));
        assert!(proyecto.contains("* [sub](sub/) - 1 archivo"), "{proyecto}");
        assert!(proyecto.contains("# Conceptos\n\n* [Plan](plan.md) - El plan"), "{proyecto}");
        assert!(proyecto.contains("* [Glosario BCP.docx](Glosario%20BCP.docx)"));
        assert!(proyecto.contains("* [Libre](libre.md)"));
        assert_eq!(std::fs::read_to_string(v.join("proyecto/index.deprecado.md")).unwrap(), "# Mi índice a mano\n");
        let root = std::fs::read_to_string(v.join("index.md")).unwrap();
        assert!(root.starts_with("---\nokf_version: \"0.3\"\n"));
        assert!(root.contains("* [proyecto](proyecto/) - 1 carpeta, 3 archivos"), "{root}");

        // Sin cambios: no se reescribe (misma fecha de modificación)
        let before = std::fs::metadata(v.join("proyecto/index.md")).unwrap().modified().unwrap();
        std::thread::sleep(Duration::from_millis(20));
        ix.regenerate_all();
        assert_eq!(std::fs::metadata(v.join("proyecto/index.md")).unwrap().modified().unwrap(), before);

        let (r, own) = OkfIndexer::indexes_for(v, "proyecto/plan.md");
        assert!(r.unwrap().contains("okf_version"));
        assert!(own.unwrap().contains("Plan"));
    }
}
