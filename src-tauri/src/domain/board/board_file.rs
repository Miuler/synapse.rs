//! El archivo de un tablero en formato de carpeta: la configuración en el frontmatter y el orden de
//! las tareas en el cuerpo. Nada de lo propio de una tarea va aquí.
//!
//! La marca `format: 2` lo distingue del formato anterior (una tabla), que sigue leyendo la interfaz
//! hasta que se convierte (`table_board`).

use crate::domain::board::board_error::{BoardError, BoardErrorKind as Kind};
use crate::domain::board::task_file::is_valid_task_id;
use crate::domain::issue::filter::IssueFilter;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

pub const FORMAT: u32 = 2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoardColumn {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub color: String,
}

/// De dónde salen las tareas del tablero.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BoardSource {
    pub provider: String,
    /// Vacía, la que cubra el ámbito.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub connection: String,
    /// Dirección del proyecto o repositorio.
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<IssueFilter>,
    /// La consulta que salió del filtro.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    /// Estados remotos de cada columna.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub states: BTreeMap<String, Vec<String>>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// Qué datos se ven en cada vista; vacío, los habituales.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BoardFields {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub card: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub table: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BoardFile {
    pub default_view: String,
    pub columns: Vec<BoardColumn>,
    pub column_widths: BTreeMap<String, u32>,
    pub source: Option<BoardSource>,
    pub fields: BoardFields,
    /// Bloques del frontmatter que no se interpretan, tal cual.
    pub extra_frontmatter: String,
    /// Identificadores de las tareas, en el orden del tablero.
    pub order: Vec<String>,
    /// El resto del cuerpo, tal cual.
    pub body: String,
}

const KNOWN: [&str; 7] = [
    "type",
    "format",
    "default_view",
    "columns",
    "column_widths",
    "source",
    "fields",
];

/// El frontmatter partido por sus claves de primer nivel, cada una con sus líneas. Un comentario de
/// primer nivel forma su propio bloque, sin clave: así se conserva aunque siga a un bloque que se
/// vuelve a escribir.
pub(crate) fn blocks(frontmatter: &str) -> Vec<(String, Vec<&str>)> {
    let mut blocks: Vec<(String, Vec<&str>)> = Vec::new();
    // El bloque con clave al que pertenecen las líneas sangradas
    let mut current = None;
    for line in frontmatter.lines() {
        if line.starts_with('#') {
            blocks.push((String::new(), vec![line]));
            continue;
        }
        let key = line
            .split_once(':')
            .map(|(key, _)| key)
            .filter(|_| !line.starts_with([' ', '\t', '-']));
        match (key, current) {
            (None, Some(i)) => {
                let block: &mut (String, Vec<&str>) = &mut blocks[i];
                block.1.push(line);
            }
            (key, _) => {
                current = Some(blocks.len());
                blocks.push((key.unwrap_or_default().trim().to_string(), vec![line]));
            }
        }
    }
    blocks
}

pub(crate) fn value<T: DeserializeOwned>(key: &str, lines: &[&str]) -> Result<T, BoardError> {
    let bad = |e: String| BoardError::new(Kind::Frontmatter, format!("{key}: {e}"));
    let mut map: BTreeMap<String, serde_yaml::Value> =
        serde_yaml::from_str(&lines.join("\n")).map_err(|e| bad(e.to_string()))?;
    let found = map.remove(key).ok_or_else(|| bad("vacío".into()))?;
    serde_yaml::from_value(found).map_err(|e| bad(e.to_string()))
}

pub fn parse(content: &str) -> Result<BoardFile, BoardError> {
    let content = content.replace("\r\n", "\n");
    let not_a_board = || BoardError::new(Kind::NotFolderBoard, "");
    let rest = content.strip_prefix("---\n").ok_or_else(not_a_board)?;
    let (frontmatter, body) = rest.split_once("\n---").ok_or_else(not_a_board)?;

    let blocks = blocks(frontmatter);
    let find = |name: &str| blocks.iter().find(|(key, _)| key == name).map(|(_, l)| l);
    let is_board =
        find("type").is_some_and(|l| value::<String>("type", l).is_ok_and(|t| t == "board"));
    let format = find("format").and_then(|l| value::<u32>("format", l).ok());
    if !is_board || format != Some(FORMAT) {
        return Err(not_a_board());
    }

    let mut board = BoardFile::default();
    let mut extra = Vec::new();
    for (key, lines) in &blocks {
        match key.as_str() {
            "default_view" => board.default_view = value(key, lines)?,
            "columns" => board.columns = value(key, lines)?,
            "column_widths" => board.column_widths = value(key, lines)?,
            "source" => board.source = Some(value(key, lines)?),
            "fields" => board.fields = value(key, lines)?,
            key if KNOWN.contains(&key) => {}
            _ => extra.push(lines.join("\n")),
        }
    }
    board.extra_frontmatter = extra.join("\n").trim_end().to_string();

    let mut seen = HashSet::new();
    let mut other = Vec::new();
    let mut ordering = false;
    for line in body.lines() {
        if line.starts_with("## ") {
            ordering = line.trim_end() == "## Order";
            if ordering {
                continue;
            }
        }
        if !ordering {
            other.push(line);
        } else if let Some(id) = line.strip_prefix("- ").map(str::trim)
            && is_valid_task_id(id)
            // Los nombres de archivo no distinguen mayúsculas en todos los sistemas
            && seen.insert(id.to_ascii_lowercase())
        {
            board.order.push(id.to_string());
        }
    }
    board.body = other.join("\n").trim_matches('\n').to_string();
    Ok(board)
}

/// Un texto entre comillas que vale como YAML.
fn quoted(value: &str) -> String {
    serde_json::to_string(value).expect("un texto siempre se puede escribir")
}

fn yaml<T: Serialize>(key: &str, value: &T) -> Result<String, BoardError> {
    serde_yaml::to_string(&BTreeMap::from([(key, value)]))
        .map(|text| text.trim_end().to_string())
        .map_err(|e| BoardError::new(Kind::Frontmatter, format!("{key}: {e}")))
}

pub fn serialize(board: &BoardFile) -> Result<String, BoardError> {
    let view = match board.default_view.as_str() {
        view @ ("table" | "gantt") => view,
        _ => "kanban",
    };
    let mut lines = vec![
        "---".to_string(),
        "type: board".to_string(),
        format!("format: {FORMAT}"),
        format!("default_view: {view}"),
    ];
    if !board.columns.is_empty() {
        lines.push("columns:".to_string());
        lines.extend(board.columns.iter().map(|c| {
            format!(
                "  - {{ id: {}, label: {}, color: {} }}",
                quoted(&c.id),
                quoted(&c.label),
                quoted(&c.color)
            )
        }));
    }
    if !board.column_widths.is_empty() {
        lines.push(yaml("column_widths", &board.column_widths)?);
    }
    if let Some(source) = &board.source {
        lines.push(yaml("source", source)?);
    }
    if board.fields != BoardFields::default() {
        lines.push(yaml("fields", &board.fields)?);
    }
    if !board.extra_frontmatter.is_empty() {
        lines.push(board.extra_frontmatter.clone());
    }
    lines.push("---".to_string());

    let mut sections = Vec::new();
    if !board.body.is_empty() {
        sections.push(board.body.clone());
    }
    let order: Vec<String> = board
        .order
        .iter()
        .map(|id| {
            if is_valid_task_id(id) {
                Ok(format!("- {id}"))
            } else {
                Err(BoardError::new(Kind::TaskId, id))
            }
        })
        .collect::<Result<_, _>>()?;
    sections.push(
        format!("## Order\n\n{}", order.join("\n"))
            .trim_end()
            .to_string(),
    );
    Ok(format!(
        "{}\n\n{}\n",
        lines.join("\n"),
        sections.join("\n\n")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn some(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| v.to_string()).collect()
    }

    #[test]
    fn un_tablero_completo_va_y_vuelve_igual() {
        let board = BoardFile {
            default_view: "table".into(),
            columns: vec![BoardColumn {
                id: "todo".into(),
                label: "Por \"hacer\": #1".into(),
                color: "var(--accent)".into(),
            }],
            column_widths: BTreeMap::from([("todo".into(), 320)]),
            source: Some(BoardSource {
                provider: "azure_devops".into(),
                connection: "trabajo".into(),
                scope: "https://dev.azure.com/org/Proyecto".into(),
                query: Some("SELECT [System.Id]\nWHERE x = 'a: #b'".into()),
                filter: Some(IssueFilter {
                    assignees: some(&["@me"]),
                    iterations: some(&["@current", "P\\Sprint 1"]),
                    ..Default::default()
                }),
                filter_query: Some("SELECT 1".into()),
                issue_type: Some("Bug".into()),
                assignee: None,
                states: BTreeMap::from([("done".into(), some(&["Cerrado"]))]),
                extra: BTreeMap::from([("mia".into(), serde_json::json!({"x": 1}))]),
            }),
            fields: BoardFields {
                card: some(&["key", "type"]),
                table: Vec::new(),
            },
            extra_frontmatter: "# mi comentario\ntags:\n  - trabajo".into(),
            order: some(&["ado-1", "t1"]),
            body: "# Apuntes\n\n## Otra sección\ntexto".into(),
        };
        let written = serialize(&board).unwrap();
        assert!(written.contains("format: 2\n"), "{written}");
        assert!(written.ends_with("## Order\n\n- ado-1\n- t1\n"));
        // Un criterio vacío no se escribe
        assert!(!written.contains("tag: []"));
        let read = parse(&written).unwrap();
        assert_eq!(read, board, "\n{written}");
        assert_eq!(serialize(&read).unwrap(), written);
    }

    #[test]
    fn vacio_y_con_el_orden_repetido_o_mal_escrito() {
        let text = "---\r\ntype: board\r\nformat: 2\r\n---\r\n\r\n## Order\r\n\r\n- t1\r\n- T1\r\n- ../x\r\n- t2\r\n";
        let board = parse(text).unwrap();
        assert_eq!(board.order, some(&["t1", "t2"]));
        let written = serialize(&BoardFile::default()).unwrap();
        assert_eq!(parse(&written).unwrap().order, Vec::<String>::new());
    }

    #[test]
    fn el_formato_anterior_no_se_toma_por_este() {
        for text in [
            "---\ntype: board\n---\n| id |\n",
            "---\ntype: note\nformat: 2\n---\n",
            "# nota",
        ] {
            assert_eq!(
                parse(text).unwrap_err().kind,
                Kind::NotFolderBoard,
                "{text}"
            );
        }
    }

    #[test]
    fn un_bloque_roto_no_se_reescribe_vacio() {
        let text = "---\ntype: board\nformat: 2\nsource: [roto\n---\n";
        assert_eq!(parse(text).unwrap_err().kind, Kind::Frontmatter);
        let bad = BoardFile {
            order: some(&["../x"]),
            ..Default::default()
        };
        assert_eq!(serialize(&bad).unwrap_err().kind, Kind::TaskId);
    }
}
