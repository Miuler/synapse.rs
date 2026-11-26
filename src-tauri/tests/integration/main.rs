//! Pruebas de integración en un solo binario: cada archivo es un módulo. Así se enlaza `app_lib`
//! una sola vez en lugar de una por archivo (cada enlazado de este crate tarda minutos).
//!
//! Correr un grupo: `cargo test --test integration mcp_tests::` (el prefijo es el nombre del módulo).

mod architecture;
mod baseline_measurements;
mod characterization_tests;
mod full_text_tests;
mod mcp_characterization_tests;
mod mcp_conversations_tests;
mod mcp_cross_vault_tests;
mod mcp_diagram_models_tests;
mod mcp_full_text_lines_tests;
mod mcp_search_hints_tests;
mod mcp_search_in_note_tests;
mod mcp_git_tools_tests;
mod mcp_render_diagram_tests;
mod mcp_tests;
mod navigation_tests;
