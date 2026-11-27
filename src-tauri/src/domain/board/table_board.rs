//! Convierte un tablero con tabla (el formato anterior) al de carpeta. Es una función pura: no
//! escribe nada. Las celdas se leen por su posición, como las muestra la interfaz, y lo que el formato
//! de carpeta no tiene dónde poner se conserva: las columnas propias como datos de cada tarea y el
//! resto del texto en el cuerpo del tablero.

use crate::domain::board::board_error::{BoardError, BoardErrorKind as Kind};
use crate::domain::board::board_file::{BoardColumn, BoardFile, BoardSource, blocks, value};
use crate::domain::board::task_file::{ActivityEntry, TaskFile, TaskSync, is_valid_task_id};
use crate::domain::issue::issue::EntryKind;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_yaml::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

/// La clave del frontmatter con el origen del formato anterior.
const SOURCE_KEY: &str = "azure_devops";
/// Datos de una tarea que una columna propia no puede ocupar.
const RESERVED: [&str; 6] = ["status", "start", "duration", "depends", "progress", "sync"];
/// Las columnas que escribe el formato anterior.
const COLUMNS: usize = 7;

/// Un tablero convertido: su archivo y el de cada tarea, en el orden de la tabla.
#[derive(Debug, Clone, PartialEq)]
pub struct Converted {
    pub board: BoardFile,
    pub tasks: Vec<(String, TaskFile)>,
}

#[derive(Deserialize)]
struct Link {
    id: Value,
    #[serde(default)]
    title: String,
    #[serde(default)]
    state: String,
    #[serde(default)]
    column: String,
    #[serde(default)]
    sent: Vec<String>,
    #[serde(default)]
    outside: bool,
}

fn bad(detail: impl Into<String>) -> BoardError {
    BoardError::new(Kind::Frontmatter, detail)
}

fn read<T: DeserializeOwned>(key: &str, found: Value) -> Result<T, BoardError> {
    serde_yaml::from_value(found).map_err(|e| bad(format!("{SOURCE_KEY}.{key}: {e}")))
}

/// El origen y, por tarea, lo que tenía en común con el proveedor.
fn source(
    stored: BTreeMap<String, Value>,
) -> Result<(BoardSource, HashMap<String, TaskSync>), BoardError> {
    let mut source = BoardSource {
        provider: SOURCE_KEY.into(),
        ..Default::default()
    };
    let mut links = HashMap::new();
    for (key, found) in stored {
        if found.is_null() {
            continue;
        }
        match key.as_str() {
            "project_url" => source.scope = read(&key, found)?,
            "connection" => source.connection = read(&key, found)?,
            "query" => source.query = read(&key, found)?,
            "filter" => source.filter = read(&key, found)?,
            "filter_query" => source.filter_query = read(&key, found)?,
            "work_item_type" => source.issue_type = read(&key, found)?,
            "assigned_to" => source.assignee = read(&key, found)?,
            "states" => source.states = read(&key, found)?,
            "links" => {
                for (task, link) in read::<BTreeMap<String, Link>>(&key, found)? {
                    // En el formato anterior el id es un número
                    let id = match link.id {
                        Value::Number(n) => n.to_string(),
                        other => read(&key, other)?,
                    };
                    let sync = TaskSync {
                        id,
                        rev: None,
                        title: link.title,
                        state: link.state,
                        column: link.column,
                        sent: link.sent,
                        outside: link.outside,
                    };
                    links.insert(task, sync);
                }
            }
            _ => {
                let kept = serde_json::to_value(&found).map_err(|e| bad(e.to_string()))?;
                // Un nombre del formato nuevo no puede aparecer dos veces
                let name = match key.as_str() {
                    "provider" | "scope" | "issue_type" | "assignee" => format!("legacy_{key}"),
                    _ => key,
                };
                source.extra.insert(name, kept);
            }
        }
    }
    Ok((source, links))
}

fn frontmatter(text: &str, board: &mut BoardFile) -> Result<HashMap<String, TaskSync>, BoardError> {
    let mut links = HashMap::new();
    let mut extra = Vec::new();
    board.default_view = "kanban".into();
    for (key, lines) in blocks(text) {
        let verbatim = || lines.join("\n");
        match key.as_str() {
            "format" => return Err(BoardError::new(Kind::NotTableBoard, verbatim())),
            "type" => {}
            "default_view" => {
                if let Ok(view) = value(&key, &lines) {
                    board.default_view = view;
                }
            }
            "columns" => board.columns = value::<Vec<BoardColumn>>(&key, &lines)?,
            "column_widths" => match value::<BTreeMap<String, Value>>(&key, &lines) {
                Ok(widths) => {
                    board.column_widths = widths
                        .into_iter()
                        .filter_map(|(id, w)| Some((id, w.as_f64().filter(|w| *w >= 1.0)?)))
                        .map(|(id, w)| (id, w.round() as u32))
                        .collect();
                }
                Err(_) => extra.push(verbatim()),
            },
            SOURCE_KEY => match value::<Value>(&key, &lines) {
                Ok(Value::Mapping(map)) => {
                    let map = read("", Value::Mapping(map))?;
                    let (found, synced) = source(map)?;
                    board.source = Some(found);
                    links = synced;
                }
                // Sin nada dentro no es un origen
                Ok(_) => extra.push(verbatim()),
                // Mal escrito: convertir lo dejaría sin sincronizar sin decirlo
                Err(e) => return Err(e),
            },
            // En el formato nuevo estas claves significan otra cosa
            "source" | "fields" => extra.push(format!("old_{}", verbatim())),
            _ => extra.push(verbatim()),
        }
    }
    board.extra_frontmatter = extra.join("\n").trim_end().to_string();
    Ok(links)
}

/// Las celdas de una fila; `\|` es una barra dentro de una celda.
fn cells(line: &str) -> Vec<String> {
    let line = line.trim();
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut previous = ' ';
    for c in line.chars() {
        if c == '|' && previous != '\\' {
            cells.push(std::mem::take(&mut cell));
        } else {
            cell.push(c);
        }
        previous = c;
    }
    cells.push(cell);
    // Lo de antes de la primera barra y, si la fila se cierra, lo de después de la última
    if cells.last().is_some_and(|c| c.trim().is_empty()) {
        cells.pop();
    }
    cells
        .into_iter()
        .skip(1)
        .map(|c| c.trim().replace("\\|", "|"))
        .collect()
}

fn is_separator(line: &str) -> bool {
    let cells = cells(line);
    !cells.is_empty()
        && cells
            .iter()
            .all(|c| c.contains('-') && c.chars().all(|c| c == '-' || c == ':'))
}

fn is_date(value: &str) -> bool {
    value.len() == 10
        && value.char_indices().all(|(i, c)| match i {
            4 | 7 => c == '-',
            _ => c.is_ascii_digit(),
        })
}

/// El número de una celda como lo entiende la interfaz y si la celda no dice nada más que eso.
fn number(cell: &str, suffix: char) -> (Option<u32>, bool) {
    let digits: String = cell
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect();
    let exact = !digits.is_empty() && cell.strip_suffix(suffix).unwrap_or(cell) == digits;
    (digits.parse().ok(), exact)
}

/// Guarda un dato que no tiene campo propio, con el nombre de su columna.
fn keep(task: &mut TaskFile, column: &str, value: &str) {
    if value.is_empty() {
        return;
    }
    let name = column.replace(':', " -");
    let name = name.trim();
    let field = match name {
        "type" => Some(&mut task.issue_type),
        "assignee" => Some(&mut task.assignee),
        "key" => Some(&mut task.key),
        "url" => Some(&mut task.url),
        _ => None,
    };
    match field {
        Some(field @ None) => *field = Some(value.to_string()),
        taken => {
            let name = if taken.is_some() || RESERVED.contains(&name) {
                format!("{name} (tabla)")
            } else {
                name.to_string()
            };
            task.extra.push((name, value.to_string()));
        }
    }
}

fn task(headers: &[String], row: &[String]) -> TaskFile {
    let header = |i: usize| match headers.get(i) {
        Some(name) if !name.is_empty() => name.clone(),
        _ => format!("column {}", i + 1),
    };
    let mut task = TaskFile {
        title: if row[1].is_empty() {
            "Sin título".into()
        } else {
            row[1].clone()
        },
        status: row[2].clone(),
        depends_on: if row[5].is_empty() {
            "-".into()
        } else {
            row[5].clone()
        },
        ..Default::default()
    };
    if is_date(&row[3]) {
        task.start = row[3].clone();
    } else if row[3] != "-" {
        keep(&mut task, &header(3), &row[3]);
    }
    let (days, exact) = number(&row[4], 'd');
    task.duration = days.unwrap_or(1).max(1);
    if !exact || days == Some(0) {
        keep(&mut task, &header(4), &row[4]);
    }
    let (percent, exact) = number(&row[6], '%');
    task.progress = percent.unwrap_or(0).min(100) as u8;
    if !exact || percent.is_some_and(|p| p > 100) {
        keep(&mut task, &header(6), &row[6]);
    }
    for (i, cell) in row.iter().enumerate().skip(COLUMNS) {
        keep(&mut task, &header(i), cell);
    }
    task
}

/// Las filas de la tabla de tareas (la primera) y las demás líneas.
fn table<'a>(lines: &[&'a str]) -> (Vec<(String, TaskFile)>, Vec<&'a str>) {
    let mut headers: Option<Vec<String>> = None;
    let mut tasks = Vec::new();
    let mut other = Vec::new();
    let is_row = |line: &str| line.trim_start().starts_with('|');
    let mut i = 0;
    // Filas vistas, contando como la interfaz: de ahí sale el id de la que no lo tiene
    let mut seen = 0;
    while i < lines.len() {
        if !is_row(lines[i]) {
            other.push(lines[i]);
            i += 1;
            continue;
        }
        let end = i + lines[i..].iter().take_while(|l| is_row(l)).count();
        let block = &lines[i..end];
        let titled = block.len() > 1 && is_separator(block[1]);
        let rows = match &headers {
            None => {
                headers = Some(cells(block[0]));
                seen = block.len().min(2);
                &block[block.len().min(2)..]
            }
            // Otra tabla, con su cabecera: es texto del tablero
            Some(_) if titled => {
                other.extend_from_slice(block);
                &block[..0]
            }
            // La misma tabla, partida por un hueco
            Some(_) => block,
        };
        for line in rows {
            let row = cells(line);
            if row.len() < COLUMNS {
                other.push(*line);
            } else {
                let id = if row[0].is_empty() {
                    format!("t-{}", seen - 1)
                } else {
                    row[0].clone()
                };
                tasks.push((id, task(headers.as_deref().unwrap_or_default(), &row)));
            }
            seen += 1;
        }
        i = end;
    }
    (tasks, other)
}

fn is_heading(line: &str, marks: &str, title: Option<&str>) -> bool {
    line.strip_prefix(marks).is_some_and(|rest| {
        rest.starts_with(char::is_whitespace) && title.is_none_or(|title| rest.trim() == title)
    })
}

/// `` - `fecha` — texto ``
fn comment(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix('-')?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let (at, rest) = rest.trim_start().strip_prefix('`')?.split_once('`')?;
    let rest = rest.trim_start();
    let text = rest.strip_prefix('—').or_else(|| rest.strip_prefix('-'))?;
    let text = text.starts_with(char::is_whitespace).then(|| text.trim())?;
    (!text.is_empty()).then_some((at.trim(), text))
}

/// Los comentarios de una tarea; lo que no lo sea y no siga a uno queda en una sección aparte.
fn comments(task: &mut TaskFile, lines: &[&str]) {
    let mut loose = Vec::new();
    for line in lines {
        if let Some((at, text)) = comment(line) {
            task.activity.push(ActivityEntry {
                at: at.to_string(),
                kind: EntryKind::Comment,
                author: None,
                id: None,
                edited: false,
                text: text.to_string(),
            });
        } else if !line.trim().is_empty() {
            match task.activity.last_mut() {
                Some(entry) => {
                    entry.text.push('\n');
                    entry.text.push_str(line.strip_prefix("  ").unwrap_or(line));
                }
                None => loose.push(*line),
            }
        }
    }
    if !loose.is_empty() {
        task.sections = format!("## Comentarios (tabla)\n\n{}", loose.join("\n"));
    }
}

pub fn convert(content: &str) -> Result<Converted, BoardError> {
    let content = content.replace("\r\n", "\n");
    let (front, body) = content
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---"))
        .unwrap_or(("", &content));
    let mut board = BoardFile::default();
    let mut links = frontmatter(front, &mut board)?;

    let lines: Vec<&str> = body.lines().collect();
    let start = lines
        .iter()
        .position(|l| is_heading(l, "##", Some("Comentarios")))
        .unwrap_or(lines.len());
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, l)| l.starts_with("## ") && !is_heading(l, "##", Some("Comentarios")))
        .map_or(lines.len(), |(i, _)| i);
    let (mut tasks, mut other) = table(&lines[..start]);

    // Un id que no sirva como nombre de archivo se corrige en el tablero antes de convertir
    let mut taken = HashSet::new();
    let invalid: Vec<&str> = tasks
        .iter()
        .map(|(id, _)| id.as_str())
        .filter(|id| !is_valid_task_id(id) || !taken.insert(id.to_ascii_lowercase()))
        .collect();
    if !invalid.is_empty() {
        return Err(BoardError::new(Kind::TaskId, invalid.join(", ")));
    }

    // Cada `### id` con sus líneas
    let mut sections: Vec<(Option<&str>, Vec<&str>)> = vec![(None, Vec::new())];
    for line in lines.get(start + 1..end).unwrap_or_default() {
        if is_heading(line, "###", None) {
            sections.push((Some(line[3..].trim()), vec![line]));
        } else if !is_heading(line, "##", Some("Comentarios"))
            && let Some(section) = sections.last_mut()
        {
            section.1.push(line);
        }
    }
    let mut found: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut orphans = Vec::new();
    for (id, lines) in sections {
        match id {
            Some(id) if tasks.iter().any(|(task, _)| task == id) => {
                found.entry(id).or_default().extend(&lines[1..]);
            }
            _ => orphans.extend(lines),
        }
    }
    for (id, task) in &mut tasks {
        if let Some(lines) = found.get(id.as_str()) {
            comments(task, lines);
        }
        task.sync = links.remove(id);
    }
    if orphans.iter().any(|l| !l.trim().is_empty()) {
        other.extend(["", "## Comentarios", ""]);
        other.extend(orphans);
    }
    other.extend_from_slice(&lines[end..]);

    // `## Order` es del formato nuevo
    let body: Vec<&str> = other
        .into_iter()
        .map(|l| {
            if l.trim_end() == "## Order" {
                "## Order (tabla)"
            } else {
                l
            }
        })
        .collect();
    board.body = body.join("\n").trim_matches('\n').to_string();
    board.order = tasks.iter().map(|(id, _)| id.clone()).collect();
    Ok(Converted { board, tasks })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::board::{board_file, task_file};

    const BOARD: &str = r#"---
type: board
default_view: table
columns:
  - { id: todo, label: "Por \"hacer\"", color: "var(--text-muted)" }
  - { id: doing, label: "En progreso", color: "var(--accent)" }
column_widths:
  todo: 320
azure_devops:
  project_url: https://dev.azure.com/org/Proyecto
  work_item_type: Bug
  assigned_to: ana@example.test
  query: SELECT [System.Id] FROM WorkItems
  filter:
    type: [Bug]
  states:
    todo: [To Do, New]
  clave_propia: se conserva
  links:
    ado-7:
      id: 7
      title: Uno
      state: To Do
      column: todo
      sent: ["2026-01-02T10:00:00"]
      outside: true
tags:
  - mio
# apunte
---

Texto antes de la tabla.

| id | Tarea | Estado | Inicio | Duración | Depende | Progreso | Prioridad | type |
|---|---|---|---|---|---|---|---|---|
| ado-7 | Uno \| dos | todo | 2026-01-01 | 3d | - | 40% | alta | Epic |
| t2 | Dos | doing | pronto | 2 semanas | ado-7 | 120% |  |  |

| otra | tabla |
|---|---|
| a | b |

## Comentarios

### ado-7
- `2026-01-02T10:00:00` — enviado
- `2026-01-03T10:00:00` — sin enviar
  segunda línea

### borrada
- `2026-01-04T10:00:00` — de una tarea que ya no está

## Notas

fin
"#;

    #[test]
    fn conserva_configuracion_orden_consulta_y_vinculos() {
        let Converted { board, tasks } = convert(BOARD).unwrap();
        assert_eq!(board.default_view, "table");
        assert_eq!(board.columns[0].label, "Por \"hacer\"");
        assert_eq!(board.column_widths["todo"], 320);
        assert_eq!(board.order, ["ado-7", "t2"]);
        assert_eq!(board.extra_frontmatter, "tags:\n  - mio\n# apunte");
        let source = board.source.unwrap();
        assert_eq!(source.provider, "azure_devops");
        assert_eq!(source.scope, "https://dev.azure.com/org/Proyecto");
        assert_eq!(source.connection, "");
        assert_eq!(source.issue_type.as_deref(), Some("Bug"));
        assert_eq!(source.assignee.as_deref(), Some("ana@example.test"));
        assert_eq!(
            source.query.as_deref(),
            Some("SELECT [System.Id] FROM WorkItems")
        );
        assert_eq!(source.filter.unwrap().types, ["Bug"]);
        assert_eq!(source.states["todo"], ["To Do", "New"]);
        assert_eq!(source.extra["clave_propia"], "se conserva");
        assert!(!source.extra.contains_key("links"));
        let sync = tasks[0].1.sync.as_ref().unwrap();
        assert_eq!((sync.id.as_str(), sync.outside), ("7", true));
        assert_eq!(sync.sent, ["2026-01-02T10:00:00"]);
        assert!(tasks[1].1.sync.is_none());
    }

    #[test]
    fn lleva_cada_fila_y_sus_comentarios_a_su_tarea() {
        let tasks = convert(BOARD).unwrap().tasks;
        let first = &tasks[0].1;
        assert_eq!(first.title, "Uno | dos");
        assert_eq!((first.duration, first.progress), (3, 40));
        assert_eq!(first.issue_type.as_deref(), Some("Epic"));
        assert_eq!(first.extra, [("Prioridad".to_string(), "alta".to_string())]);
        let texts: Vec<&str> = first.activity.iter().map(|a| a.text.as_str()).collect();
        assert_eq!(texts, ["enviado", "sin enviar\nsegunda línea"]);
        assert_eq!(first.activity[0].at, "2026-01-02T10:00:00");
        // Lo que no cabe en su campo se conserva con el nombre de la columna
        let second = &tasks[1].1;
        assert_eq!((second.start.as_str(), second.duration), ("", 2));
        assert_eq!(
            (second.progress, second.depends_on.as_str()),
            (100, "ado-7")
        );
        let kept: Vec<(&str, &str)> = second
            .extra
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        assert_eq!(
            kept,
            [
                ("Inicio", "pronto"),
                ("Duración", "2 semanas"),
                ("Progreso", "120%")
            ]
        );
    }

    #[test]
    fn el_resto_del_texto_queda_en_el_tablero() {
        let body = convert(BOARD).unwrap().board.body;
        for kept in [
            "Texto antes de la tabla.",
            "| otra | tabla |\n|---|---|\n| a | b |",
            "### borrada\n- `2026-01-04T10:00:00` — de una tarea que ya no está",
            "## Notas\n\nfin",
        ] {
            assert!(body.contains(kept), "falta {kept:?} en {body:?}");
        }
        assert!(!body.contains("ado-7"));
    }

    #[test]
    fn lo_convertido_se_vuelve_a_leer_igual() {
        let converted = convert(BOARD).unwrap();
        let written = board_file::serialize(&converted.board).unwrap();
        assert_eq!(board_file::parse(&written).unwrap(), converted.board);
        for (_, task) in converted.tasks {
            assert_eq!(
                task_file::parse(&task_file::serialize(&task)).unwrap(),
                task
            );
        }
    }

    #[test]
    fn un_tablero_corriente_no_deja_texto_suelto() {
        let table = "---\ntype: board\n---\n\n| id | Tarea | Estado | Inicio | Duración | Depende | Progreso |\n|---|---|---|---|---|---|---|\n| t1 | A | todo | 2026-01-01 | 1d | - | 0% |\n\n## Comentarios\n\n### t1\n- `2026-01-01T00:00:00` — hola\n";
        let converted = convert(table).unwrap();
        assert_eq!(converted.board.body, "");
        assert!(converted.board.source.is_none());
        assert!(converted.tasks[0].1.extra.is_empty());
    }

    #[test]
    fn no_convierte_ids_que_no_sirven_de_archivo() {
        let table = "| id | T | E | I | D | De | P |\n|-|-|-|-|-|-|-|\n| a b | x | | | | | |\n| t1 | x | | | | | |\n| T1 | x | | | | | |\n| index | x | | | | | |\n";
        let error = convert(table).unwrap_err();
        assert_eq!(error.kind, Kind::TaskId);
        assert_eq!(error.detail, "a b, T1, index");
    }

    #[test]
    fn no_convierte_lo_ya_convertido_ni_un_origen_mal_escrito() {
        let converted = "---\ntype: board\nformat: 2\n---\n";
        assert_eq!(convert(converted).unwrap_err().kind, Kind::NotTableBoard);
        let broken = "---\ntype: board\nazure_devops:\n  project_url: [\n---\n";
        assert_eq!(convert(broken).unwrap_err().kind, Kind::Frontmatter);
    }
}
