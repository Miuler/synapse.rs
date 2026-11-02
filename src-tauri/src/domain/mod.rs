// TODO: `serde` sigue permitido en el dominio por ahora (excepción temporal de Synapse).
// La regla de `docs/arquitectura_cebolla.md` lo prohíbe: la serialización debe vivir en DTOs
// de `infrastructure` con mappers que consumen el valor. Duda abierta: migrar por etapas
// (empezando por los modelos más usados) sin pagar copias extra en las respuestas grandes
// (árbol de la bóveda, contenido de notas, resultados de búsqueda).

pub mod config;
pub mod graph;
pub mod mcp;
pub mod note;
pub mod pdf;
pub mod plugin;
pub mod search;
pub mod service;
pub mod vault;
pub mod version;
pub mod workspace;
