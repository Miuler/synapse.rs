//! Modelo común de las tareas de un gestor de issues (Azure DevOps, GitHub, Jira…): cada proveedor
//! traduce a y desde él, y el tablero no sabe de cuál vienen.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Estado normalizado: permite columnas, resúmenes y filtros iguales para todos los proveedores.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StateCategory {
    Todo,
    Doing,
    Done,
    Cancelled,
}

impl StateCategory {
    /// Respaldo por nombre, para cuando el proveedor no declara la categoría. Un nombre que no se
    /// reconoce queda por hacer: es preferible verlo pendiente que darlo por hecho.
    pub fn from_name(name: &str) -> Self {
        match name.trim().to_lowercase().as_str() {
            "in progress" | "doing" | "active" | "committed" | "resolved" => Self::Doing,
            "done" | "closed" | "completed" => Self::Done,
            "removed" | "cancelled" | "canceled" => Self::Cancelled,
            _ => Self::Todo,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IssueState {
    /// El nombre que usa el proveedor.
    pub name: String,
    pub category: StateCategory,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Person {
    pub name: String,
    /// Identificador en el proveedor (correo, usuario o id de cuenta).
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Issue {
    /// Identificador en el proveedor; para la aplicación es texto opaco.
    pub id: String,
    /// Referencia que se muestra: `AB#123`, `dueño/repo#5`, `PROJ-12`.
    pub key: String,
    pub url: Option<String>,
    #[serde(rename = "type")]
    pub issue_type: String,
    pub title: String,
    /// Markdown.
    pub description: Option<String>,
    pub state: IssueState,
    pub assignee: Option<Person>,
    pub labels: Vec<String>,
    pub updated_at: Option<String>,
    /// Versión leída: una escritura solo se admite sobre ella. Texto opaco.
    pub revision: Option<String>,
    /// Lo que no cabe en el modelo común.
    pub extra: BTreeMap<String, String>,
}

/// Clase de una entrada de actividad: lo que alguien escribió o lo que registró el proveedor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    Comment,
    History,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FieldChange {
    pub field: String,
    pub from: Option<String>,
    pub to: Option<String>,
}

/// Una entrada de la actividad de una tarea.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IssueEntry {
    /// Identificador en el proveedor: evita volver a enviar lo traído.
    pub id: String,
    pub kind: EntryKind,
    pub author: Option<Person>,
    pub at: String,
    /// Markdown.
    pub text: String,
    /// Campos que cambiaron, en las entradas del histórico.
    pub changes: Vec<FieldChange>,
}

/// La cuenta con la que entra una credencial.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Account {
    /// `None` si el servidor acepta la credencial sin decir de quién es.
    pub name: Option<String>,
}

/// Cambios de una tarea existente: solo se escribe lo presente.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct IssueChanges {
    pub title: Option<String>,
    /// Nombre de un estado del proveedor.
    pub state: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewIssue {
    #[serde(rename = "type")]
    pub issue_type: String,
    pub title: String,
    pub assignee: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::EntryKind;
    use super::StateCategory::{self, *};

    #[test]
    fn la_clase_de_una_entrada_viaja_como_comment_o_history() {
        let json = serde_json::to_string(&[EntryKind::Comment, EntryKind::History]).unwrap();
        assert_eq!(json, r#"["comment","history"]"#);
    }

    #[test]
    fn normaliza_los_estados_por_nombre_sin_distinguir_mayusculas() {
        let cases = [
            ("To Do", Todo),
            ("new", Todo),
            ("Proposed", Todo),
            ("In Progress", Doing),
            (" ACTIVE ", Doing),
            ("Resolved", Doing),
            ("Done", Done),
            ("Closed", Done),
            ("Completed", Done),
            ("Removed", Cancelled),
            ("Canceled", Cancelled),
        ];
        for (name, category) in cases {
            assert_eq!(StateCategory::from_name(name), category, "{name}");
        }
    }

    #[test]
    fn un_estado_desconocido_queda_por_hacer() {
        assert_eq!(StateCategory::from_name("Esperando a terceros"), Todo);
        assert_eq!(StateCategory::from_name(""), Todo);
    }
}
