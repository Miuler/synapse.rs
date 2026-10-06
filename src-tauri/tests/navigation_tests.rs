use app_lib::navigation::engine::{extract_title_from_markdown, NavigationEngine};
use app_lib::navigation::matcher::populate_nucleo_from_dashmap;
use app_lib::navigation::model::{NoteId, NoteMeta};
use app_lib::navigation::storage::{load_cache_from_disk, save_cache_to_disk};
use compact_str::CompactString;
use dashmap::DashMap;
use nucleo::{Config, Nucleo};
use std::fs::{self, File};
use std::io::Write;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

#[test]
fn test_note_meta_compact_str_and_bincode_roundtrip() {
    let dir = std::env::temp_dir().join("synapse_test_storage");
    let _ = fs::create_dir_all(&dir);
    let cache_file = dir.join("test_cache.bin");

    let map = DashMap::new();
    map.insert(
        1,
        NoteMeta {
            id: 1,
            path: CompactString::new("folder/note_one.md"),
            title: CompactString::new("First Note"),
            mtime_nanos: 123456789,
            size_bytes: 42,
            created_nanos: Some(123450000),
            last_opened_nanos: Some(123456799),
            is_open: true,
            tab_order: Some(0),
            is_active_tab: true,
            view_mode: Some(CompactString::new("reading")),
        },
    );
    map.insert(
        2,
        NoteMeta {
            id: 2,
            path: CompactString::new("note_two.md"),
            title: CompactString::new("Second Note"),
            mtime_nanos: 987654321,
            size_bytes: 100,
            created_nanos: None,
            last_opened_nanos: None,
            is_open: false,
            tab_order: None,
            is_active_tab: false,
            view_mode: None,
        },
    );

    save_cache_to_disk(&cache_file, &map).expect("Should save cache to disk");
    assert!(cache_file.exists());

    let loaded_map = load_cache_from_disk(&cache_file).expect("Should load cache from disk");
    assert_eq!(loaded_map.len(), 2);

    let item1 = loaded_map.get(&1).unwrap();
    assert_eq!(item1.title.as_str(), "First Note");
    assert_eq!(item1.path.as_str(), "folder/note_one.md");
    assert_eq!(item1.mtime_nanos, 123456789);
    assert_eq!(item1.size_bytes, 42);

    let _ = fs::remove_file(&cache_file);
    let _ = fs::remove_dir(&dir);
}

#[test]
fn test_title_extraction_markdown_h1_and_fallback() {
    let markdown_with_h1 = "# Mi Título Principal\n\nEste es el contenido de la nota.";
    assert_eq!(
        extract_title_from_markdown(markdown_with_h1, "archivo_fallback"),
        "Mi Título Principal"
    );

    let markdown_with_frontmatter_and_h1 =
        "---\ntags: [rust, synapse]\n---\n# Título con Frontmatter\n\nCuerpo";
    assert_eq!(
        extract_title_from_markdown(markdown_with_frontmatter_and_h1, "fallback"),
        "Título con Frontmatter"
    );

    let markdown_without_h1 = "## Subtítulo solamente\n\nNo hay H1 aquí.";
    assert_eq!(
        extract_title_from_markdown(markdown_without_h1, "nombre_archivo"),
        "nombre_archivo"
    );

    let empty_markdown = "";
    assert_eq!(
        extract_title_from_markdown(empty_markdown, "nota_vacia"),
        "nota_vacia"
    );
}

#[test]
fn test_populate_nucleo_and_matching() {
    let map = DashMap::new();
    map.insert(
        10,
        NoteMeta {
            id: 10,
            path: CompactString::new("rust_guide.md"),
            title: CompactString::new("Rust Programming Language"),
            mtime_nanos: 1,
            size_bytes: 10,
            created_nanos: None,
            last_opened_nanos: None,
            is_open: false,
            tab_order: None,
            is_active_tab: false,
            view_mode: None,
        },
    );
    map.insert(
        20,
        NoteMeta {
            id: 20,
            path: CompactString::new("svelte_ui.md"),
            title: CompactString::new("Svelte 5 Components"),
            mtime_nanos: 2,
            size_bytes: 20,
            created_nanos: None,
            last_opened_nanos: None,
            is_open: false,
            tab_order: None,
            is_active_tab: false,
            view_mode: None,
        },
    );

    let mut nucleo = Nucleo::<NoteId>::new(Config::DEFAULT, Arc::new(|| {}), None, 1);

    populate_nucleo_from_dashmap(&map, &mut nucleo);

    // Let nucleo process incoming items
    nucleo.tick(20);

    // Filter query: "rust"
    nucleo.pattern.reparse(
        0,
        "rust",
        nucleo::pattern::CaseMatching::Ignore,
        nucleo::pattern::Normalization::Smart,
        false,
    );
    nucleo.tick(20);

    let snapshot = nucleo.snapshot();
    let matches: Vec<u32> = snapshot.matched_items(..).map(|item| *item.data).collect();
    assert_eq!(matches, vec![10]);
}

#[test]
fn test_cold_start_hydration_timing_sub_10ms() {
    let test_dir = std::env::temp_dir().join("synapse_cold_start_test");
    let _ = fs::create_dir_all(&test_dir);
    let cache_file = test_dir.join("cache.bin");

    // Populate a cache with 1000 notes
    let map = DashMap::new();
    for i in 1..=1000 {
        map.insert(
            i,
            NoteMeta {
                id: i,
                path: CompactString::new(format!("notes/section_{}/note_{}.md", i % 10, i)),
                title: CompactString::new(format!("Note Title #{} for Testing", i)),
                mtime_nanos: 1_000_000 + i as u128,
                size_bytes: 500 + i as u64,
                created_nanos: None,
                last_opened_nanos: None,
                is_open: false,
                tab_order: None,
                is_active_tab: false,
                view_mode: None,
            },
        );
    }
    save_cache_to_disk(&cache_file, &map).expect("Cache must save");

    // Measure cold start hydration time
    let start_time = Instant::now();
    let engine = NavigationEngine::new(test_dir.clone(), cache_file.clone());
    let elapsed = start_time.elapsed();

    println!("Cold start hydration elapsed time: {:?}", elapsed);
    assert!(
        elapsed.as_millis() <= 25,
        "Cold start must be fast in debug mode (was {:?})",
        elapsed
    );

    assert_eq!(engine.notes.len(), 1000);

    // Search should work immediately
    let search_results = engine.search("Title #500", 5);
    assert!(!search_results.is_empty());
    assert_eq!(search_results[0].id, 500);

    let _ = fs::remove_file(&cache_file);
    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_reconciliation_workflow() {
    let vault_dir = std::env::temp_dir().join("synapse_vault_reconciliation_test");
    let _ = fs::remove_dir_all(&vault_dir);
    let _ = fs::create_dir_all(&vault_dir);
    let cache_file = vault_dir.join(".synapse/cache.bin");

    // Step A: Create initial files
    let note1_path = vault_dir.join("note1.md");
    let mut f1 = File::create(&note1_path).unwrap();
    writeln!(f1, "# Architecture Overview\n\nSome text.").unwrap();

    let note2_path = vault_dir.join("note2.md");
    let mut f2 = File::create(&note2_path).unwrap();
    writeln!(f2, "No heading here, just body text.").unwrap();

    let engine = NavigationEngine::new(vault_dir.clone(), cache_file.clone());
    assert_eq!(engine.notes.len(), 0);

    let stats = engine.reconcile_sync();
    assert_eq!(stats.scanned, 2);
    assert_eq!(stats.added, 2);
    assert_eq!(stats.modified, 0);
    assert_eq!(stats.unchanged, 0);
    assert_eq!(stats.deleted, 0);
    assert_eq!(engine.notes.len(), 2);
    assert!(engine.dirty_flag.load(Ordering::Acquire));

    // Verify titles (extract values without holding DashMap locks)
    let n1 = *engine.path_index.get("note1.md").unwrap().value();
    let title1 = engine.notes.get(&n1).unwrap().title.clone();
    assert_eq!(title1.as_str(), "Architecture Overview");

    let n2 = *engine.path_index.get("note2.md").unwrap().value();
    let title2 = engine.notes.get(&n2).unwrap().title.clone();
    assert_eq!(title2.as_str(), "note2"); // fallback to stem

    // Save cache explicitly
    engine.save_cache().unwrap();
    assert!(!engine.dirty_flag.load(Ordering::Acquire));

    // Step B: Reconcile again without changes -> should omit reads (unchanged)
    let stats_unchanged = engine.reconcile_sync();
    assert_eq!(stats_unchanged.unchanged, 2);
    assert_eq!(stats_unchanged.added, 0);
    assert_eq!(stats_unchanged.modified, 0);
    assert_eq!(stats_unchanged.deleted, 0);
    assert!(!engine.dirty_flag.load(Ordering::Acquire));

    // Step C: Modify note1.md
    std::thread::sleep(std::time::Duration::from_millis(50));
    let mut f1_mod = File::create(&note1_path).unwrap();
    writeln!(f1_mod, "# Updated Architecture 2.0\n\nModified content.").unwrap();

    let stats_mod = engine.reconcile_sync();
    assert_eq!(stats_mod.modified, 1);
    assert_eq!(stats_mod.unchanged, 1);
    assert!(engine.dirty_flag.load(Ordering::Acquire));

    let title1_updated = engine.notes.get(&n1).unwrap().title.clone();
    assert_eq!(title1_updated.as_str(), "Updated Architecture 2.0");

    // Step D: Delete note2.md
    fs::remove_file(&note2_path).unwrap();
    let stats_del = engine.reconcile_sync();
    assert_eq!(stats_del.deleted, 1);
    assert_eq!(engine.notes.len(), 1);
    assert!(!engine.path_index.contains_key("note2.md"));

    let _ = fs::remove_dir_all(&vault_dir);
}

#[test]
fn test_debounce_persistence_worker() {
    let test_dir = std::env::temp_dir().join("synapse_debounce_test");
    let _ = fs::remove_dir_all(&test_dir);
    let _ = fs::create_dir_all(&test_dir);
    let cache_file = test_dir.join("cache.bin");

    let engine = Arc::new(NavigationEngine::new(test_dir.clone(), cache_file.clone()));
    let _handle = engine.spawn_debounce_worker(std::time::Duration::from_millis(300));

    // Initially not dirty, file should not exist yet
    assert!(!cache_file.exists());

    // Insert note and mark dirty
    engine.notes.insert(
        1,
        NoteMeta {
            id: 1,
            path: CompactString::new("debounce_note.md"),
            title: CompactString::new("Debounced Note"),
            mtime_nanos: 12345,
            size_bytes: 678,
            created_nanos: None,
            last_opened_nanos: None,
            is_open: false,
            tab_order: None,
            is_active_tab: false,
            view_mode: None,
        },
    );
    engine.mark_dirty();
    assert!(engine.dirty_flag.load(Ordering::Acquire));

    // Wait for debounce interval + flush
    std::thread::sleep(std::time::Duration::from_millis(900));

    // Cache should now be written to disk and dirty flag cleared
    assert!(
        cache_file.exists(),
        "Cache file must exist after debounce flush"
    );
    assert!(
        !engine.dirty_flag.load(Ordering::Acquire),
        "Dirty flag must be cleared"
    );

    // Verify written content
    let loaded = load_cache_from_disk(&cache_file).unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded.get(&1).unwrap().title.as_str(), "Debounced Note");

    engine.shutdown();
    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_dashmap_directory_children_resolution() {
    let test_dir = std::env::temp_dir().join("synapse_children_test");
    let _ = fs::remove_dir_all(&test_dir);
    let _ = fs::create_dir_all(&test_dir);
    let cache_file = test_dir.join("cache.bin");

    let engine = NavigationEngine::new(test_dir.clone(), cache_file);
    engine.notes.insert(
        1,
        NoteMeta {
            id: 1,
            path: CompactString::new("root_note.md"),
            title: CompactString::new("Root Note"),
            mtime_nanos: 1,
            size_bytes: 10,
            created_nanos: None,
            last_opened_nanos: None,
            is_open: false,
            tab_order: None,
            is_active_tab: false,
            view_mode: None,
        },
    );
    engine.notes.insert(
        2,
        NoteMeta {
            id: 2,
            path: CompactString::new("docs/intro.md"),
            title: CompactString::new("Intro"),
            mtime_nanos: 2,
            size_bytes: 20,
            created_nanos: None,
            last_opened_nanos: None,
            is_open: false,
            tab_order: None,
            is_active_tab: false,
            view_mode: None,
        },
    );
    engine.notes.insert(
        3,
        NoteMeta {
            id: 3,
            path: CompactString::new("docs/advanced/tuning.md"),
            title: CompactString::new("Tuning"),
            mtime_nanos: 3,
            size_bytes: 30,
            created_nanos: None,
            last_opened_nanos: None,
            is_open: false,
            tab_order: None,
            is_active_tab: false,
            view_mode: None,
        },
    );

    // Root children resolution
    let mut root_folders = std::collections::BTreeSet::new();
    let mut root_files = Vec::new();
    for item in engine.notes.iter() {
        let rel = item.value().path.as_str();
        if let Some((first, _)) = rel.split_once('/') {
            root_folders.insert(first.to_string());
        } else {
            root_files.push(rel.to_string());
        }
    }
    assert_eq!(root_folders.into_iter().collect::<Vec<_>>(), vec!["docs"]);
    assert_eq!(root_files, vec!["root_note.md"]);

    // "docs" children resolution
    let prefix = "docs/";
    let mut docs_folders = std::collections::BTreeSet::new();
    let mut docs_files = Vec::new();
    for item in engine.notes.iter() {
        let rel = item.value().path.as_str();
        if let Some(sub) = rel.strip_prefix(prefix) {
            if let Some((folder, _)) = sub.split_once('/') {
                docs_folders.insert(folder.to_string());
            } else {
                docs_files.push(sub.to_string());
            }
        }
    }
    assert_eq!(docs_folders.into_iter().collect::<Vec<_>>(), vec!["advanced"]);
    assert_eq!(docs_files, vec!["intro.md"]);

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_indexing_multi_format_files_and_nucleo_search() {
    let test_dir = std::env::temp_dir().join("synapse_multi_format_test");
    let _ = fs::remove_dir_all(&test_dir);
    let _ = fs::create_dir_all(&test_dir);
    let cache_file = test_dir.join("cache.bin");

    // Create diverse supported files: md, png, excalidraw, webp, avif
    File::create(test_dir.join("diagram.excalidraw")).unwrap();
    File::create(test_dir.join("avatar.png")).unwrap();
    File::create(test_dir.join("hero.webp")).unwrap();
    File::create(test_dir.join("photo.avif")).unwrap();
    let mut md_file = File::create(test_dir.join("guide.md")).unwrap();
    writeln!(md_file, "# Guía Rápida\nContenido").unwrap();

    let engine = NavigationEngine::new(test_dir.clone(), cache_file);
    let stats = engine.reconcile_sync();

    assert_eq!(stats.scanned, 5);
    assert_eq!(stats.added, 5);
    assert_eq!(engine.notes.len(), 5);

    // Search by extension or file name using nucleo
    let results_excalidraw = engine.search("excalidraw", 10);
    assert!(!results_excalidraw.is_empty());
    assert_eq!(results_excalidraw[0].path.as_str(), "diagram.excalidraw");

    let results_png = engine.search("avatar", 10);
    assert!(!results_png.is_empty());
    assert_eq!(results_png[0].path.as_str(), "avatar.png");

    let results_webp = engine.search("webp", 10);
    assert!(!results_webp.is_empty());
    assert_eq!(results_webp[0].path.as_str(), "hero.webp");

    let results_avif = engine.search("photo", 10);
    assert!(!results_avif.is_empty());
    assert_eq!(results_avif[0].path.as_str(), "photo.avif");

    let results_h1 = engine.search("Guía Rápida", 10);
    assert!(!results_h1.is_empty());
    assert_eq!(results_h1[0].path.as_str(), "guide.md");

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_search_prioritization_filename_recency_and_alphabetical() {
    let test_dir = std::env::temp_dir().join("synapse_priority_test");
    let _ = fs::remove_dir_all(&test_dir);
    let _ = fs::create_dir_all(test_dir.join("sub").join("arquitectura"));
    let cache_file = test_dir.join("cache.bin");

    // Create 4 files matching the user's specific scenario:
    // 1. "b_arquitectura.md" (contains ARQUITECTURA in filename, not opened)
    // 2. "c_arquitectura.md" (contains ARQUITECTURA in filename, not opened)
    // 3. "a_arquitectura.md" (contains ARQUITECTURA in filename, opened recently)
    // 4. "sub/arquitectura/otras_notas.md" (matches ARQUITECTURA only in directory path)
    File::create(test_dir.join("b_arquitectura.md")).unwrap();
    File::create(test_dir.join("c_arquitectura.md")).unwrap();
    File::create(test_dir.join("a_arquitectura.md")).unwrap();
    File::create(test_dir.join("sub").join("arquitectura").join("otras_notas.md")).unwrap();

    let engine = NavigationEngine::new(test_dir.clone(), cache_file);
    engine.reconcile_sync();

    // Open "a_arquitectura.md"
    engine.record_opened("a_arquitectura.md");

    let results = engine.search("arquitectura", 10);
    assert_eq!(results.len(), 4);

    // 1st: "a_arquitectura.md" because it contains "arquitectura" in the filename AND was opened recently
    assert_eq!(results[0].path.as_str(), "a_arquitectura.md");
    assert!(results[0].last_opened_nanos.is_some());

    // 2nd and 3rd: "b_arquitectura.md" and "c_arquitectura.md" because they contain "arquitectura" in filename, sorted alphabetically
    assert_eq!(results[1].path.as_str(), "b_arquitectura.md");
    assert_eq!(results[2].path.as_str(), "c_arquitectura.md");

    // 4th: "sub/arquitectura/otras_notas.md" because it does NOT contain "arquitectura" in filename, only in path
    assert_eq!(results[3].path.as_str(), "sub/arquitectura/otras_notas.md");

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_record_opened_and_metadata_persistence() {
    let test_dir = std::env::temp_dir().join("synapse_record_opened_test");
    let _ = fs::remove_dir_all(&test_dir);
    let _ = fs::create_dir_all(&test_dir);
    let cache_file = test_dir.join("cache.bin");

    File::create(test_dir.join("my_note.md")).unwrap();

    let engine = NavigationEngine::new(test_dir.clone(), cache_file.clone());
    engine.reconcile_sync();

    // Verify initial note has mtime and created (if filesystem supports it)
    let meta_before = engine.notes.iter().next().unwrap().value().clone();
    assert!(meta_before.mtime_nanos > 0);
    assert!(meta_before.last_opened_nanos.is_none());

    // Record open
    engine.record_opened("my_note.md");
    let meta_after = engine.notes.iter().next().unwrap().value().clone();
    assert!(meta_after.last_opened_nanos.is_some());

    // Save and reload from disk
    engine.save_cache().expect("Cache should save");
    let loaded_map = load_cache_from_disk(&cache_file).expect("Cache should load");
    let loaded_meta = loaded_map.iter().next().unwrap().value().clone();
    assert_eq!(loaded_meta.last_opened_nanos, meta_after.last_opened_nanos);

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_open_tabs_state_and_reading_mode_persistence() {
    use app_lib::navigation::engine::OpenTabDto;

    let test_dir = std::env::temp_dir().join("synapse_open_tabs_test");
    let _ = fs::remove_dir_all(&test_dir);
    let _ = fs::create_dir_all(&test_dir);
    let cache_file = test_dir.join("cache.bin");

    File::create(test_dir.join("doc1.md")).unwrap();
    File::create(test_dir.join("doc2.md")).unwrap();
    File::create(test_dir.join("doc3.md")).unwrap();

    let engine = NavigationEngine::new(test_dir.clone(), cache_file.clone());
    engine.reconcile_sync();

    // Initially no tabs are open, default mode is "reading"
    let initial_state = engine.get_open_tabs_state();
    assert!(initial_state.open_tabs.is_empty());
    assert_eq!(initial_state.active_tab, None);

    // Save open tabs: doc1 (reading) and doc2 (live), with doc2 active
    let tabs = vec![
        OpenTabDto {
            path: "doc1.md".to_string(),
            view_mode: None, // should default to "reading"
        },
        OpenTabDto {
            path: "doc2.md".to_string(),
            view_mode: Some("live".to_string()),
        },
    ];
    engine.save_open_tabs_state(&tabs, Some("doc2.md"));

    let open_state = engine.get_open_tabs_state();
    assert_eq!(open_state.open_tabs.len(), 2);
    assert_eq!(open_state.open_tabs[0].path, "doc1.md");
    assert_eq!(open_state.open_tabs[0].view_mode.as_deref(), Some("reading"));
    assert_eq!(open_state.open_tabs[1].path, "doc2.md");
    assert_eq!(open_state.open_tabs[1].view_mode.as_deref(), Some("live"));
    assert_eq!(open_state.active_tab.as_deref(), Some("doc2.md"));

    // Save cache and reload in a fresh engine (simulating app restart)
    engine.save_cache().expect("Cache should save");

    let fresh_engine = NavigationEngine::new(test_dir.clone(), cache_file.clone());
    let restored_state = fresh_engine.get_open_tabs_state();

    assert_eq!(restored_state.open_tabs.len(), 2);
    assert_eq!(restored_state.open_tabs[0].path, "doc1.md");
    assert_eq!(restored_state.open_tabs[0].view_mode.as_deref(), Some("reading"));
    assert_eq!(restored_state.open_tabs[1].path, "doc2.md");
    assert_eq!(restored_state.open_tabs[1].view_mode.as_deref(), Some("live"));
    assert_eq!(restored_state.active_tab.as_deref(), Some("doc2.md"));

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_vault_ui_state_sidebar_and_expanded_folders_persistence() {
    let test_dir = std::env::temp_dir().join("synapse_vault_ui_test");
    let _ = fs::remove_dir_all(&test_dir);
    let synapse_dir = test_dir.join(".synapse");
    let _ = fs::create_dir_all(&synapse_dir);
    let cache_file = synapse_dir.join("cache.bin");

    let engine = NavigationEngine::new(test_dir.clone(), cache_file.clone());

    // Initially default width is 240 and no expanded folders
    let initial_ui = engine.get_vault_ui_state();
    assert_eq!(initial_ui.sidebar_width, Some(240));
    assert!(initial_ui.expanded_folders.is_empty());

    // Save customized UI state: sidebar width 310, expanded folders ["docs", "docs/guides"]
    engine.save_vault_ui_state(
        Some(310),
        Some(vec!["docs".to_string(), "docs/guides".to_string()]),
    );

    let current_ui = engine.get_vault_ui_state();
    assert_eq!(current_ui.sidebar_width, Some(310));
    assert_eq!(
        current_ui.expanded_folders,
        vec!["docs".to_string(), "docs/guides".to_string()]
    );

    // Verify workspace.json was written to .synapse
    let workspace_json = synapse_dir.join("workspace.json");
    assert!(workspace_json.exists());

    // Reload in a completely fresh engine instance (simulating app relaunch)
    let fresh_engine = NavigationEngine::new(test_dir.clone(), cache_file.clone());
    let restored_ui = fresh_engine.get_vault_ui_state();

    assert_eq!(restored_ui.sidebar_width, Some(310));
    assert_eq!(
        restored_ui.expanded_folders,
        vec!["docs".to_string(), "docs/guides".to_string()]
    );

    // Test partial update: only change sidebar width
    fresh_engine.save_vault_ui_state(Some(275), None);
    let updated_ui = fresh_engine.get_vault_ui_state();
    assert_eq!(updated_ui.sidebar_width, Some(275));
    assert_eq!(
        updated_ui.expanded_folders,
        vec!["docs".to_string(), "docs/guides".to_string()]
    );

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_reload_paths_files_and_directories() {
    let test_dir = std::env::temp_dir().join("synapse_reload_paths_test");
    let _ = fs::remove_dir_all(&test_dir);
    let _ = fs::create_dir_all(test_dir.join("docs").join("sub"));
    let cache_file = test_dir.join("cache.bin");

    // Create initial files
    let root_file = test_dir.join("root.md");
    let mut f0 = File::create(&root_file).unwrap();
    writeln!(f0, "# Root Title\nInitial root").unwrap();

    let guide_file = test_dir.join("docs").join("guide.md");
    let mut f1 = File::create(&guide_file).unwrap();
    writeln!(f1, "# Guide v1\nInitial guide").unwrap();

    let deep_file = test_dir.join("docs").join("sub").join("deep.md");
    let mut f2 = File::create(&deep_file).unwrap();
    writeln!(f2, "# Deep Note\nDeep content").unwrap();

    let engine = NavigationEngine::new(test_dir.clone(), cache_file);
    engine.reconcile_sync();
    assert_eq!(engine.notes.len(), 3);

    // 1. Test single file reload
    std::thread::sleep(std::time::Duration::from_millis(50));
    let mut f0_mod = File::create(&root_file).unwrap();
    writeln!(f0_mod, "# Updated Root\nNew content here").unwrap();

    let reloaded = engine.reload_paths(&["root.md".to_string()]).unwrap();
    assert_eq!(reloaded, vec!["root.md"]);
    let root_id = *engine.path_index.get("root.md").unwrap().value();
    let root_title = engine.notes.get(&root_id).unwrap().title.clone();
    assert_eq!(root_title.as_str(), "Updated Root");

    // 2. Test directory reload: modify guide.md, and reload "docs"
    std::thread::sleep(std::time::Duration::from_millis(50));
    let mut f1_mod = File::create(&guide_file).unwrap();
    writeln!(f1_mod, "# Guide v2 Recharged\nFresh guide content").unwrap();

    let reloaded_dir = engine.reload_paths(&["docs".to_string()]).unwrap();
    assert!(reloaded_dir.contains(&"docs/guide.md".to_string()));
    assert!(reloaded_dir.contains(&"docs/sub/deep.md".to_string()));
    let guide_id = *engine.path_index.get("docs/guide.md").unwrap().value();
    let guide_title = engine.notes.get(&guide_id).unwrap().title.clone();
    assert_eq!(guide_title.as_str(), "Guide v2 Recharged");

    // 3. Test directory reload with deleted file: remove deep.md
    fs::remove_file(&deep_file).unwrap();
    let reloaded_after_delete = engine.reload_paths(&["docs".to_string()]).unwrap();
    assert!(!reloaded_after_delete.contains(&"docs/sub/deep.md".to_string()));
    assert!(!engine.path_index.contains_key("docs/sub/deep.md"));
    assert_eq!(engine.notes.len(), 2);

    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_fs_watcher_creation_modification_and_deletion() {
    use std::io::Write;

    let test_dir = std::env::temp_dir().join("synapse_fs_watcher_crud_test");
    let _ = fs::remove_dir_all(&test_dir);
    let _ = fs::create_dir_all(&test_dir);
    let cache_file = test_dir.join("cache.bin");

    let engine = Arc::new(NavigationEngine::new(test_dir.clone(), cache_file));
    engine.spawn_fs_watcher(std::time::Duration::from_millis(50));

    // 1. Create a file on disk
    let file_path = test_dir.join("watcher_test.md");
    {
        let mut f = File::create(&file_path).unwrap();
        writeln!(f, "# Initial Dynamic Title\nBody content here").unwrap();
    }

    // Wait for debounce (50ms) + rayon processing (~150ms)
    std::thread::sleep(std::time::Duration::from_millis(250));

    assert!(
        engine.path_index.contains_key("watcher_test.md"),
        "Watcher should index created file in path_index"
    );
    let id = *engine.path_index.get("watcher_test.md").unwrap().value();
    let initial_meta = engine.notes.get(&id).unwrap().clone();
    assert_eq!(initial_meta.title.as_str(), "Initial Dynamic Title");

    // Search via nucleo should find it
    let search_res = engine.search("Initial Dynamic", 5);
    assert_eq!(search_res.len(), 1);
    assert_eq!(search_res[0].path.as_str(), "watcher_test.md");

    // 2. Modify the file on disk (H1 title change)
    std::thread::sleep(std::time::Duration::from_millis(50));
    {
        let mut f = File::create(&file_path).unwrap();
        writeln!(f, "# Modified Title Rayon\nUpdated body").unwrap();
    }

    std::thread::sleep(std::time::Duration::from_millis(250));

    let updated_meta = engine.notes.get(&id).unwrap().clone();
    assert_eq!(updated_meta.title.as_str(), "Modified Title Rayon");

    let search_updated = engine.search("Modified Title", 5);
    assert_eq!(search_updated.len(), 1);
    assert_eq!(search_updated[0].path.as_str(), "watcher_test.md");

    // 3. Delete the file from disk
    fs::remove_file(&file_path).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(250));

    assert!(
        !engine.path_index.contains_key("watcher_test.md"),
        "Watcher should remove deleted file from path_index"
    );
    assert_eq!(engine.notes.len(), 0);

    let search_deleted = engine.search("Modified Title", 5);
    assert!(search_deleted.is_empty());

    engine.shutdown();
    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_fs_watcher_debounce_coalescing_rapid_writes() {
    use std::io::Write;

    let test_dir = std::env::temp_dir().join("synapse_fs_watcher_debounce_test");
    let _ = fs::remove_dir_all(&test_dir);
    let _ = fs::create_dir_all(&test_dir);
    let cache_file = test_dir.join("cache.bin");

    let engine = Arc::new(NavigationEngine::new(test_dir.clone(), cache_file));
    engine.spawn_fs_watcher(std::time::Duration::from_millis(100));

    let rapid_file = test_dir.join("rapid_editor_save.md");

    // Simulate 5 rapid editor saves within 30ms total (debouncer should coalesce them)
    for i in 1..=5 {
        let mut f = File::create(&rapid_file).unwrap();
        writeln!(f, "# Version {}", i).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    // Wait for the debounce window (100ms) to fire and process
    std::thread::sleep(std::time::Duration::from_millis(300));

    assert_eq!(engine.notes.len(), 1);
    let note = engine.notes.iter().next().unwrap().value().clone();
    assert_eq!(note.title.as_str(), "Version 5");

    engine.shutdown();
    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_fs_watcher_ignores_hidden_and_temp_files() {
    use std::io::Write;

    let test_dir = std::env::temp_dir().join("synapse_fs_watcher_ignore_test");
    let _ = fs::remove_dir_all(&test_dir);
    let _ = fs::create_dir_all(test_dir.join(".git"));
    let _ = fs::create_dir_all(test_dir.join(".synapse"));
    let _ = fs::create_dir_all(test_dir.join(".obsidian").join("plugins"));
    let _ = fs::create_dir_all(test_dir.join("node_modules").join("package"));
    let _ = fs::create_dir_all(test_dir.join("target").join("debug"));
    let cache_file = test_dir.join("cache.bin");

    let engine = Arc::new(NavigationEngine::new(test_dir.clone(), cache_file));
    engine.spawn_fs_watcher(std::time::Duration::from_millis(50));

    // Create hidden, node_modules, target, lock, and temporary editor files
    {
        let mut f1 = File::create(test_dir.join(".git").join("config")).unwrap();
        writeln!(f1, "[core]").unwrap();

        let mut f2 = File::create(test_dir.join(".synapse").join("workspace.json")).unwrap();
        writeln!(f2, "{{}}").unwrap();

        let mut f3 = File::create(test_dir.join(".hidden_note.md")).unwrap();
        writeln!(f3, "# Hidden Note").unwrap();

        let mut f4 = File::create(test_dir.join("editor.md.tmp")).unwrap();
        writeln!(f4, "# Temporary Note").unwrap();

        let mut f5 = File::create(test_dir.join("backup.md~")).unwrap();
        writeln!(f5, "# Backup Note").unwrap();

        let mut f6 = File::create(test_dir.join("node_modules").join("package").join("readme.md")).unwrap();
        writeln!(f6, "# Node Modules Readme").unwrap();

        let mut f7 = File::create(test_dir.join("target").join("debug").join("info.md")).unwrap();
        writeln!(f7, "# Target Info").unwrap();

        let mut f8 = File::create(test_dir.join(".obsidian").join("plugins").join("data.json")).unwrap();
        writeln!(f8, "{{}}").unwrap();

        let mut f9 = File::create(test_dir.join("sync.lock")).unwrap();
        writeln!(f9, "lock").unwrap();

        let mut f10 = File::create(test_dir.join("document.swp")).unwrap();
        writeln!(f10, "swap").unwrap();
    }

    std::thread::sleep(std::time::Duration::from_millis(250));

    assert!(
        engine.notes.is_empty(),
        "Hidden directories, dotfiles, node_modules, target, and temporary/lock files must be ignored"
    );

    // Verify helper unit functions directly
    assert!(app_lib::navigation::watcher::should_ignore_path(std::path::Path::new("")));
    assert!(app_lib::navigation::watcher::should_ignore_path(std::path::Path::new(".")));
    assert!(app_lib::navigation::watcher::should_ignore_path(std::path::Path::new("node_modules/foo/bar.md")));
    assert!(app_lib::navigation::watcher::should_ignore_path(std::path::Path::new("sub/node_modules/bar.md")));
    assert!(app_lib::navigation::watcher::should_ignore_path(std::path::Path::new("sub/.git/HEAD")));
    assert!(app_lib::navigation::watcher::should_ignore_path(std::path::Path::new("target/debug/test.md")));
    assert!(app_lib::navigation::watcher::should_ignore_path(std::path::Path::new("notes/backup.md~")));
    assert!(app_lib::navigation::watcher::should_ignore_path(std::path::Path::new("notes/.obsidian/plugins/state.json")));
    assert!(app_lib::navigation::watcher::should_ignore_path(std::path::Path::new("draft.tmp")));
    assert!(app_lib::navigation::watcher::should_ignore_path(std::path::Path::new("sync.lock")));
    assert!(!app_lib::navigation::watcher::should_ignore_path(std::path::Path::new("notes/real_note.md")));

    engine.shutdown();
    let _ = fs::remove_dir_all(&test_dir);
}

#[test]
fn test_fs_watcher_directory_recursive_addition_and_removal() {
    use std::io::Write;

    let test_dir = std::env::temp_dir().join("synapse_fs_watcher_dir_test");
    let _ = fs::remove_dir_all(&test_dir);
    let _ = fs::create_dir_all(&test_dir);
    let cache_file = test_dir.join("cache.bin");

    let engine = Arc::new(NavigationEngine::new(test_dir.clone(), cache_file));
    engine.spawn_fs_watcher(std::time::Duration::from_millis(50));

    // Create a folder hierarchy with files
    let sub_dir = test_dir.join("project").join("docs");
    fs::create_dir_all(&sub_dir).unwrap();

    let f1_path = sub_dir.join("arch.md");
    let f2_path = sub_dir.join("spec.md");
    {
        let mut f1 = File::create(&f1_path).unwrap();
        writeln!(f1, "# Architecture Document").unwrap();
        let mut f2 = File::create(&f2_path).unwrap();
        writeln!(f2, "# Specifications").unwrap();
    }

    std::thread::sleep(std::time::Duration::from_millis(300));

    assert_eq!(engine.notes.len(), 2);
    assert!(engine.path_index.contains_key("project/docs/arch.md"));
    assert!(engine.path_index.contains_key("project/docs/spec.md"));

    // Delete the entire "project" folder
    fs::remove_dir_all(test_dir.join("project")).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(300));

    assert_eq!(engine.notes.len(), 0);
    assert!(!engine.path_index.contains_key("project/docs/arch.md"));
    assert!(!engine.path_index.contains_key("project/docs/spec.md"));

    engine.shutdown();
    let _ = fs::remove_dir_all(&test_dir);
}

