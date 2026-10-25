use crate::domain::models::full_text::{FullTextIndexStatus, FullTextSearchResponse};
use crate::domain::services::search_service::{SearchResult, SearchService};
use crate::infrastructure::services::nucleo_search_service::NucleoSearchService;
use crate::infrastructure::tauri::dto::QuickOpenSearchResult;
use crate::infrastructure::tauri::state::AppState;
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
pub fn search_items_command(query: String, items: Vec<String>) -> Vec<SearchResult> {
    let search_service = NucleoSearchService::new();
    search_service.search_items(&query, &items)
}

#[tauri::command]
pub fn search_notes_command(
    state: State<'_, AppState>,
    query: String,
) -> Result<QuickOpenSearchResult, String> {
    let guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    let Some(ref engine) = *guard else {
        return Ok(QuickOpenSearchResult {
            results: Vec::new(),
            total_files: 0,
            matched_files: 0,
        });
    };
    let (matches, total_files, matched_files) = engine.search_with_stats(&query, 50);
    let recent_notes = engine.get_recent_notes(15);
    let recent_set: std::collections::HashSet<compact_str::CompactString> =
        recent_notes.into_iter().map(|n| n.path).collect();

    let results = matches
        .into_iter()
        .enumerate()
        .map(|(idx, m)| {
            let is_recent = recent_set.contains(&m.path);
            SearchResult {
                text: m.title.to_string(),
                score: (1000_u32).saturating_sub(idx as u32 * 10),
                match_indices: Vec::new(),
                note_path: Some(m.path.to_string()),
                is_recent: Some(is_recent),
                last_opened_nanos: m.last_opened_nanos,
            }
        })
        .collect();
    Ok(QuickOpenSearchResult {
        results,
        total_files,
        matched_files,
    })
}

#[tauri::command]
pub async fn full_text_search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<FullTextSearchResponse, String> {
    let ft_components = {
        let guard = state.full_text.lock().map_err(|e| e.to_string())?;
        guard.clone()
    };
    let Some(ft_components) = ft_components else {
        return Ok(FullTextSearchResponse {
            hits: Vec::new(),
            total_hits: 0,
            elapsed_ms: 0.0,
            status: FullTextIndexStatus::default(),
        });
    };

    let start = std::time::Instant::now();
    let res = tauri::async_runtime::spawn_blocking(move || {
        let (hits, total_hits) = ft_components.use_cases.search(&query, limit)?;
        let status = ft_components.indexer.status();
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        Ok(FullTextSearchResponse {
            hits,
            total_hits,
            elapsed_ms,
            status,
        })
    })
    .await
    .map_err(|e| format!("Error en tarea de búsqueda: {}", e))?;

    res
}

#[tauri::command]
pub fn get_full_text_index_status(state: State<'_, AppState>) -> Result<FullTextIndexStatus, String> {
    let guard = state.full_text.lock().map_err(|e| e.to_string())?;
    let Some(ref ft_components) = *guard else {
        return Ok(FullTextIndexStatus::default());
    };
    Ok(ft_components.indexer.status())
}

#[tauri::command]
pub fn rebuild_full_text_index(state: State<'_, AppState>) -> Result<(), String> {
    let guard = state.full_text.lock().map_err(|e| e.to_string())?;
    let Some(ref ft_components) = *guard else {
        return Ok(());
    };
    ft_components.indexer.rebuild()
}

#[tauri::command]
pub fn resolve_vault_link(
    state: State<'_, AppState>,
    link: String,
    base_file: Option<String>,
) -> Result<Option<String>, String> {
    let guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    let Some(ref engine) = *guard else {
        return Ok(None);
    };
    Ok(engine.resolve_link_path_with_base(&link, base_file.as_deref()))
}

#[tauri::command]
pub fn resolve_vault_links(
    state: State<'_, AppState>,
    links: Vec<String>,
    base_file: Option<String>,
) -> Result<HashMap<String, String>, String> {
    let guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    let Some(ref engine) = *guard else {
        return Ok(HashMap::new());
    };
    Ok(engine.resolve_link_paths_with_base(&links, base_file.as_deref()))
}

#[tauri::command]
pub fn render_markdown_wikilinks(
    state: State<'_, AppState>,
    content: String,
    base_file: Option<String>,
) -> Result<String, String> {
    let guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    let Some(ref engine) = *guard else {
        return Ok(content);
    };
    Ok(engine.render_wikilinks_in_markdown_with_base(&content, base_file.as_deref()))
}

#[tauri::command]
pub fn get_vault_graph(
    state: State<'_, AppState>,
    focus_path: Option<String>,
    depth: Option<usize>,
) -> Result<crate::domain::models::graph::VaultGraphData, String> {
    let guard = state.navigation_engine.lock().map_err(|e| e.to_string())?;
    let Some(ref engine) = *guard else {
        return Ok(crate::domain::models::graph::VaultGraphData::default());
    };
    Ok(engine.get_vault_graph(focus_path.as_deref(), depth))
}
