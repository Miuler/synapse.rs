//! Casos de uso: uno por intención de negocio, con un único `execute`.

pub mod cancel_agent_prompt_use_case;
pub mod change_agent_mode_use_case;
pub mod change_agent_model_use_case;
pub mod delete_agent_profile_use_case;
pub mod delete_agent_transcript_use_case;
pub mod delete_all_agent_transcripts_use_case;
pub mod install_agent_use_case;
pub mod list_agent_transcripts_use_case;
pub mod list_agents_use_case;
pub mod load_agent_transcript_use_case;
pub mod open_agent_transcript_use_case;
pub mod respond_permission_use_case;
pub mod save_agent_profile_use_case;
pub mod save_agent_transcript_use_case;
pub mod send_agent_prompt_use_case;
pub mod set_agent_process_limit_use_case;
pub mod start_agent_session_use_case;
pub mod stop_agent_session_use_case;

#[cfg(test)]
mod tests;
