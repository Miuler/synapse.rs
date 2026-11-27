//! Lo que un proveedor declara de sí mismo: las diferencias se muestran, no se disimulan.

use serde::Serialize;
use std::collections::BTreeMap;

/// Cómo se cambia el estado de una tarea.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StateChange {
    /// Se escribe el campo.
    Field,
    /// Abierta o cerrada; las columnas intermedias son etiquetas.
    OpenClosed,
    /// Solo por las transiciones que permite el flujo.
    Transition,
}

/// Con qué se detecta que una tarea cambió desde que se leyó.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictCheck {
    Revision,
    UpdatedAt,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Capabilities {
    pub create: bool,
    pub comment: bool,
    pub state_change: StateChange,
    pub conflict_check: ConflictCheck,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProviderDescriptor {
    /// Valor de `provider` en la configuración y en los tableros.
    pub id: String,
    pub name: String,
    /// Lenguaje de sus consultas (WIQL, JQL…).
    pub query_language: String,
    pub default_query: String,
    pub default_issue_type: String,
    /// Estados de cada columna habitual: el primero es el que se envía, todos valen al traer.
    pub default_states: BTreeMap<String, Vec<String>>,
    pub capabilities: Capabilities,
}
