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
        elapsed.as_millis() <= 15,
        "Cold start must be sub-15ms in debug mode (was {:?})",
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
