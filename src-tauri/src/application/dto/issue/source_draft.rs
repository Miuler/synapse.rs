//! El borrador de la configuración del origen de un tablero, tal como está abierto en la ventana.

use crate::domain::issue::filter::IssueFilter;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Cómo se eligen las tareas.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryMode {
    /// La consulta del proveedor por defecto: las propias.
    #[default]
    Mine,
    /// Por los criterios del filtro.
    Criteria,
    /// Con una consulta escrita.
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftColumn {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SourceDraft {
    pub provider: String,
    /// Id de la conexión elegida; vacío, la que cubra el ámbito.
    #[serde(default)]
    pub connection: String,
    pub scope: String,
    #[serde(default)]
    pub mode: QueryMode,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub filter: IssueFilter,
    /// Estados configurados por columna.
    #[serde(default)]
    pub states: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub columns: Vec<DraftColumn>,
    /// Ids de las tareas ya vinculadas en este ámbito.
    #[serde(default)]
    pub linked: Vec<String>,
    /// La fecha de hoy (`AAAA-MM-DD`): decide la iteración en curso.
    pub today: String,
}

/// Lo único que el agente puede cambiar del borrador.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftChange {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<IssueFilter>,
    /// Solo las columnas presentes cambian.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub states: Option<BTreeMap<String, Vec<String>>>,
}
