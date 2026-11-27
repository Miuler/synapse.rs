//! Los criterios con los que un tablero elige sus tareas, sin la sintaxis de ningún proveedor.

use serde::{Deserialize, Serialize};

/// En `assignee`, quien sincroniza.
pub const ME: &str = "@me";
/// En `iteration`, las iteraciones en curso el día de la consulta.
pub const CURRENT: &str = "@current";

/// Dentro de un criterio basta con cumplir uno de sus valores; entre criterios, todos.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct IssueFilter {
    #[serde(rename = "type")]
    pub types: Vec<String>,
    #[serde(rename = "state")]
    pub states: Vec<String>,
    #[serde(rename = "assignee")]
    pub assignees: Vec<String>,
    #[serde(rename = "iteration")]
    pub iterations: Vec<String>,
    #[serde(rename = "area")]
    pub areas: Vec<String>,
    #[serde(rename = "tag")]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Criterion {
    Type,
    State,
    Assignee,
    Iteration,
    Area,
    Tag,
}
