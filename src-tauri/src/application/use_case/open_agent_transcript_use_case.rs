use crate::application::dto::agent::OpenAgentTranscriptCommand;
use crate::application::service::system_shell::SystemShell;
use crate::domain::agent::AgentTranscriptRepository;

/// Abre el archivo crudo de una conversación con la aplicación por defecto del sistema.
#[derive(Clone)]
pub struct OpenAgentTranscriptUseCase<R: AgentTranscriptRepository, S: SystemShell> {
    transcripts: R,
    shell: S,
}

impl<R: AgentTranscriptRepository, S: SystemShell> OpenAgentTranscriptUseCase<R, S> {
    pub fn new(transcripts: R, shell: S) -> Self {
        Self { transcripts, shell }
    }

    pub fn execute(&self, command: OpenAgentTranscriptCommand) -> Result<(), String> {
        let (dir, file) = self.transcripts.location(&command.cwd, &command.id)?;
        self.shell.open_with_default_app(&dir, &file)
    }
}
