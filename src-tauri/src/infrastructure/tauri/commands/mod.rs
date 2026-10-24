pub mod assets;
pub mod file_ops;
pub mod git;
pub mod plugins;
pub mod search;
pub mod session;
pub mod system;
pub mod vault;
pub mod workspace;

pub use assets::*;
pub use file_ops::*;
pub use git::*;
pub use plugins::*;
pub use search::*;
pub use session::*;
pub use system::*;
pub use vault::*;
pub use workspace::*;

// Re-exports for backwards compatibility
pub use crate::infrastructure::tauri::dto::*;
pub use crate::infrastructure::tauri::state::*;
