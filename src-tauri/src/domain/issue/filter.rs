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
    #[serde(rename = "type", skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<String>,
    #[serde(rename = "state", skip_serializing_if = "Vec::is_empty")]
    pub states: Vec<String>,
    #[serde(rename = "assignee", skip_serializing_if = "Vec::is_empty")]
    pub assignees: Vec<String>,
    #[serde(rename = "iteration", skip_serializing_if = "Vec::is_empty")]
    pub iterations: Vec<String>,
    #[serde(rename = "area", skip_serializing_if = "Vec::is_empty")]
    pub areas: Vec<String>,
    #[serde(rename = "tag", skip_serializing_if = "Vec::is_empty")]
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
