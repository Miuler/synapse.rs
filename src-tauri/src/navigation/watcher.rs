use compact_str::CompactString;
use dashmap::DashMap;
use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, DebouncedEvent, Debouncer};
use nucleo::{Nucleo, Utf32String};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use super::engine::extract_title_from_markdown;
use super::matcher::populate_nucleo_from_dashmap;
use super::model::{NoteId, NoteMeta};
use crate::domain::events::vault_events::{VaultChange, VaultChangeObserver};
use crate::domain::models::file_types::SupportedFileTypes;
use std::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultFsChangeEvent {
    pub paths: Vec<String>,
    pub deleted: Vec<String>,
}

pub type ChangeCallback = Arc<dyn Fn(VaultFsChangeEvent) + Send + Sync>;

#[derive(Debug)]
pub struct ParsedFileMeta {
    pub compact_path: CompactString,
    pub compact_title: CompactString,
    pub mtime_nanos: u128,
    pub size_bytes: u64,
    pub created_nanos: Option<u128>,
}

use crate::domain::models::config::IgnoredConfig;

pub fn default_ignored_config() -> &'static IgnoredConfig {
    static CONFIG: std::sync::OnceLock<IgnoredConfig> = std::sync::OnceLock::new();
    CONFIG.get_or_init(|| crate::domain::models::config::AppConfig::default().ignored)
}

/// Determines if a directory or file name should be completely ignored (build artifacts, dependencies, etc.)
pub fn is_ignored_dir_or_file(name: &str) -> bool {
    default_ignored_config().is_ignored_dir_or_file(name)
}

/// Determines if a path component or name represents a hidden or temporary file that should be ignored.
pub fn should_ignore_path(rel_path: &Path) -> bool {
    default_ignored_config().should_ignore_path(rel_path)
}

/// Processes a debounced batch of filesystem events using Rayon for concurrent parsing.
#[allow(clippy::too_many_arguments)]
pub fn process_events_batch(
    vault_path: &Path,
    file_types: &SupportedFileTypes,
    notes: &Arc<DashMap<NoteId, NoteMeta>>,
    path_index: &Arc<DashMap<CompactString, NoteId>>,
    nucleo: &Arc<Mutex<Nucleo<NoteId>>>,
    dirty_flag: &Arc<AtomicBool>,
    last_mutation: &Arc<Mutex<Instant>>,
    next_id: &Arc<AtomicU32>,
    on_change: &Arc<Mutex<Option<ChangeCallback>>>,
    observers: &Arc<RwLock<Vec<Arc<dyn VaultChangeObserver>>>>,
    events: Vec<DebouncedEvent>,
) {
    if events.is_empty() {
        return;
    }

    let mut existing_paths = HashSet::new();
    let mut deleted_paths = HashSet::new();

    for event in events {
        let abs_path = &event.path;
        let Ok(rel_path) = abs_path.strip_prefix(vault_path) else {
            continue;
        };

        if should_ignore_path(rel_path) {
            continue;
        }

        if abs_path.exists() {
            if abs_path.is_dir() {
                // If a directory was created/modified, only shallow walk non-ignored subfolders (max depth 3)
                // rather than traversing the entire vault.
                for entry_res in jwalk::WalkDir::new(abs_path)
                    .max_depth(3)
                    .skip_hidden(true)
                    .process_read_dir(|depth, _path, _state, children| {
                        if depth.is_none() {
                            return;
                        }
                        children.retain(|entry_res| {
                            entry_res
                                .as_ref()
                                .map(|e| {
                                    let name = e.file_name.to_string_lossy();
                                    !is_ignored_dir_or_file(&name)
                                })
                                .unwrap_or(false)
                        });
                    })
                {
                    if let Ok(entry) = entry_res {
                        if entry.file_type.is_file() {
                            let entry_path = entry.path();
                            if let Ok(sub_rel) = entry_path.strip_prefix(vault_path) {
                                if !should_ignore_path(sub_rel) {
                                    if let Some(name) =
                                        entry_path.file_name().and_then(|s| s.to_str())
                                    {
                                        if file_types.is_supported_file(name) {
                                            existing_paths.insert(entry_path);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else if abs_path.is_file() {
                if let Some(file_name) = abs_path.file_name().and_then(|n| n.to_str()) {
                    if file_types.is_supported_file(file_name) {
                        existing_paths.insert(abs_path.clone());
                    }
                }
            }
        } else {
            // Path does not exist on disk: either a deleted file or a deleted directory
            let rel_str = rel_path.to_string_lossy().replace('\\', "/");
            let compact_del = CompactString::new(&rel_str);
            // Only consider it a deleted path if it was actually tracked in path_index
            // or is a directory prefix of tracked notes.
            if path_index.contains_key(&compact_del) {
                deleted_paths.insert(rel_str);
            } else {
                let dir_prefix = format!("{}/", rel_str);
                let is_tracked_dir = notes.iter().any(|item| item.value().path.starts_with(&dir_prefix));
                if is_tracked_dir {
                    deleted_paths.insert(rel_str);
                }
            }
        }
    }

    let existing_vec: Vec<PathBuf> = existing_paths.into_iter().collect();

    // Concurrently parse metadata and markdown titles using rayon thread pool
    let parsed_items: Vec<ParsedFileMeta> = existing_vec
        .into_par_iter()
        .filter_map(|abs_path| {
            let meta = std::fs::metadata(&abs_path).ok()?;
            if !meta.is_file() {
                return None;
            }

            let rel_path_buf = abs_path.strip_prefix(vault_path).ok()?;
            let rel_str = rel_path_buf.to_string_lossy().replace('\\', "/");
            let compact_path = CompactString::new(&rel_str);

            let size_bytes = meta.len();
            let mtime_nanos = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let created_nanos = meta
                .created()
                .ok()
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos());

            // Avoid redundant disk read/parse if mtime and size are identical
            if let Some(id_ref) = path_index.get(&compact_path) {
                let id = *id_ref.value();
                drop(id_ref);
                if let Some(existing) = notes.get(&id) {
                    if existing.mtime_nanos == mtime_nanos && existing.size_bytes == size_bytes {
                        return None;
                    }
                }
            }

            let file_stem = abs_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled");

            let is_markdown = abs_path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"))
                .unwrap_or(false);

            let title = if is_markdown {
                let content = std::fs::read_to_string(&abs_path).unwrap_or_default();
                extract_title_from_markdown(&content, file_stem)
            } else {
                abs_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(file_stem)
                    .to_string()
            };

            Some(ParsedFileMeta {
                compact_path,
                compact_title: CompactString::new(title),
                mtime_nanos,
                size_bytes,
                created_nanos,
            })
        })
        .collect();

    let mut modified_or_added = false;
    let injector = nucleo.lock().ok().map(|n| n.injector());

    for item in &parsed_items {
        if let Some(id_ref) = path_index.get(&item.compact_path) {
            let existing_id = *id_ref.value();
            drop(id_ref);

            let (
                prev_last_opened,
                prev_is_open,
                prev_tab_order,
                prev_is_active,
                prev_view_mode,
            ) = if let Some(existing) = notes.get(&existing_id) {
                (
                    existing.last_opened_nanos,
                    existing.is_open,
                    existing.tab_order,
                    existing.is_active_tab,
                    existing.view_mode.clone(),
                )
            } else {
                (None, false, None, false, None)
            };

            let updated_meta = NoteMeta {
                id: existing_id,
                path: item.compact_path.clone(),
                title: item.compact_title.clone(),
                mtime_nanos: item.mtime_nanos,
                size_bytes: item.size_bytes,
                created_nanos: item.created_nanos,
                last_opened_nanos: prev_last_opened,
                is_open: prev_is_open,
                tab_order: prev_tab_order,
                is_active_tab: prev_is_active,
                view_mode: prev_view_mode,
            };

            notes.insert(existing_id, updated_meta);
            modified_or_added = true;

            if let Some(ref inj) = injector {
                let compact_title = item.compact_title.clone();
                let compact_path = item.compact_path.clone();
                inj.push(existing_id, move |_target, cols| {
                    if compact_title.as_str() == compact_path.as_str() {
                        cols[0] = Utf32String::from(compact_title.as_str());
                    } else {
                        let mut s =
                            String::with_capacity(compact_title.len() + 1 + compact_path.len());
                        s.push_str(compact_title.as_str());
                        s.push(' ');
                        s.push_str(compact_path.as_str());
                        cols[0] = Utf32String::from(s.as_str());
                    }
                });
            }
        } else {
            let new_id = next_id.fetch_add(1, Ordering::Relaxed);
            let new_meta = NoteMeta {
                id: new_id,
                path: item.compact_path.clone(),
                title: item.compact_title.clone(),
                mtime_nanos: item.mtime_nanos,
                size_bytes: item.size_bytes,
                created_nanos: item.created_nanos,
                last_opened_nanos: None,
                is_open: false,
                tab_order: None,
                is_active_tab: false,
                view_mode: Some(CompactString::new("reading")),
            };

            path_index.insert(item.compact_path.clone(), new_id);
            notes.insert(new_id, new_meta);
            modified_or_added = true;

            if let Some(ref inj) = injector {
                let compact_title = item.compact_title.clone();
                let compact_path = item.compact_path.clone();
                inj.push(new_id, move |_target, cols| {
                    if compact_title.as_str() == compact_path.as_str() {
                        cols[0] = Utf32String::from(compact_title.as_str());
                    } else {
                        let mut s =
                            String::with_capacity(compact_title.len() + 1 + compact_path.len());
                        s.push_str(compact_title.as_str());
                        s.push(' ');
                        s.push_str(compact_path.as_str());
                        cols[0] = Utf32String::from(s.as_str());
                    }
                });
            }
        }
    }

    let mut deleted_ids = Vec::new();
    for del_path in &deleted_paths {
        let compact_del = CompactString::new(del_path);
        if let Some(id_ref) = path_index.get(&compact_del) {
            deleted_ids.push(*id_ref.value());
        }

        // Also check if del_path is a directory prefix
        let dir_prefix = format!("{}/", del_path);
        for item in notes.iter() {
            if item.value().path.starts_with(&dir_prefix) {
                deleted_ids.push(*item.key());
            }
        }
    }

    if !deleted_ids.is_empty() {
        for id in &deleted_ids {
            if let Some((_, removed)) = notes.remove(id) {
                path_index.remove(&removed.path);
            }
        }

        if let Ok(mut nucleo_lock) = nucleo.lock() {
            nucleo_lock.restart(false);
            populate_nucleo_from_dashmap(notes, &mut nucleo_lock);
        }
    }

    if modified_or_added || !deleted_ids.is_empty() {
        if let Ok(mut t) = last_mutation.lock() {
            *t = Instant::now();
        }
        dirty_flag.store(true, Ordering::Release);

        let modified_paths: Vec<String> = parsed_items
            .into_iter()
            .map(|p| p.compact_path.to_string())
            .collect();
        let deleted_paths_list: Vec<String> = deleted_paths.into_iter().collect();

        if let Ok(guard) = on_change.lock() {
            if let Some(ref cb) = *guard {
                cb(VaultFsChangeEvent {
                    paths: modified_paths.clone(),
                    deleted: deleted_paths_list.clone(),
                });
            }
        }

        if let Ok(lock) = observers.read() {
            for obs in lock.iter() {
                if !modified_paths.is_empty() {
                    obs.on_vault_change(VaultChange::Upserted(modified_paths.clone()));
                }
                if !deleted_paths_list.is_empty() {
                    obs.on_vault_change(VaultChange::Removed(deleted_paths_list.clone()));
                }
            }
        }
    }
}

/// Filesystem watcher backed by native reactive OS notifications (notify v6+)
/// coalesced via notify-debouncer-mini, piping events to a worker thread and
/// rayon thread pool for concurrent metadata parsing and index updating.
pub struct VaultFsWatcher {
    debouncer: Option<Debouncer<RecommendedWatcher>>,
    worker_handle: Option<std::thread::JoinHandle<()>>,
}

impl VaultFsWatcher {
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        vault_path: PathBuf,
        debounce_duration: Duration,
        notes: Arc<DashMap<NoteId, NoteMeta>>,
        path_index: Arc<DashMap<CompactString, NoteId>>,
        nucleo: Arc<Mutex<Nucleo<NoteId>>>,
        dirty_flag: Arc<AtomicBool>,
        last_mutation: Arc<Mutex<Instant>>,
        next_id: Arc<AtomicU32>,
        file_types: SupportedFileTypes,
        shutdown_flag: Arc<AtomicBool>,
        on_change: Arc<Mutex<Option<ChangeCallback>>>,
        observers: Arc<RwLock<Vec<Arc<dyn VaultChangeObserver>>>>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (tx, rx) = mpsc::channel::<Vec<DebouncedEvent>>();

        let mut debouncer =
            new_debouncer(debounce_duration, move |res: DebounceEventResult| match res {
                Ok(events) => {
                    if !events.is_empty() {
                        let _ = tx.send(events);
                    }
                }
                Err(err) => {
                    eprintln!("[VaultFsWatcher] notify error: {:?}", err);
                }
            })?;

        debouncer
            .watcher()
            .watch(&vault_path, RecursiveMode::Recursive)?;

        let worker_vault_path = vault_path;
        let worker_shutdown = Arc::clone(&shutdown_flag);
        let worker_on_change = Arc::clone(&on_change);
        let worker_observers = Arc::clone(&observers);

        let worker_handle = std::thread::Builder::new()
            .name("vault-fs-parser".to_string())
            .spawn(move || loop {
                if worker_shutdown.load(Ordering::Relaxed) {
                    break;
                }

                match rx.recv_timeout(Duration::from_millis(200)) {
                    Ok(events) => {
                        process_events_batch(
                            &worker_vault_path,
                            &file_types,
                            &notes,
                            &path_index,
                            &nucleo,
                            &dirty_flag,
                            &last_mutation,
                            &next_id,
                            &worker_on_change,
                            &worker_observers,
                            events,
                        );
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        continue;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        break;
                    }
                }
            })?;

        Ok(Self {
            debouncer: Some(debouncer),
            worker_handle: Some(worker_handle),
        })
    }

    pub fn stop(&mut self) {
        self.debouncer.take();
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for VaultFsWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}
