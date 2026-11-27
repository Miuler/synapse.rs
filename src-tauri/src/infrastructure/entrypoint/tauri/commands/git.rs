use crate::application::dto::version::change_history_command::ChangeHistoryCommand;
use crate::application::dto::version::squash_preview_query::SquashPreviewQuery;
use crate::domain::version::history_change::{HistoryAction, SquashPreview, UndoPoint};
use crate::application::dto::version::checkout_branch_command::CheckoutBranchCommand;
use crate::application::dto::version::create_branch_command::CreateBranchCommand;
use crate::application::dto::version::get_branches_query::GetBranchesQuery;
use crate::application::dto::version::get_commit_details_query::GetCommitDetailsQuery;
use crate::application::dto::version::get_repo_log_query::GetRepoLogQuery;
use crate::application::dto::version::create_tag_command::CreateTagCommand;
use crate::application::dto::version::fast_forward_command::FastForwardCommand;
use crate::application::dto::version::delete_ref_command::{DeleteRefCommand, RefToDelete};
use crate::application::dto::version::get_file_diff_query::GetFileDiffQuery;
use crate::application::dto::version::get_file_revision_query::GetFileRevisionQuery;
use crate::application::dto::version::get_file_version_history_query::GetFileVersionHistoryQuery;
use crate::application::dto::version::get_image_diff_query::GetImageDiffQuery;
use crate::application::dto::version::get_vault_version_state_query::GetVaultVersionStateQuery;
use crate::application::dto::version::push_versions_command::PushVersionsCommand;
use crate::application::dto::version::revert_files_command::RevertFilesCommand;
use crate::application::dto::version::save_version_command::SaveVersionCommand;
use crate::application::dto::version::track_files_command::TrackFilesCommand;
use crate::application::dto::version::untrack_files_command::UntrackFilesCommand;
use crate::domain::version::version::{
    BranchList, CommitDetails, FileDiff, FileRevision, FileVersionHistory, ImageDiff, RepoLog,
    VaultVersionState,
};
use crate::infrastructure::config::state::AppState;
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub async fn get_vault_git_status(
    state: State<'_, AppState>,
    folder_path: Option<String>,
) -> Result<VaultVersionState, String> {
    let vault_path = match folder_path {
        Some(p) => {
            let p = PathBuf::from(p);
            if !state.allows_path(&p) && !state.is_known_vault(&p) {
                return Err(format!("Ruta no autorizada: {}", p.display()));
            }
            p
        }
        None => {
            let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
            match *guard {
                Some(ref p) => p.clone(),
                None => {
                    return Ok(VaultVersionState {
                        is_repo: false,
                        branch: None,
                        statuses: Default::default(),
                        repos: Vec::new(),
                    });
                }
            }
        }
    };
    let use_case = state.get_vault_version_state_use_case.clone();
    tauri::async_runtime::spawn_blocking(move || {
        use_case.execute(GetVaultVersionStateQuery { vault_path })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn git_add_paths(state: State<'_, AppState>, paths: Vec<String>) -> Result<(), String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state
        .track_files_use_case
        .execute(TrackFilesCommand {
            vault_path: vault_path.clone(),
            paths,
        })
        .map_err(|err| {
            log::error!("[commands::git_add_paths] {}", err);
            eprintln!("[commands::git_add_paths] {}", err);
            err
        })
}

#[tauri::command]
pub fn git_restore_paths(state: State<'_, AppState>, paths: Vec<String>) -> Result<(), String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state
        .revert_files_use_case
        .execute(RevertFilesCommand {
            vault_path: vault_path.clone(),
            paths,
        })
        .map_err(|err| {
            log::error!("[commands::git_restore_paths] {}", err);
            eprintln!("[commands::git_restore_paths] {}", err);
            err
        })
}

#[tauri::command]
pub fn git_restore_staged_paths(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<(), String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state
        .untrack_files_use_case
        .execute(UntrackFilesCommand {
            vault_path: vault_path.clone(),
            paths,
        })
        .map_err(|err| {
            log::error!("[commands::git_restore_staged_paths] {}", err);
            eprintln!("[commands::git_restore_staged_paths] {}", err);
            err
        })
}

#[tauri::command]
pub fn git_commit_paths(
    state: State<'_, AppState>,
    paths: Vec<String>,
    message: String,
) -> Result<String, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state
        .save_version_use_case
        .execute(SaveVersionCommand {
            vault_path: vault_path.clone(),
            paths,
            message,
        })
        .map_err(|err| {
            log::error!("[commands::git_commit_paths] {}", err);
            eprintln!("[commands::git_commit_paths] {}", err);
            err
        })
}

#[tauri::command(async)]
pub fn get_git_file_diff(
    state: State<'_, AppState>,
    path: String,
    staged: bool,
    revision: Option<String>,
) -> Result<FileDiff, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state
        .get_file_diff_use_case
        .execute(GetFileDiffQuery {
            vault_path: vault_path.clone(),
            path,
            staged,
            revision,
        })
        .map_err(|err| {
            log::error!("[commands::get_git_file_diff] {}", err);
            eprintln!("[commands::get_git_file_diff] {}", err);
            err
        })
}

/// Lee un archivo de texto de la bóveda sin permitir salir de ella.
fn read_vault_text(vault_path: &std::path::Path, path: &str) -> Result<String, String> {
    let vault = vault_path.canonicalize().map_err(|e| e.to_string())?;
    let file = vault
        .join(path)
        .canonicalize()
        .map_err(|e| format!("No se pudo abrir {}: {}", path, e))?;
    if !file.starts_with(&vault) {
        return Err(format!("{} está fuera de la bóveda", path));
    }
    std::fs::read_to_string(&file)
        .map_err(|_| format!("{} no es un archivo de texto y no se puede comparar", path))
}

/// Diff de dos archivos de la bóveda; no depende de git.
#[tauri::command(async)]
pub fn diff_vault_files(
    state: State<'_, AppState>,
    old_path: String,
    new_path: String,
) -> Result<FileDiff, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    let old_content = read_vault_text(vault_path, &old_path)?;
    let new_content = read_vault_text(vault_path, &new_path)?;
    Ok(build_files_diff(
        old_path,
        new_path,
        old_content,
        new_content,
    ))
}

/// Diff de un archivo entre dos revisiones de Git (p. ej. el parche de un commit: `abc^` contra `abc`).
/// Si el archivo no existe en una de ellas, esa versión cuenta como vacía.
#[tauri::command(async)]
pub fn diff_git_revisions(
    state: State<'_, AppState>,
    path: String,
    old_revision: String,
    new_revision: String,
) -> Result<FileDiff, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    let read = |revision: String| -> Result<String, String> {
        let rev = state
            .get_file_revision_use_case
            .execute(GetFileRevisionQuery {
                vault_path: vault_path.clone(),
                path: path.clone(),
                staged: false,
                revision: Some(revision),
            })?;
        Ok(if rev.exists {
            rev.content.unwrap_or_default()
        } else {
            String::new()
        })
    };
    let old_content = read(old_revision)?;
    let new_content = read(new_revision)?;
    Ok(build_files_diff(
        path.clone(),
        path,
        old_content,
        new_content,
    ))
}

fn build_files_diff(
    old_path: String,
    new_path: String,
    old_content: String,
    new_content: String,
) -> FileDiff {
    let unified = similar::TextDiff::from_lines(&old_content, &new_content)
        .unified_diff()
        .header(&format!("a/{}", old_path), &format!("b/{}", new_path))
        .to_string();
    let diff = if unified.is_empty() {
        unified
    } else {
        format!("diff --git a/{} b/{}\n{}", old_path, new_path, unified)
    };
    FileDiff {
        only_staged: false,
        path: new_path.clone(),
        is_staged: false,
        old_file_name: old_path,
        new_file_name: new_path,
        old_content,
        new_content,
        diff,
    }
}

#[tauri::command(async)]
pub fn get_git_file_revision(
    state: State<'_, AppState>,
    path: String,
    staged: bool,
    revision: Option<String>,
) -> Result<FileRevision, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state
        .get_file_revision_use_case
        .execute(GetFileRevisionQuery {
            vault_path: vault_path.clone(),
            path,
            staged,
            revision,
        })
        .map_err(|err| {
            log::error!("[commands::get_git_file_revision] {}", err);
            eprintln!("[commands::get_git_file_revision] {}", err);
            err
        })
}

#[tauri::command(async)]
pub fn get_git_image_diff(
    state: State<'_, AppState>,
    path: String,
    staged: bool,
    revision: Option<String>,
) -> Result<ImageDiff, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state
        .get_image_diff_use_case
        .execute(GetImageDiffQuery {
            vault_path: vault_path.clone(),
            path,
            staged,
            revision,
        })
        .map_err(|err| {
            log::error!("[commands::get_git_image_diff] {}", err);
            eprintln!("[commands::get_git_image_diff] {}", err);
            err
        })
}

#[tauri::command(async)]
pub fn get_git_file_log(
    state: State<'_, AppState>,
    path: String,
    max_count: Option<usize>,
) -> Result<FileVersionHistory, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    state
        .get_file_version_history_use_case
        .execute(GetFileVersionHistoryQuery {
            vault_path: vault_path.clone(),
            path,
            max_count,
        })
        .map_err(|err| {
            log::error!("[commands::get_git_file_log] {}", err);
            eprintln!("[commands::get_git_file_log] {}", err);
            err
        })
}

/// Carpeta del repositorio elegido (`repo`, relativo a la bóveda; vacío o ausente = la raíz).
fn repo_dir(state: &State<'_, AppState>, repo: Option<&str>) -> Result<PathBuf, String> {
    let guard = state.active_vault_path.lock().map_err(|e| e.to_string())?;
    let Some(ref vault_path) = *guard else {
        return Err("No hay ninguna bóveda abierta".to_string());
    };
    let repo = repo.unwrap_or("").trim_matches('/');
    if repo.is_empty() {
        return Ok(vault_path.clone());
    }
    if repo.split('/').any(|c| c == ".." || c.is_empty()) {
        return Err(format!("Repositorio no válido: {repo}"));
    }
    Ok(vault_path.join(repo))
}

#[tauri::command(async)]
pub fn get_git_branches(state: State<'_, AppState>, repo: Option<String>) -> Result<BranchList, String> {
    let vault_path = repo_dir(&state, repo.as_deref())?;
    state.get_branches_use_case.execute(GetBranchesQuery { vault_path })
}

/// Log de todas las ramas del repositorio elegido, con padres y refs para dibujar el grafo.
#[tauri::command(async)]
pub fn get_git_repo_log(
    state: State<'_, AppState>,
    repo: Option<String>,
    max_count: Option<usize>,
) -> Result<RepoLog, String> {
    let vault_path = repo_dir(&state, repo.as_deref())?;
    state
        .get_repo_log_use_case
        .execute(GetRepoLogQuery { vault_path, max_count })
        .map_err(|err| {
            log::error!("[commands::get_git_repo_log] {}", err);
            err
        })
}

/// Mensaje completo y archivos cambiados de un commit (rutas relativas al repositorio).
#[tauri::command(async)]
pub fn get_git_commit_details(
    state: State<'_, AppState>,
    repo: Option<String>,
    commit: String,
) -> Result<CommitDetails, String> {
    let vault_path = repo_dir(&state, repo.as_deref())?;
    state
        .get_commit_details_use_case
        .execute(GetCommitDetailsQuery { vault_path, commit })
}

#[tauri::command]
pub fn git_create_tag(
    state: State<'_, AppState>,
    repo: Option<String>,
    name: String,
    commit: String,
    message: Option<String>,
) -> Result<String, String> {
    let vault_path = repo_dir(&state, repo.as_deref())?;
    state.create_tag_use_case.execute(CreateTagCommand { vault_path, name, commit, message })
}

/// Avanza la rama local `branch` hasta `target` (fast-forward); con `dry_run` solo comprueba.
#[tauri::command]
pub fn git_fast_forward(
    state: State<'_, AppState>,
    repo: Option<String>,
    branch: String,
    target: String,
    dry_run: Option<bool>,
) -> Result<String, String> {
    let vault_path = repo_dir(&state, repo.as_deref())?;
    state.fast_forward_use_case.execute(FastForwardCommand { vault_path, branch, target, dry_run: dry_run.unwrap_or(false) })
}

/// Qué uniría un squash desde `from` hasta el último commit de la rama actual.
#[tauri::command]
pub fn git_squash_preview(
    state: State<'_, AppState>,
    repo: Option<String>,
    from: String,
) -> Result<SquashPreview, String> {
    let vault_path = repo_dir(&state, repo.as_deref())?;
    state
        .squash_preview_use_case
        .execute(SquashPreviewQuery { vault_path, from })
}

/// Squash, revert o reset de la rama actual, y su deshacer.
#[tauri::command]
pub fn git_change_history(
    state: State<'_, AppState>,
    repo: Option<String>,
    action: HistoryAction,
) -> Result<Option<UndoPoint>, String> {
    let vault_path = repo_dir(&state, repo.as_deref())?;
    let result = state
        .change_history_use_case
        .execute(ChangeHistoryCommand { vault_path, action });

    // Sincronizar el árbol: revert y reset --hard cambian archivos (también si fallan a medias)
    if let Ok(guard) = state.navigation_engine.lock()
        && let Some(ref engine) = *guard {
            engine.spawn_reconciliation();
        }

    result
}

/// Borra un tag (`kind` = `tag`) o una rama local (`kind` = `branch`, `force` = `-D`).
#[tauri::command]
pub fn git_delete_ref(
    state: State<'_, AppState>,
    repo: Option<String>,
    kind: String,
    name: String,
    force: Option<bool>,
) -> Result<String, String> {
    let vault_path = repo_dir(&state, repo.as_deref())?;
    let target = match kind.as_str() {
        "tag" => RefToDelete::Tag(name),
        "branch" => RefToDelete::Branch { name, force: force.unwrap_or(false) },
        other => return Err(format!("Tipo de referencia no válido: {other}")),
    };
    state.delete_ref_use_case.execute(DeleteRefCommand { vault_path, target })
}

#[tauri::command]
pub fn git_checkout_branch(
    state: State<'_, AppState>,
    branch_name: String,
    repo: Option<String>,
) -> Result<String, String> {
    let vault_path = repo_dir(&state, repo.as_deref())?;
    let result = state
        .checkout_branch_use_case
        .execute(CheckoutBranchCommand {
            vault_path,
            branch_name,
        })?;

    // Sincronizar el árbol de navegación ya que los archivos cambiaron en disco
    if let Ok(guard) = state.navigation_engine.lock()
        && let Some(ref engine) = *guard {
            engine.spawn_reconciliation();
        }

    Ok(result)
}

#[tauri::command]
pub fn git_create_branch(
    state: State<'_, AppState>,
    new_branch: String,
    base_branch: String,
    checkout: bool,
    repo: Option<String>,
) -> Result<String, String> {
    let vault_path = repo_dir(&state, repo.as_deref())?;
    let result = state.create_branch_use_case.execute(CreateBranchCommand {
        vault_path,
        new_branch,
        base_branch,
        checkout,
    })?;

    if checkout
        && let Ok(guard) = state.navigation_engine.lock()
            && let Some(ref engine) = *guard {
                engine.spawn_reconciliation();
            }

    Ok(result)
}

#[tauri::command]
pub async fn git_push_vault(state: State<'_, AppState>, repo: Option<String>) -> Result<String, String> {
    let vault_path = repo_dir(&state, repo.as_deref())?;
    let use_case = state.push_versions_use_case.clone();
    tauri::async_runtime::spawn_blocking(move || {
        use_case.execute(PushVersionsCommand { vault_path })
    })
    .await
    .map_err(|e| e.to_string())?
}

// ── Ejecutable de Git ───────────────────────────────────────────────────────────────────────────
// La ruta elegida se guarda en `<config>/config.jsonc` (`git.path`); sin ella se usa `git` del PATH.

use crate::infrastructure::repository::file::workspace_registry_repository_impl::WorkspaceRegistryRepositoryImpl;
use crate::infrastructure::service::os::git_binary::{self, GitInfo};

/// Git en uso: ruta configurada (si hay) y versión (`None` si no se encuentra).
#[tauri::command]
pub async fn get_git_info() -> GitInfo {
    git_binary::info()
}

/// Guarda la ruta del ejecutable de Git (`None` vuelve a usar el del PATH).
#[tauri::command]
pub async fn set_git_path(
    state: State<'_, AppState>,
    path: Option<String>,
) -> Result<GitInfo, String> {
    // Comprobar la ruta ya la ejecuta: si no salió del diálogo, se confirma antes
    if let Some(p) = path.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
        let current = git_binary::configured_path();
        if current.as_deref() != Some(std::path::Path::new(p))
            && !state.allowed_paths.has_program(std::path::Path::new(p))
        {
            crate::infrastructure::config::allowed_paths::confirm_native(
                "Ejecutable de Git",
                format!("Synapse ejecutará este programa como Git:\n\n{p:?}\n\n¿Usarlo?"),
            )
            .await?;
        }
    }
    git_binary::save(&WorkspaceRegistryRepositoryImpl::resolve_config_dir(), path.as_deref())
}

/// Diálogo nativo para elegir el ejecutable de Git.
#[tauri::command]
pub async fn pick_git_executable(state: State<'_, AppState>) -> Result<Option<String>, String> {
    #[cfg(feature = "desktop")]
    {
        let mut dialog = rfd::AsyncFileDialog::new().set_title("Elige el ejecutable de Git");
        if cfg!(windows) {
            dialog = dialog.add_filter("git.exe", &["exe"]);
            if std::path::Path::new(r"C:\Program Files\Git\cmd").is_dir() {
                dialog = dialog.set_directory(r"C:\Program Files\Git\cmd");
            }
        }
        Ok(dialog.pick_file().await.map(|f| {
            state.allowed_paths.grant_program(f.path());
            f.path().to_string_lossy().into_owned()
        }))
    }
    #[cfg(not(feature = "desktop"))]
    {
        let _ = state;
        Ok(None)
    }
}
