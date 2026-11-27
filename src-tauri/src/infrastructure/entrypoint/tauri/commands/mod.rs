pub mod appearance;
pub mod assets;
pub mod docx;
pub mod file_ops;
pub mod git;
pub mod mcp;
pub mod plugins;
pub mod search;
pub mod session;
pub mod system;
pub mod updates;
pub mod vault;
pub mod boards;
pub mod issues;
pub mod workspace;

pub use appearance::*;
pub use assets::*;
pub use docx::*;
pub use file_ops::*;
pub use git::*;
pub use mcp::*;
pub use plugins::*;
pub use search::*;
pub use session::*;
pub use system::*;
pub use updates::*;
pub use vault::*;
pub use boards::*;
pub use issues::*;
pub use workspace::*;

// Re-exports for backwards compatibility
pub use crate::infrastructure::config::state::*;
pub use crate::infrastructure::entrypoint::tauri::dto::*;
