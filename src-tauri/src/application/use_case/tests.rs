use super::list_agents_use_case::ListAgentsUseCase;
use super::save_agent_profile_use_case::SaveAgentProfileUseCase;
use super::send_agent_prompt_use_case::SendAgentPromptUseCase;
use super::start_agent_session_use_case::StartAgentSessionUseCase;
use crate::application::dto::agent::*;
use crate::application::integration::service::{AgentCatalog, AgentInstaller, AgentRunner};
use crate::domain::agent::*;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct InMemoryAgentProfileRepositoryImpl(Arc<Mutex<Vec<AgentProfile>>>);

impl AgentProfileRepository for InMemoryAgentProfileRepositoryImpl {
    fn list(&self) -> Result<Vec<AgentProfile>, String> {
        Ok(self.0.lock().unwrap().clone())
    }
    fn save(&self, profile: AgentProfile) -> Result<(), String> {
        let mut all = self.0.lock().unwrap();
        all.retain(|p| p.id != profile.id);
        all.push(profile);
        Ok(())
    }
    fn delete(&self, id: &str) -> Result<(), String> {
        self.0.lock().unwrap().retain(|p| p.id != id);
        Ok(())
    }
}

#[derive(Clone)]
struct MockAgentCatalogImpl;

impl AgentCatalog for MockAgentCatalogImpl {
    async fn list(&self) -> Result<Vec<AgentProfile>, String> {
        Ok(vec![AgentProfile {
            id: "claude-acp".into(),
            name: "Claude".into(),
            source: AgentSource::Verified,
            description: None,
            version: Some("1.0.0".into()),
            installed: false,
            launch: None,
        }])
    }
}

#[derive(Clone, Default)]
struct MockAgentInstallerImpl(Arc<Mutex<Vec<String>>>);

impl AgentInstaller for MockAgentInstallerImpl {
    async fn ensure_installed(&self, agent_id: &str) -> Result<AgentLaunch, String> {
        self.0.lock().unwrap().push(agent_id.to_string());
        Ok(AgentLaunch {
            command: format!("/agents/{agent_id}"),
            ..Default::default()
        })
    }
}

#[derive(Clone, Default)]
struct MockAgentRunnerImpl(Arc<Mutex<Vec<String>>>);

impl AgentRunner for MockAgentRunnerImpl {
    async fn start(
        &self,
        agent_id: &str,
        launch: AgentLaunch,
        cwd: &str,
        _tools: Vec<ToolServer>,
        _instructions: Option<String>,
        _resume: Option<String>,
    ) -> Result<AgentSession, String> {
        self.0.lock().unwrap().push(launch.command);
        Ok(AgentSession {
            id: "s1".into(),
            agent_id: agent_id.into(),
            cwd: cwd.into(),
            ..Default::default()
        })
    }
    async fn prompt(&self, _session_id: &str, _blocks: Vec<PromptBlock>) -> Result<String, String> {
        Ok("end_turn".into())
    }
    async fn cancel(&self, _session_id: &str) -> Result<(), String> {
        Ok(())
    }
    async fn respond_permission(
        &self,
        _s: &str,
        _r: &str,
        _d: PermissionDecision,
    ) -> Result<(), String> {
        Ok(())
    }
    async fn set_mode(&self, _s: &str, _m: &str) -> Result<(), String> {
        Ok(())
    }
    async fn set_model(&self, _s: &str, _m: &str) -> Result<(), String> {
        Ok(())
    }
    async fn stop(&self, _session_id: &str) -> Result<(), String> {
        Ok(())
    }
    fn set_process_limit(&self, _agent_id: &str, _limit: usize) {}
}

fn custom(id: &str) -> SaveAgentProfileCommand {
    SaveAgentProfileCommand {
        id: id.into(),
        name: "Mío".into(),
        launch: AgentLaunch {
            command: "mi-agente".into(),
            ..Default::default()
        },
    }
}

#[tokio::test]
async fn lista_verificados_y_personalizados() {
    let repo = InMemoryAgentProfileRepositoryImpl::default();
    SaveAgentProfileUseCase::new(repo.clone())
        .execute(custom("propio"))
        .unwrap();
    let agents = ListAgentsUseCase::new(MockAgentCatalogImpl, repo)
        .execute(ListAgentsQuery)
        .await
        .unwrap();
    let ids: Vec<_> = agents.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(ids, ["claude-acp", "propio"]);
}

#[tokio::test]
async fn un_verificado_se_instala_antes_de_lanzarse() {
    let installer = MockAgentInstallerImpl::default();
    let runner = MockAgentRunnerImpl::default();
    let session = StartAgentSessionUseCase::new(
        InMemoryAgentProfileRepositoryImpl::default(),
        installer.clone(),
        runner.clone(),
    )
    .execute(StartAgentSessionCommand {
        agent_id: "claude-acp".into(),
        cwd: "/boveda".into(),
        tools: vec![],
        instructions: None,
        resume: None,
    })
    .await
    .unwrap();
    assert_eq!(session.cwd, "/boveda");
    assert_eq!(*installer.0.lock().unwrap(), ["claude-acp"]);
    assert_eq!(*runner.0.lock().unwrap(), ["/agents/claude-acp"]);
}

#[tokio::test]
async fn un_personalizado_se_lanza_con_su_comando() {
    let repo = InMemoryAgentProfileRepositoryImpl::default();
    SaveAgentProfileUseCase::new(repo.clone())
        .execute(custom("propio"))
        .unwrap();
    let installer = MockAgentInstallerImpl::default();
    let runner = MockAgentRunnerImpl::default();
    StartAgentSessionUseCase::new(repo, installer.clone(), runner.clone())
        .execute(StartAgentSessionCommand {
            agent_id: "propio".into(),
            cwd: "/b".into(),
            tools: vec![],
            instructions: None,
            resume: None,
        })
        .await
        .unwrap();
    assert!(installer.0.lock().unwrap().is_empty());
    assert_eq!(*runner.0.lock().unwrap(), ["mi-agente"]);
}

#[tokio::test]
async fn rechaza_un_mensaje_vacio() {
    let r = SendAgentPromptUseCase::new(MockAgentRunnerImpl::default())
        .execute(SendAgentPromptCommand {
            session_id: "s1".into(),
            blocks: vec![PromptBlock::Text { text: "  ".into() }],
        })
        .await;
    assert!(r.is_err());
}
