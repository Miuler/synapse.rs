use compact_str::CompactString;
use dashmap::DashMap;
use nucleo::{Config, Nucleo, Utf32String};
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use super::matcher::populate_nucleo_from_dashmap;
use super::model::{NoteId, NoteMeta};
use super::storage;
use crate::domain::models::file_types::SupportedFileTypes;

/// Extracts the title from markdown content (first H1) or falls back to file stem.
pub fn extract_title_from_markdown(content: &str, file_stem: &str) -> String {
    let parser = Parser::new(content);
    let mut in_h1 = false;
    let mut title = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                ..
            }) => {
                in_h1 = true;
            }
            Event::End(TagEnd::Heading(HeadingLevel::H1)) => {
                break;
            }
            Event::Text(text) if in_h1 => {
                title.push_str(&text);
            }
            Event::Code(text) if in_h1 => {
                title.push_str(&text);
            }
            _ => {}
        }
    }

    let trimmed = title.trim();
    if !trimmed.is_empty() {
        trimmed.to_string()
    } else {
        file_stem.to_string()
    }
}

/// Statistics reported during reconciliation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReconciliationStats {
    pub scanned: usize,
    pub unchanged: usize,
    pub added: usize,
    pub modified: usize,
    pub deleted: usize,
}

/// Navigation and fast in-memory indexing engine with cold cache hydration.
pub struct NavigationEngine {
    pub vault_path: PathBuf,
    pub cache_path: PathBuf,
    pub notes: Arc<DashMap<NoteId, NoteMeta>>,
    pub path_index: Arc<DashMap<CompactString, NoteId>>,
    pub nucleo: Arc<Mutex<Nucleo<NoteId>>>,
    pub dirty_flag: Arc<AtomicBool>,
    pub next_id: Arc<AtomicU32>,
    pub last_mutation: Arc<Mutex<Instant>>,
    pub shutdown_flag: Arc<AtomicBool>,
    pub file_types: SupportedFileTypes,
}

impl NavigationEngine {
    /// Step 1: Immediate hydration.
    /// Checks if binary cache exists:
    /// - If exists: Deserializes into DashMap and populates nucleo immediately (<10ms).
    /// - If not exists: Initializes empty DashMap and ready for scanning.
    pub fn new(vault_path: PathBuf, cache_path: PathBuf) -> Self {
        Self::with_file_types(vault_path, cache_path, SupportedFileTypes::default())
    }

    pub fn with_file_types(
        vault_path: PathBuf,
        cache_path: PathBuf,
        file_types: SupportedFileTypes,
    ) -> Self {
        let mut nucleo_inst = Nucleo::new(Config::DEFAULT, Arc::new(|| {}), None, 1);

        let notes_map = if cache_path.exists() {
            match storage::load_cache_from_disk(&cache_path) {
                Ok(loaded) => {
                    populate_nucleo_from_dashmap(&loaded, &mut nucleo_inst);
                    loaded
                }
                Err(err) => {
                    eprintln!(
                        "[NavigationEngine] Warning: Failed to load cache from {:?}: {}",
                        cache_path, err
                    );
                    DashMap::new()
                }
            }
        } else {
            DashMap::new()
        };

        let path_index = DashMap::new();
        let mut max_id = 0u32;
        for item in notes_map.iter() {
            let id = *item.key();
            if id > max_id {
                max_id = id;
            }
            path_index.insert(item.value().path.clone(), id);
        }

        Self {
            vault_path,
            cache_path,
            notes: Arc::new(notes_map),
            path_index: Arc::new(path_index),
            nucleo: Arc::new(Mutex::new(nucleo_inst)),
            dirty_flag: Arc::new(AtomicBool::new(false)),
            next_id: Arc::new(AtomicU32::new(max_id + 1)),
            last_mutation: Arc::new(Mutex::new(Instant::now())),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            file_types,
        }
    }

    /// Creates and starts the engine with secondary thread reconciliation and persistence debounce.
    pub fn start(vault_path: PathBuf, cache_path: PathBuf) -> Arc<Self> {
        Self::start_with_file_types(vault_path, cache_path, SupportedFileTypes::default())
    }

    pub fn start_with_file_types(
        vault_path: PathBuf,
        cache_path: PathBuf,
        file_types: SupportedFileTypes,
    ) -> Arc<Self> {
        let engine = Arc::new(Self::with_file_types(vault_path, cache_path, file_types));
        engine.start_background_workers();
        engine
    }

    /// Spawns the secondary reconciliation thread and periodic debounce worker.
    pub fn start_background_workers(self: &Arc<Self>) {
        self.spawn_reconciliation();
        self.spawn_debounce_worker(Duration::from_secs(3));
    }

    /// Step 2: Background reconciliation runner using `jwalk::WalkDir`.
    pub fn spawn_reconciliation(self: &Arc<Self>) -> thread::JoinHandle<ReconciliationStats> {
        let engine = Arc::clone(self);
        thread::spawn(move || engine.reconcile_sync())
    }

    /// Synchronous reconciliation logic (callable from tests or background threads).
    pub fn reconcile_sync(&self) -> ReconciliationStats {
        let mut stats = ReconciliationStats::default();
        let mut seen_ids = HashSet::new();

        let injector = match self.nucleo.lock() {
            Ok(n) => n.injector(),
            Err(e) => {
                eprintln!("[NavigationEngine] Poisoned nucleo lock: {}", e);
                return stats;
            }
        };

        // Parallel walk using jwalk
        for entry_res in jwalk::WalkDir::new(&self.vault_path).skip_hidden(true) {
            let entry = match entry_res {
                Ok(e) => e,
                Err(_) => continue,
            };

            if !entry.file_type.is_file() {
                continue;
            }

            let path = entry.path();
            let file_name = match path.file_name().and_then(|s| s.to_str()) {
                Some(name) => name,
                None => continue,
            };

            // Skip hidden files/directories
            if file_name.starts_with('.') {
                continue;
            }

            // Check if file is supported (md, png, jpg, jpeg, webp, avif, excalidraw, etc.)
            if !self.file_types.is_supported_file(file_name) {
                continue;
            }

            stats.scanned += 1;

            let rel_path_str = match path.strip_prefix(&self.vault_path) {
                Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };
            let compact_path = CompactString::new(&rel_path_str);

            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let size_bytes = meta.len();
            let mtime_nanos: u128 = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos())
                .unwrap_or(0);

            let file_stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled");

            let is_markdown = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"))
                .unwrap_or(false);

            if let Some(id_ref) = self.path_index.get(&compact_path) {
                let existing_id = *id_ref.value();
                drop(id_ref);
                seen_ids.insert(existing_id);

                let is_unchanged = if let Some(existing_meta) = self.notes.get(&existing_id) {
                    existing_meta.mtime_nanos == mtime_nanos
                        && existing_meta.size_bytes == size_bytes
                } else {
                    false
                };

                if is_unchanged {
                    stats.unchanged += 1;
                } else {
                    let title = if is_markdown {
                        let content = std::fs::read_to_string(&path).unwrap_or_default();
                        extract_title_from_markdown(&content, file_stem)
                    } else {
                        file_name.to_string()
                    };
                    let compact_title = CompactString::new(&title);

                    let note_meta = NoteMeta {
                        id: existing_id,
                        path: compact_path.clone(),
                        title: compact_title.clone(),
                        mtime_nanos,
                        size_bytes,
                    };

                    self.notes.insert(existing_id, note_meta);
                    self.mark_dirty();
                    stats.modified += 1;

                    injector.push(existing_id, move |_target, cols| {
                        if compact_title.as_str() == compact_path.as_str() {
                            cols[0] = Utf32String::from(compact_title.as_str());
                        } else {
                            let mut s = String::with_capacity(compact_title.len() + 1 + compact_path.len());
                            s.push_str(compact_title.as_str());
                            s.push(' ');
                            s.push_str(compact_path.as_str());
                            cols[0] = Utf32String::from(s.as_str());
                        }
                    });
                }
            } else {
                // New file discovered
                let new_id = self.next_id.fetch_add(1, Ordering::Relaxed);
                seen_ids.insert(new_id);

                let title = if is_markdown {
                    let content = std::fs::read_to_string(&path).unwrap_or_default();
                    extract_title_from_markdown(&content, file_stem)
                } else {
                    file_name.to_string()
                };
                let compact_title = CompactString::new(&title);

                let note_meta = NoteMeta {
                    id: new_id,
                    path: compact_path.clone(),
                    title: compact_title.clone(),
                    mtime_nanos,
                    size_bytes,
                };

                self.path_index.insert(compact_path.clone(), new_id);
                self.notes.insert(new_id, note_meta);
                self.mark_dirty();
                stats.added += 1;

                injector.push(new_id, move |_target, cols| {
                    if compact_title.as_str() == compact_path.as_str() {
                        cols[0] = Utf32String::from(compact_title.as_str());
                    } else {
                        let mut s = String::with_capacity(compact_title.len() + 1 + compact_path.len());
                        s.push_str(compact_title.as_str());
                        s.push(' ');
                        s.push_str(compact_path.as_str());
                        cols[0] = Utf32String::from(s.as_str());
                    }
                });
            }
        }

        // Detect deletions: IDs in DashMap that were not encountered in filesystem walk
        let mut deleted_ids = Vec::new();
        for item in self.notes.iter() {
            let id = *item.key();
            if !seen_ids.contains(&id) {
                deleted_ids.push(id);
            }
        }

        if !deleted_ids.is_empty() {
            for id in &deleted_ids {
                if let Some((_, removed)) = self.notes.remove(id) {
                    self.path_index.remove(&removed.path);
                    self.mark_dirty();
                    stats.deleted += 1;
                }
            }

            // Re-populate nucleo to eliminate removed IDs
            if let Ok(mut nucleo_lock) = self.nucleo.lock() {
                nucleo_lock.restart(false);
                populate_nucleo_from_dashmap(&self.notes, &mut nucleo_lock);
            }
        }

        stats
    }

    /// Step 3: Debounce persistence worker (3 to 5 seconds of inactivity).
    pub fn spawn_debounce_worker(
        self: &Arc<Self>,
        debounce_duration: Duration,
    ) -> thread::JoinHandle<()> {
        let notes = Arc::clone(&self.notes);
        let cache_path = self.cache_path.clone();
        let dirty_flag = Arc::clone(&self.dirty_flag);
        let last_mutation = Arc::clone(&self.last_mutation);
        let shutdown_flag = Arc::clone(&self.shutdown_flag);

        thread::spawn(move || {
            while !shutdown_flag.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(500));

                if dirty_flag.load(Ordering::Acquire) {
                    let elapsed = last_mutation
                        .lock()
                        .map(|t| t.elapsed())
                        .unwrap_or(Duration::ZERO);

                    if elapsed >= debounce_duration {
                        if let Err(e) = storage::save_cache_to_disk(&cache_path, &notes) {
                            eprintln!("[NavigationEngine] Debounce save failed: {}", e);
                        } else {
                            dirty_flag.store(false, Ordering::Release);
                        }
                    }
                }
            }

            // Flush before thread exit if dirty
            if dirty_flag.load(Ordering::Acquire) {
                let _ = storage::save_cache_to_disk(&cache_path, &notes);
                dirty_flag.store(false, Ordering::Release);
            }
        })
    }

    /// Marks the cache as dirty and records timestamp for debounce timer.
    pub fn mark_dirty(&self) {
        if let Ok(mut t) = self.last_mutation.lock() {
            *t = Instant::now();
        }
        self.dirty_flag.store(true, Ordering::Release);
    }

    /// Explicit save hook for app shutdown or manual flush.
    pub fn save_cache(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        storage::save_cache_to_disk(&self.cache_path, &self.notes)?;
        self.dirty_flag.store(false, Ordering::Release);
        Ok(())
    }

    /// Quick search through nucleo interactive fuzzy matcher.
    pub fn search(&self, query: &str, limit: usize) -> Vec<NoteMeta> {
        let mut nucleo = match self.nucleo.lock() {
            Ok(n) => n,
            Err(_) => return Vec::new(),
        };

        nucleo.pattern.reparse(
            0,
            query,
            nucleo::pattern::CaseMatching::Ignore,
            nucleo::pattern::Normalization::Smart,
            false,
        );

        nucleo.tick(10);
        let snapshot = nucleo.snapshot();
        let mut results = Vec::new();

        for item in snapshot.matched_items(..).take(limit) {
            let id = *item.data;
            if let Some(meta) = self.notes.get(&id) {
                results.push(meta.clone());
            }
        }

        results
    }

    /// Requests background threads to terminate.
    pub fn shutdown(&self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        let _ = self.save_cache();
    }
}
