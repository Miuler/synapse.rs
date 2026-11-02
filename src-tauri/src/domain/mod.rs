// TODO: `serde` sigue permitido en el dominio por ahora (excepción temporal de Synapse).
// La regla de `docs/arquitectura_cebolla.md` lo prohíbe: la serialización debe vivir en DTOs
// de `infrastructure` con mappers que consumen el valor. Duda abierta: migrar por etapas
// (empezando por los modelos más usados) sin pagar copias extra en las respuestas grandes
// (árbol de la bóveda, contenido de notas, resultados de búsqueda).
pub mod events;
pub mod models;
pub mod repositories;
pub mod services;
pub mod value_objects;
