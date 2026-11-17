//! Comandos Tauri de los agentes: convierten la entrada en `Command`/`Query` y llaman al caso de uso.

use crate::application::dto::agent::*;
use crate::domain::agent::{
    AgentLaunch, AgentProfile, AgentSession, AgentTranscript, AgentTranscriptSummary,
    PermissionDecision, PromptBlock, ToolServer,
};
use crate::infrastructure::config::agent_state::AgentState;
use crate::infrastructure::config::state::AppState;
use std::collections::BTreeMap;
use tauri::State;

#[tauri::command]
pub async fn agent_list(state: State<'_, AgentState>) -> Result<Vec<AgentProfile>, String> {
    state.list_agents.execute(ListAgentsQuery).await
}

#[tauri::command]
pub async fn agent_install(state: State<'_, AgentState>, agent_id: String) -> Result<(), String> {
    state
        .install_agent
        .execute(InstallAgentCommand { agent_id })
        .await
}

#[tauri::command]
pub fn agent_save_profile(
    state: State<'_, AgentState>,
    id: String,
    name: String,
    command: String,
    args: Vec<String>,
    env: BTreeMap<String, String>,
) -> Result<AgentProfile, String> {
    state.save_agent_profile.execute(SaveAgentProfileCommand {
        id,
        name,
        launch: AgentLaunch { command, args, env },
    })
}

#[tauri::command]
pub fn agent_delete_profile(state: State<'_, AgentState>, id: String) -> Result<(), String> {
    state
        .delete_agent_profile
        .execute(DeleteAgentProfileCommand { id })
}

/// El MCP de Synapse para el agente: siempre el de la instancia abierta (comparte índice y estado
/// con la ventana). Se arranca si estaba apagado. Los agentes con MCP por HTTP se conectan directo;
/// los que solo usan stdio, a través de `synapsers --mcp-proxy <url>`, que reenvía a la misma instancia.
fn synapse_tools(app: &AppState, conversation: Option<&str>) -> Result<Vec<ToolServer>, String> {
    let status = app
        .mcp_runtime
        .start()
        .map_err(|e| format!("No se pudo iniciar el MCP de Synapse: {e}"))?;
    if !status.running {
        return Err("No se pudo iniciar el MCP de Synapse".into());
    }
    // Cada conversación usa su propia URL del MCP, para saber quién habla con las demás
    let url = match conversation
        .filter(|c| c.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-'))
    {
        Some(conv) => format!("{}?conv={conv}", status.endpoint),
        None => status.endpoint,
    };
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    Ok(vec![ToolServer {
        name: crate::infrastructure::integration::service::acp::agent_runner_impl::SYNAPSE_TOOLS
            .into(),
        http_url: Some(url.clone()),
        process: Some(AgentLaunch {
            command: exe.to_string_lossy().into_owned(),
            args: vec!["--mcp-proxy".into(), url],
            env: BTreeMap::new(),
        }),
    }])
}

/// Indicaciones de inicio: el agente trabaja con el MCP de esta instancia de Synapse y, si el
/// modo feedback está activo, anota lo que le falte al MCP.
pub(crate) fn session_instructions(feedback: bool) -> String {
    let name = crate::infrastructure::integration::service::acp::agent_runner_impl::SYNAPSE_TOOLS;
    let mut text = format!(
        "[Synapse] Estás trabajando dentro de Synapse, una app de notas, sobre la bóveda abierta \
         (tu carpeta de trabajo). El servidor MCP `{name}` es el de esta misma ventana de Synapse: \
         úsalo siempre para la bóveda y no otros servidores MCP que también se llamen «synapse» \
         (pueden apuntar a otra bóveda o a otra instancia).\n\
         - Para leer notas usa `read_note` (primero `mode: \"outline_only\"` en notas largas), \
         `search_notes`, `full_text_search`, `search_in_note`, `list_folder_contents` y \
         `get_backlinks_and_relations`.\n\
         - Para crear, editar, mover o borrar notas usa `write_note`, `patch_note`, \
         `append_to_note`, `update_note_metadata`, `move_notes` y `delete_note`, no la escritura \
         directa de archivos: así Synapse mantiene su índice, sus enlaces y sus metadatos al día.\n\
         - Usa la escritura directa de archivos solo si `{name}` no puede hacer lo que necesitas.\n\
         - En el panel puede haber otras conversaciones con agentes (#1, #2…). Si el usuario te pide \
         preguntarle o avisarle algo a otra, usa `list_conversations` y `send_to_conversation` de \
         `{name}`; es asíncrono: si preguntas, su respuesta te llegará después como un mensaje nuevo.\n\
         - Si lo que necesitas está en otra bóveda del usuario (por ejemplo, documentación de otro \
         sistema), usa `list_vaults` y pasa `vault` a las herramientas de lectura (`search_notes`, \
         `read_note`…). Es solo lectura y la primera vez el usuario debe permitirlo.\n\
         - Para el historial de Git de un archivo usa `git_file_history`; para saber qué cambió \
         entre dos commits, `git_diff_revisions` (y `render_diagram` con `revision` si es una \
         imagen o un diagrama); para leerlo como era en un commit, `git_read_revision`."
    );
    if feedback {
        text.push_str(&format!(
            "\n- Modo feedback activo: cada vez que a `{name}` le falte algo para completar la tarea \
             (una herramienta, un parámetro, un formato), regístralo con su herramienta \
             `report_feedback` antes de usar una alternativa."
        ));
    }
    text
}

#[tauri::command]
pub async fn agent_start_session(
    state: State<'_, AgentState>,
    app: State<'_, AppState>,
    agent_id: String,
    cwd: String,
    feedback: Option<bool>,
    resume: Option<String>,
    conversation_id: Option<String>,
) -> Result<AgentSession, String> {
    let tools = synapse_tools(&app, conversation_id.as_deref())?;
    let instructions = Some(session_instructions(
        feedback.unwrap_or(false) && cfg!(feature = "develop"),
    ));
    state
        .start_agent_session
        .execute(StartAgentSessionCommand {
            agent_id,
            cwd,
            tools,
            instructions,
            resume,
        })
        .await
}

#[tauri::command]
pub async fn agent_send_prompt(
    state: State<'_, AgentState>,
    session_id: String,
    blocks: Vec<PromptBlock>,
) -> Result<String, String> {
    state
        .send_agent_prompt
        .execute(SendAgentPromptCommand { session_id, blocks })
        .await
}

#[tauri::command]
pub async fn agent_set_mode(
    state: State<'_, AgentState>,
    session_id: String,
    mode_id: String,
) -> Result<(), String> {
    state
        .change_agent_mode
        .execute(ChangeAgentModeCommand {
            session_id,
            mode_id,
        })
        .await
}

#[tauri::command]
pub async fn agent_set_model(
    state: State<'_, AgentState>,
    session_id: String,
    model_id: String,
) -> Result<(), String> {
    state
        .change_agent_model
        .execute(ChangeAgentModelCommand {
            session_id,
            model_id,
        })
        .await
}

#[tauri::command]
pub async fn agent_cancel_prompt(
    state: State<'_, AgentState>,
    session_id: String,
) -> Result<(), String> {
    state
        .cancel_agent_prompt
        .execute(CancelAgentPromptCommand { session_id })
        .await
}

/// `option_id` vacío o ausente equivale a cancelar.
#[tauri::command]
pub async fn agent_respond_permission(
    state: State<'_, AgentState>,
    session_id: String,
    request_id: String,
    option_id: Option<String>,
) -> Result<(), String> {
    let decision = match option_id.filter(|o| !o.is_empty()) {
        Some(option_id) => PermissionDecision::Selected { option_id },
        None => PermissionDecision::Cancelled,
    };
    state
        .respond_permission
        .execute(RespondPermissionCommand {
            session_id,
            request_id,
            decision,
        })
        .await
}

#[tauri::command]
pub async fn agent_stop_session(
    state: State<'_, AgentState>,
    session_id: String,
) -> Result<(), String> {
    state
        .stop_agent_session
        .execute(StopAgentSessionCommand { session_id })
        .await
}

#[tauri::command]
pub fn agent_save_transcript(
    state: State<'_, AgentState>,
    transcript: AgentTranscript,
) -> Result<(), String> {
    state
        .save_agent_transcript
        .execute(SaveAgentTranscriptCommand { transcript })
}

#[tauri::command]
pub fn agent_list_transcripts(
    state: State<'_, AgentState>,
    cwd: String,
) -> Result<Vec<AgentTranscriptSummary>, String> {
    state
        .list_agent_transcripts
        .execute(ListAgentTranscriptsQuery { cwd })
}

#[tauri::command]
pub fn agent_load_transcript(
    state: State<'_, AgentState>,
    cwd: String,
    id: String,
) -> Result<AgentTranscript, String> {
    state
        .load_agent_transcript
        .execute(LoadAgentTranscriptQuery { cwd, id })
}

#[tauri::command]
pub fn agent_delete_transcript(
    state: State<'_, AgentState>,
    cwd: String,
    id: String,
) -> Result<(), String> {
    state
        .delete_agent_transcript
        .execute(DeleteAgentTranscriptCommand { cwd, id })
}

#[tauri::command]
pub fn agent_open_transcript(
    state: State<'_, AgentState>,
    cwd: String,
    id: String,
) -> Result<(), String> {
    state
        .open_agent_transcript
        .execute(OpenAgentTranscriptCommand { cwd, id })
}

#[tauri::command]
pub fn agent_delete_all_transcripts(
    state: State<'_, AgentState>,
    cwd: String,
) -> Result<(), String> {
    state
        .delete_all_agent_transcripts
        .execute(DeleteAllAgentTranscriptsCommand { cwd })
}

/// Si esta compilación incluye el modo feedback del MCP (feature `develop`).
#[tauri::command]
pub fn agent_feedback_available() -> bool {
    cfg!(feature = "develop")
}

/// Registro de depuración de ACP (Ajustes → Agentes de IA): activa o desactiva y devuelve el archivo.
#[tauri::command]
pub fn agent_set_debug_log(enabled: bool) -> String {
    use crate::infrastructure::integration::service::acp::debug_log;
    debug_log::set_enabled(enabled);
    debug_log::path().to_string_lossy().into_owned()
}

/// Conversaciones que comparte un proceso del agente (Ajustes → Agentes de IA).
#[tauri::command]
pub fn agent_set_process_limit(state: State<'_, AgentState>, agent_id: String, limit: usize) {
    state
        .set_agent_process_limit
        .execute(SetAgentProcessLimitCommand { agent_id, limit });
}
