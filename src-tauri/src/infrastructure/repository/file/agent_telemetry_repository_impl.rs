use crate::domain::agent::{
    AgentSessionTelemetry, AgentTelemetryRepository, AgentTranscript, AgentUsageRecord,
    IterationTelemetry, McpCallTelemetry,
};
use crate::infrastructure::repository::file::workspace_registry_repository_impl::WorkspaceRegistryRepositoryImpl;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Persiste:
/// 1. Telemetría detallada por conversación en `<bóveda>/.synapse/agent-sessions/telemetry/<id>.json`
///    (iteraciones, duración, uso de herramientas y request/response de llamadas al MCP de Synapse).
/// 2. Registro global de uso en `~/.config/synapsers/agent-usage.jsonl` con referencia a ambos JSON.
#[derive(Clone)]
pub struct AgentTelemetryRepositoryImpl {
    global_usage_path: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl Default for AgentTelemetryRepositoryImpl {
    fn default() -> Self {
        Self::new(WorkspaceRegistryRepositoryImpl::resolve_config_dir().join("agent-usage.jsonl"))
    }
}

impl AgentTelemetryRepositoryImpl {
    pub fn new(global_usage_path: PathBuf) -> Self {
        Self {
            global_usage_path,
            lock: Arc::default(),
        }
    }

    fn validate_vault_and_id(cwd: &str, conversation_id: &str) -> Result<(), String> {
        if cwd.is_empty() || !Path::new(cwd).is_dir() {
            return Err(format!("Vault «{cwd}» does not exist"));
        }
        if !AgentTranscript::valid_id(conversation_id) {
            return Err(format!(
                "Invalid conversation ID: {conversation_id}"
            ));
        }
        Ok(())
    }

    fn transcript_path(cwd: &str, conversation_id: &str) -> PathBuf {
        Path::new(cwd)
            .join(".synapse")
            .join("agent-sessions")
            .join(format!("{conversation_id}.json"))
    }

    fn telemetry_path(cwd: &str, conversation_id: &str) -> PathBuf {
        Path::new(cwd)
            .join(".synapse")
            .join("agent-sessions")
            .join("telemetry")
            .join(format!("{conversation_id}.json"))
    }

    fn load_or_create(cwd: &str, conversation_id: &str) -> Result<AgentSessionTelemetry, String> {
        let path = Self::telemetry_path(cwd, conversation_id);
        match std::fs::read(&path) {
            Ok(data) => serde_json::from_slice(&data)
                .map_err(|e| format!("Invalid session telemetry: {e}")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(AgentSessionTelemetry {
                conversation_id: conversation_id.to_string(),
                transcript_path: Self::transcript_path(cwd, conversation_id)
                    .to_string_lossy()
                    .into_owned(),
                agent_id: String::new(),
                agent_name: String::new(),
                cwd: cwd.to_string(),
                acp_session_id: None,
                iterations: vec![],
                mcp_calls: vec![],
            }),
            Err(e) => Err(e.to_string()),
        }
    }

    fn save_session_telemetry(
        path: &Path,
        telemetry: &AgentSessionTelemetry,
    ) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let data = serde_json::to_vec_pretty(telemetry).map_err(|e| e.to_string())?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, data).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, path).map_err(|e| e.to_string())
    }

    fn append_global_usage(&self, record: &AgentUsageRecord) -> Result<(), String> {
        if let Some(dir) = self.global_usage_path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let line = serde_json::to_string(record).map_err(|e| e.to_string())?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.global_usage_path)
            .map_err(|e| e.to_string())?;
        writeln!(file, "{line}").map_err(|e| e.to_string())
    }

    fn now_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }

    fn vault_name(cwd: &str) -> String {
        Path::new(cwd)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(cwd)
            .to_string()
    }

    fn detect_git_branch(cwd: &str) -> Option<String> {
        let head = std::fs::read_to_string(Path::new(cwd).join(".git").join("HEAD")).ok()?;
        let trimmed = head.trim();
        if let Some(branch) = trimmed.strip_prefix("ref: refs/heads/") {
            Some(branch.to_string())
        } else if !trimmed.is_empty() {
            Some(trimmed.chars().take(12).collect())
        } else {
            None
        }
    }
}

impl AgentTelemetryRepository for AgentTelemetryRepositoryImpl {
    fn record_iteration(
        &self,
        cwd: &str,
        conversation_id: &str,
        agent_id: &str,
        agent_name: &str,
        acp_session_id: Option<&str>,
        mut iteration: IterationTelemetry,
    ) -> Result<(), String> {
        Self::validate_vault_and_id(cwd, conversation_id)?;
        let _guard = self.lock.lock().map_err(|e| e.to_string())?;

        let telemetry_path = Self::telemetry_path(cwd, conversation_id);
        let transcript_path = Self::transcript_path(cwd, conversation_id);
        let mut session = Self::load_or_create(cwd, conversation_id)?;

        session.agent_id = agent_id.to_string();
        session.agent_name = agent_name.to_string();
        if let Some(sid) = acp_session_id {
            session.acp_session_id = Some(sid.to_string());
        }

        // Correlacionar llamadas al MCP de Synapse ocurridas dentro del rango temporal de esta iteración
        let matched_mcp_ids: Vec<String> = session
            .mcp_calls
            .iter()
            .filter(|c| c.started_at >= iteration.started_at && c.started_at <= iteration.ended_at)
            .map(|c| c.id.clone())
            .collect();
        for id in matched_mcp_ids {
            if !iteration.mcp_call_ids.contains(&id) {
                iteration.mcp_call_ids.push(id);
            }
        }

        if iteration.effort.is_none() {
            iteration.effort = crate::domain::agent::agent_telemetry::infer_effort(
                iteration.model.as_deref(),
                iteration.mode.as_deref(),
            );
        }

        let vault_name = Self::vault_name(cwd);
        let git_branch = Self::detect_git_branch(cwd);

        let mut tags = vec![
            format!("vault:{vault_name}"),
            format!("agent:{agent_id}"),
            format!("session:{conversation_id}"),
        ];
        if let Some(ref m) = iteration.model {
            tags.push(format!("model:{m}"));
        }
        if let Some(ref m) = iteration.mode {
            tags.push(format!("mode:{m}"));
        }
        if let Some(ref e) = iteration.effort {
            tags.push(format!("effort:{e}"));
        }
        if let Some(ref b) = git_branch {
            tags.push(format!("branch:{b}"));
        }
        for t in &iteration.tags {
            if !tags.contains(t) {
                tags.push(t.clone());
            }
        }
        iteration.tags = tags.clone();

        let record = AgentUsageRecord {
            id: iteration.id.clone(),
            recorded_at: Self::now_ms(),
            started_at: iteration.started_at,
            ended_at: iteration.ended_at,
            duration_ms: iteration.duration_ms,
            agent_id: agent_id.to_string(),
            agent_name: agent_name.to_string(),
            model: iteration.model.clone(),
            mode: iteration.mode.clone(),
            effort: iteration.effort.clone(),
            vault_cwd: cwd.to_string(),
            vault_name,
            git_branch,
            conversation_id: conversation_id.to_string(),
            acp_session_id: session.acp_session_id.clone(),
            transcript_path: transcript_path.to_string_lossy().into_owned(),
            telemetry_path: telemetry_path.to_string_lossy().into_owned(),
            turn_index: iteration.turn_index,
            stop_reason: iteration.stop_reason.clone(),
            usage_used: iteration.usage_used,
            usage_size: iteration.usage_size,
            cost_amount: iteration.cost_amount,
            cost_delta: iteration.cost_delta,
            cost_currency: iteration.cost_currency.clone(),
            tool_calls_count: iteration.tool_calls.len() as u32,
            mcp_calls_count: iteration.mcp_call_ids.len() as u32,
            tags,
        };

        session.iterations.push(iteration);
        Self::save_session_telemetry(&telemetry_path, &session)?;
        self.append_global_usage(&record)
    }

    fn record_mcp_call(
        &self,
        cwd: &str,
        conversation_id: &str,
        call: McpCallTelemetry,
    ) -> Result<(), String> {
        Self::validate_vault_and_id(cwd, conversation_id)?;
        let _guard = self.lock.lock().map_err(|e| e.to_string())?;

        let telemetry_path = Self::telemetry_path(cwd, conversation_id);
        let mut session = Self::load_or_create(cwd, conversation_id)?;
        session.mcp_calls.push(call);
        Self::save_session_telemetry(&telemetry_path, &session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agent::ToolCallTelemetry;
    use serde_json::json;

    #[test]
    fn registra_mcp_correlaciona_iteracion_y_guarda_uso_global() {
        let vault = tempfile::tempdir().unwrap();
        let global_dir = tempfile::tempdir().unwrap();
        let global_usage = global_dir.path().join("agent-usage.jsonl");
        let repo = AgentTelemetryRepositoryImpl::new(global_usage.clone());
        let cwd = vault.path().to_str().unwrap();

        repo.record_mcp_call(
            cwd,
            "conv-1",
            McpCallTelemetry {
                id: "mcp-1500-1".into(),
                started_at: 1500,
                ended_at: 1540,
                duration_ms: 40,
                method: "tools/call".into(),
                tool_name: Some("read_note".into()),
                jsonrpc_id: Some(json!(1)),
                request: json!({"method": "tools/call", "params": {"name": "read_note"}}),
                response: json!({"result": {"content": []}}),
                is_error: false,
            },
        )
        .unwrap();

        repo.record_iteration(
            cwd,
            "conv-1",
            "claude-acp",
            "Claude",
            Some("acp-99"),
            IterationTelemetry {
                id: "iter-1000-1".into(),
                turn_index: 1,
                model: Some("claude-sonnet-4-high".into()),
                mode: Some("code".into()),
                effort: None,
                started_at: 1000,
                ended_at: 2000,
                duration_ms: 1000,
                stop_reason: "end_turn".into(),
                usage_used: Some(1200),
                usage_size: Some(200_000),
                cost_amount: Some(0.05),
                cost_delta: Some(0.05),
                cost_currency: Some("USD".into()),
                tool_calls: vec![ToolCallTelemetry {
                    tool_call_id: "tc-1".into(),
                    title: "read_note".into(),
                    kind: Some("read".into()),
                    status: Some("completed".into()),
                    started_at: Some(1490),
                    ended_at: Some(1550),
                    duration_ms: Some(60),
                }],
                mcp_call_ids: vec![],
                tags: vec!["custom:test".into()],
            },
        )
        .unwrap();

        // Verificar sidecar en la bóveda
        let sidecar_path = vault
            .path()
            .join(".synapse/agent-sessions/telemetry/conv-1.json");
        assert!(sidecar_path.exists());
        let session: AgentSessionTelemetry =
            serde_json::from_slice(&std::fs::read(&sidecar_path).unwrap()).unwrap();
        assert_eq!(session.mcp_calls.len(), 1);
        assert_eq!(session.iterations.len(), 1);
        assert_eq!(session.iterations[0].mcp_call_ids, vec!["mcp-1500-1"]);

        // Verificar registro global JSONL con referencias a ambos JSON
        let lines: Vec<String> = std::fs::read_to_string(&global_usage)
            .unwrap()
            .lines()
            .map(str::to_string)
            .collect();
        assert_eq!(lines.len(), 1);
        let rec: AgentUsageRecord = serde_json::from_str(&lines[0]).unwrap();
        assert_eq!(rec.model.as_deref(), Some("claude-sonnet-4-high"));
        assert_eq!(rec.effort.as_deref(), Some("high"));
        assert!(!rec.vault_name.is_empty());
        assert!(rec.tags.iter().any(|i| i == "agent:claude-acp"));
        assert!(rec.tags.iter().any(|i| i == "effort:high"));
        assert!(rec.tags.iter().any(|i| i == "custom:test"));
        assert_eq!(rec.tool_calls_count, 1);
        assert_eq!(rec.mcp_calls_count, 1);
        assert!(rec.transcript_path.ends_with(".synapse/agent-sessions/conv-1.json"));
        assert!(rec
            .telemetry_path
            .ends_with(".synapse/agent-sessions/telemetry/conv-1.json"));
    }
}
