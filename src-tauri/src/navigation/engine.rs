use compact_str::CompactString;
use dashmap::DashMap;
use nucleo::{Config, Nucleo, Utf32String};
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct OpenTabDto {
    pub path: String,
    pub view_mode: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceOpenTabsState {
    pub open_tabs: Vec<OpenTabDto>,
    pub active_tab: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct VaultUiState {
    pub sidebar_width: Option<u32>,
    pub expanded_folders: Vec<String>,
}

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
    pub sidebar_width: Arc<AtomicU32>,
    pub expanded_folders: Arc<DashMap<CompactString, bool>>,
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

        let mut initial_sidebar_width = 240u32;
        let expanded_folders = DashMap::new();

        if let Some(parent) = cache_path.parent() {
            let workspace_file = parent.join("workspace.json");
            if workspace_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&workspace_file) {
                    if let Ok(ui_state) = serde_json::from_str::<VaultUiState>(&content) {
                        if let Some(w) = ui_state.sidebar_width {
                            initial_sidebar_width = w;
                        }
                        for folder in ui_state.expanded_folders {
                            expanded_folders.insert(CompactString::new(folder), true);
                        }
                    }
                }
            }
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
            sidebar_width: Arc::new(AtomicU32::new(initial_sidebar_width)),
            expanded_folders: Arc::new(expanded_folders),
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

            let created_nanos: Option<u128> = meta
                .created()
                .ok()
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos());

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

                let (
                    is_unchanged,
                    prev_last_opened,
                    prev_is_open,
                    prev_tab_order,
                    prev_is_active,
                    prev_view_mode,
                ) = if let Some(existing_meta) = self.notes.get(&existing_id) {
                    (
                        existing_meta.mtime_nanos == mtime_nanos
                            && existing_meta.size_bytes == size_bytes,
                        existing_meta.last_opened_nanos,
                        existing_meta.is_open,
                        existing_meta.tab_order,
                        existing_meta.is_active_tab,
                        existing_meta.view_mode.clone(),
                    )
                } else {
                    (false, None, false, None, false, None)
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
                        created_nanos,
                        last_opened_nanos: prev_last_opened,
                        is_open: prev_is_open,
                        tab_order: prev_tab_order,
                        is_active_tab: prev_is_active,
                        view_mode: prev_view_mode,
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
                    created_nanos,
                    last_opened_nanos: None,
                    is_open: false,
                    tab_order: None,
                    is_active_tab: false,
                    view_mode: Some(CompactString::new("reading")),
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
        self.save_workspace_ui_disk();
        self.dirty_flag.store(false, Ordering::Release);
        Ok(())
    }

    /// Records that a note was opened (e.g. in an active tab or preview).
    pub fn record_opened(&self, relative_path: &str) {
        let compact_path = CompactString::new(relative_path);
        if let Some(id_ref) = self.path_index.get(&compact_path) {
            let id = *id_ref.value();
            drop(id_ref);
            if let Some(mut meta_entry) = self.notes.get_mut(&id) {
                let now_nanos = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .ok()
                    .map(|d| d.as_nanos())
                    .unwrap_or(0);
                meta_entry.last_opened_nanos = Some(now_nanos);
                self.mark_dirty();
            }
        }
    }

    /// Returns the most recently opened notes (up to limit), ordered by last_opened_nanos descending.
    pub fn get_recent_notes(&self, limit: usize) -> Vec<NoteMeta> {
        let mut recent: Vec<NoteMeta> = self
            .notes
            .iter()
            .filter_map(|item| {
                if item.value().last_opened_nanos.is_some() {
                    Some(item.value().clone())
                } else {
                    None
                }
            })
            .collect();

        recent.sort_by(|a, b| b.last_opened_nanos.cmp(&a.last_opened_nanos));
        recent.into_iter().take(limit).collect()
    }

    /// Saves the open tabs state, tab order, active tab, and view modes in the DashMap entries.
    pub fn save_open_tabs_state(&self, tabs: &[OpenTabDto], active_path: Option<&str>) {
        let mut open_map = std::collections::HashMap::new();
        for (idx, tab) in tabs.iter().enumerate() {
            open_map.insert(tab.path.as_str(), (idx as u32, tab.view_mode.as_deref()));
        }

        for mut item in self.notes.iter_mut() {
            let note = item.value_mut();
            let path_str = note.path.as_str();
            if let Some(&(order, mode_opt)) = open_map.get(path_str) {
                note.is_open = true;
                note.tab_order = Some(order);
                note.is_active_tab = active_path == Some(path_str);
                let effective_mode = mode_opt.unwrap_or("reading");
                note.view_mode = Some(CompactString::new(effective_mode));
            } else {
                note.is_open = false;
                note.tab_order = None;
                note.is_active_tab = false;
            }
        }
        self.mark_dirty();
    }

    /// Retrieves the restored open tabs state from the DashMap entries.
    pub fn get_open_tabs_state(&self) -> WorkspaceOpenTabsState {
        let mut open_notes: Vec<NoteMeta> = self
            .notes
            .iter()
            .filter(|item| item.value().is_open)
            .map(|item| item.value().clone())
            .collect();

        // Sort by tab_order
        open_notes.sort_by_key(|n| n.tab_order.unwrap_or(u32::MAX));

        let mut active_tab = None;
        let mut open_tabs = Vec::with_capacity(open_notes.len());

        for note in open_notes {
            if note.is_active_tab {
                active_tab = Some(note.path.to_string());
            }
            let mode = note
                .view_mode
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "reading".to_string());
            open_tabs.push(OpenTabDto {
                path: note.path.to_string(),
                view_mode: Some(mode),
            });
        }

        if active_tab.is_none() && !open_tabs.is_empty() {
            active_tab = Some(open_tabs[0].path.clone());
        }

        WorkspaceOpenTabsState {
            open_tabs,
            active_tab,
        }
    }

    /// Sets view mode for a specific note.
    pub fn set_note_view_mode(&self, relative_path: &str, view_mode: &str) {
        let compact_path = CompactString::new(relative_path);
        if let Some(id_ref) = self.path_index.get(&compact_path) {
            let id = *id_ref.value();
            drop(id_ref);
            if let Some(mut meta_entry) = self.notes.get_mut(&id) {
                meta_entry.view_mode = Some(CompactString::new(view_mode));
                self.mark_dirty();
            }
        }
    }

    /// Saves workspace UI state (sidebar width, expanded folders) to .synapse/workspace.json on disk.
    pub fn save_workspace_ui_disk(&self) {
        if let Some(parent) = self.cache_path.parent() {
            let _ = std::fs::create_dir_all(parent);
            let workspace_file = parent.join("workspace.json");
            let ui_state = self.get_vault_ui_state();
            if let Ok(json) = serde_json::to_string_pretty(&ui_state) {
                let _ = std::fs::write(&workspace_file, json);
            }
        }
    }

    /// Updates the vault UI state in memory and persists it to .synapse/workspace.json.
    pub fn save_vault_ui_state(
        &self,
        sidebar_width: Option<u32>,
        expanded_folders: Option<Vec<String>>,
    ) {
        if let Some(width) = sidebar_width {
            self.sidebar_width.store(width, Ordering::SeqCst);
        }
        if let Some(folders) = expanded_folders {
            self.expanded_folders.clear();
            for folder in folders {
                self.expanded_folders.insert(CompactString::new(folder), true);
            }
        }
        self.save_workspace_ui_disk();
        self.mark_dirty();
    }

    /// Retrieves the current vault UI state from DashMap and atomic state.
    pub fn get_vault_ui_state(&self) -> VaultUiState {
        let width = self.sidebar_width.load(Ordering::SeqCst);
        let mut folders: Vec<String> = self
            .expanded_folders
            .iter()
            .filter(|item| *item.value())
            .map(|item| item.key().to_string())
            .collect();
        folders.sort();
        VaultUiState {
            sidebar_width: if width > 0 { Some(width) } else { None },
            expanded_folders: folders,
        }
    }

    /// Interactive fuzzy search with strict prioritization:
    /// 1. Files where the file name matches or contains the query come first.
    /// 2. Within each group, files opened recently (last_opened_nanos) come first.
    /// 3. Alphabetical order by file name (case-insensitive) for non-recent / equal recency.
    ///
    /// Returns (matches, total_files_count, matched_files_count)
    pub fn search_with_stats(&self, query: &str, limit: usize) -> (Vec<NoteMeta>, usize, usize) {
        let total_files = self.notes.len();
        let trimmed_query = query.trim();
        if trimmed_query.is_empty() {
            let mut all_notes: Vec<NoteMeta> =
                self.notes.iter().map(|item| item.value().clone()).collect();
            all_notes.sort_by(|a, b| {
                match (a.last_opened_nanos, b.last_opened_nanos) {
                    (Some(ta), Some(tb)) if ta != tb => return tb.cmp(&ta),
                    (Some(_), None) => return std::cmp::Ordering::Less,
                    (None, Some(_)) => return std::cmp::Ordering::Greater,
                    _ => {}
                }
                let name_a = a.path.rsplit('/').next().unwrap_or(a.path.as_str());
                let name_b = b.path.rsplit('/').next().unwrap_or(b.path.as_str());
                name_a.to_lowercase().cmp(&name_b.to_lowercase())
            });
            let matches = all_notes.into_iter().take(limit).collect();
            return (matches, total_files, total_files);
        }

        let mut nucleo = match self.nucleo.lock() {
            Ok(n) => n,
            Err(_) => return (Vec::new(), total_files, 0),
        };

        nucleo.pattern.reparse(
            0,
            trimmed_query,
            nucleo::pattern::CaseMatching::Ignore,
            nucleo::pattern::Normalization::Smart,
            false,
        );

        nucleo.tick(10);
        let snapshot = nucleo.snapshot();
        let matched_count = snapshot.matched_item_count() as usize;
        let query_lower = trimmed_query.to_lowercase();

        let mut candidates = Vec::new();
        // Take up to 1000 matched items and preserve their original nucleo rank
        for (nucleo_rank, item) in snapshot.matched_items(..).take(1000).enumerate() {
            let id = *item.data;
            if let Some(meta) = self.notes.get(&id) {
                candidates.push((meta.clone(), nucleo_rank));
            }
        }
        drop(nucleo);

        candidates.sort_by(|(a, rank_a), (b, rank_b)| {
            let name_a = a.path.rsplit('/').next().unwrap_or(a.path.as_str());
            let name_b = b.path.rsplit('/').next().unwrap_or(b.path.as_str());
            let name_a_lower = name_a.to_lowercase();
            let name_b_lower = name_b.to_lowercase();

            let stem_a_lower = match name_a_lower.rsplit_once('.') {
                Some((stem, _)) => stem,
                None => name_a_lower.as_str(),
            };
            let stem_b_lower = match name_b_lower.rsplit_once('.') {
                Some((stem, _)) => stem,
                None => name_b_lower.as_str(),
            };

            let title_a_lower = a.title.to_lowercase();
            let title_b_lower = b.title.to_lowercase();

            // Compute match tier:
            // 0: Exact filename/stem match
            // 1: Filename contains query
            // 2: Title contains query
            // 3: Folder or fuzzy match
            let tier = |name_lower: &str, stem_lower: &str, title_lower: &str| -> u8 {
                if stem_lower == query_lower {
                    0
                } else if name_lower.contains(&query_lower) {
                    1
                } else if title_lower.contains(&query_lower) {
                    2
                } else {
                    3
                }
            };

            let tier_a = tier(&name_a_lower, stem_a_lower, &title_a_lower);
            let tier_b = tier(&name_b_lower, stem_b_lower, &title_b_lower);

            let is_filename_a = tier_a <= 1;
            let is_filename_b = tier_b <= 1;

            // Rule 1: Prioritize matches by file name first
            if is_filename_a != is_filename_b {
                return if is_filename_a {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                };
            }

            // Rule 2: If both match by file name (or within same group), recently opened files first
            match (a.last_opened_nanos, b.last_opened_nanos) {
                (Some(ta), Some(tb)) if ta != tb => return tb.cmp(&ta),
                (Some(_), None) => return std::cmp::Ordering::Less,
                (None, Some(_)) => return std::cmp::Ordering::Greater,
                _ => {}
            }

            // If neither was opened (or opened at the exact same time):
            if tier_a != tier_b {
                return tier_a.cmp(&tier_b);
            }

            // Rule 3: For filename matches, alphabetical order by file name
            if is_filename_a {
                let cmp_name = name_a_lower.cmp(&name_b_lower);
                if cmp_name != std::cmp::Ordering::Equal {
                    return cmp_name;
                }
            }

            // For other tiers, respect nucleo score ranking
            rank_a.cmp(rank_b)
        });

        let matches = candidates.into_iter().map(|(meta, _)| meta).take(limit).collect();
        (matches, total_files, matched_count)
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<NoteMeta> {
        self.search_with_stats(query, limit).0
    }

    /// Requests background threads to terminate.
    pub fn shutdown(&self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        let _ = self.save_cache();
    }

    /// Forces reloading metadata into DashMap for given paths (files or directories).
    /// If a path is a directory (or empty string/root), reloads all files in that directory
    /// and its subdirectories, removes deleted files, and updates the nucleo search index.
    pub fn reload_paths(&self, paths: &[String]) -> Result<Vec<String>, String> {
        let mut reloaded_files = std::collections::BTreeSet::new();

        let target_paths: Vec<String> = if paths.is_empty() {
            vec![String::new()]
        } else {
            paths.to_vec()
        };

        for raw_path in target_paths {
            let clean_rel = raw_path
                .trim_matches('/')
                .replace('\\', "/");
            let full_path = if clean_rel.is_empty() || clean_rel == "." {
                self.vault_path.clone()
            } else {
                self.vault_path.join(&clean_rel)
            };

            if !full_path.exists() {
                // If the path does not exist on disk, remove it and any children from DashMap
                let mut deleted_ids = Vec::new();
                for item in self.notes.iter() {
                    let note_path = item.value().path.as_str();
                    let is_match = if clean_rel.is_empty() || clean_rel == "." {
                        true
                    } else {
                        note_path == clean_rel || note_path.starts_with(&format!("{}/", clean_rel))
                    };
                    if is_match {
                        deleted_ids.push(*item.key());
                    }
                }
                for id in deleted_ids {
                    if let Some((_, removed)) = self.notes.remove(&id) {
                        self.path_index.remove(&removed.path);
                    }
                }
                continue;
            }

            if full_path.is_dir() {
                let mut files_in_dir = Vec::new();
                collect_supported_files_recursive(&full_path, &self.file_types, &mut files_in_dir);

                let mut seen_in_dir = HashSet::new();
                for p in files_in_dir {
                    let rel_path_str = match p.strip_prefix(&self.vault_path) {
                        Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
                        Err(_) => continue,
                    };

                    eprintln!("[D1] before reload_file_into_dashmap for {:?}", rel_path_str);
                    self.reload_file_into_dashmap(&p, &rel_path_str);
                    eprintln!("[D2] after reload_file_into_dashmap for {:?}", rel_path_str);
                    seen_in_dir.insert(rel_path_str.clone());
                    reloaded_files.insert(rel_path_str);
                }

                // Remove deleted files within this directory scope
                eprintln!("[D3] checking deletions");
                let mut deleted_ids = Vec::new();
                for item in self.notes.iter() {
                    let note_path = item.value().path.as_str();
                    let in_scope = if clean_rel.is_empty() || clean_rel == "." {
                        true
                    } else {
                        note_path == clean_rel || note_path.starts_with(&format!("{}/", clean_rel))
                    };
                    if in_scope && !seen_in_dir.contains(note_path) {
                        deleted_ids.push(*item.key());
                    }
                }
                eprintln!("[D4] removing {} deleted notes", deleted_ids.len());
                for id in deleted_ids {
                    if let Some((_, removed)) = self.notes.remove(&id) {
                        self.path_index.remove(&removed.path);
                    }
                }
                eprintln!("[D5] removed deleted notes");
            } else if full_path.is_file() {
                let file_name = match full_path.file_name().and_then(|s| s.to_str()) {
                    Some(name) => name,
                    None => continue,
                };
                if !file_name.starts_with('.') && self.file_types.is_supported_file(file_name) {
                    let rel_path_str = match full_path.strip_prefix(&self.vault_path) {
                        Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
                        Err(_) => clean_rel.clone(),
                    };
                    self.reload_file_into_dashmap(&full_path, &rel_path_str);
                    reloaded_files.insert(rel_path_str);
                }
            }
        }

        if let Ok(mut nucleo_lock) = self.nucleo.lock() {
            nucleo_lock.restart(false);
            populate_nucleo_from_dashmap(&self.notes, &mut nucleo_lock);
        }

        self.mark_dirty();
        Ok(reloaded_files.into_iter().collect())
    }

    /// Internal helper to force reload a single file's metadata from disk into DashMap.
    fn reload_file_into_dashmap(&self, path: &std::path::Path, rel_path_str: &str) {
        let meta = match path.metadata() {
            Ok(m) => m,
            Err(_) => return,
        };

        let size_bytes = meta.len();
        let mtime_nanos: u128 = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
            .unwrap_or(0);

        let created_nanos: Option<u128> = meta
            .created()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos());

        let file_stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled");

        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled");

        let is_markdown = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"))
            .unwrap_or(false);

        let title = if is_markdown {
            let content = std::fs::read_to_string(path).unwrap_or_default();
            extract_title_from_markdown(&content, file_stem)
        } else {
            file_name.to_string()
        };

        let compact_title = CompactString::new(&title);
        let compact_path = CompactString::new(rel_path_str);

        let (note_id, note_meta) = if let Some(id_ref) = self.path_index.get(&compact_path) {
            let existing_id = *id_ref.value();
            drop(id_ref);

            let (prev_last_opened, prev_is_open, prev_tab_order, prev_is_active, prev_view_mode) = {
                let meta_opt = self.notes.get(&existing_id);
                let vals = if let Some(ref existing_meta) = meta_opt {
                    (
                        existing_meta.last_opened_nanos,
                        existing_meta.is_open,
                        existing_meta.tab_order,
                        existing_meta.is_active_tab,
                        existing_meta.view_mode.clone(),
                    )
                } else {
                    (None, false, None, false, None)
                };
                drop(meta_opt);
                vals
            };

            (
                existing_id,
                NoteMeta {
                    id: existing_id,
                    path: compact_path.clone(),
                    title: compact_title.clone(),
                    mtime_nanos,
                    size_bytes,
                    created_nanos,
                    last_opened_nanos: prev_last_opened,
                    is_open: prev_is_open,
                    tab_order: prev_tab_order,
                    is_active_tab: prev_is_active,
                    view_mode: prev_view_mode,
                },
            )
        } else {
            let new_id = self.next_id.fetch_add(1, Ordering::Relaxed);
            self.path_index.insert(compact_path.clone(), new_id);
            (
                new_id,
                NoteMeta {
                    id: new_id,
                    path: compact_path.clone(),
                    title: compact_title.clone(),
                    mtime_nanos,
                    size_bytes,
                    created_nanos,
                    last_opened_nanos: None,
                    is_open: false,
                    tab_order: None,
                    is_active_tab: false,
                    view_mode: Some(CompactString::new("reading")),
                },
            )
        };

        self.notes.insert(note_id, note_meta);
    }
}

fn collect_supported_files_recursive(
    dir: &std::path::Path,
    file_types: &SupportedFileTypes,
    out: &mut Vec<PathBuf>,
) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = match path.file_name().and_then(|s| s.to_str()) {
                Some(name) => name,
                None => continue,
            };
            if file_name.starts_with('.') {
                continue;
            }
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    collect_supported_files_recursive(&path, file_types, out);
                } else if ft.is_file() && file_types.is_supported_file(file_name) {
                    out.push(path);
                }
            }
        }
    }
}

