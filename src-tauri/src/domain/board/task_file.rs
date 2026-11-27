//! El archivo de una tarea:
//!
//! ```markdown
//! # Título
//!
//! - status: doing
//! - sync: {"id":"123","title":"…","state":"In Progress","column":"doing"}
//!
//! ## Description
//!
//! > El texto, como cita para que nada de él parezca parte del formato.
//!
//! ## Activity
//!
//! - `2026-11-26T10:00:00` comment · Ana · id:987
//!   Primera línea.
//! ```

use crate::domain::board::board_error::{BoardError, BoardErrorKind};
use crate::domain::issue::issue::EntryKind;
use serde::{Deserialize, Serialize};

/// Lo que un tablero y su proveedor tenían en común la última vez que se sincronizó.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TaskSync {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    pub title: String,
    pub state: String,
    pub column: String,
    /// Fechas de los comentarios ya enviados.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sent: Vec<String>,
    /// Ya no la devuelve la consulta del tablero.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub outside: bool,
    /// Huella de la última descripción que ambos lados tenían.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// El tipo que había antes de cambiarlo aquí: mientras difiera, traer no lo pisa.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub issue_type: Option<String>,
    /// Como `type`, para el asignado; vacío, no tenía.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivityEntry {
    pub at: String,
    pub kind: EntryKind,
    #[serde(default)]
    pub author: Option<String>,
    /// Identificador en el proveedor.
    #[serde(default)]
    pub id: Option<String>,
    /// Cambiada a mano: el proveedor sigue teniendo la original.
    #[serde(default)]
    pub edited: bool,
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TaskFile {
    pub title: String,
    /// Columna; vacía, la primera del tablero.
    pub status: String,
    #[serde(rename = "type")]
    pub issue_type: Option<String>,
    pub assignee: Option<String>,
    pub key: Option<String>,
    pub url: Option<String>,
    pub start: String,
    /// Días.
    pub duration: u32,
    pub depends_on: String,
    pub progress: u8,
    pub sync: Option<TaskSync>,
    /// Datos que no se interpretan, en su orden.
    pub extra: Vec<(String, String)>,
    /// Markdown.
    pub description: String,
    pub activity: Vec<ActivityEntry>,
    /// Texto antes de la primera sección que no es un dato.
    pub preamble: String,
    /// Otras secciones, tal cual.
    pub sections: String,
}

impl Default for TaskFile {
    fn default() -> Self {
        Self {
            title: String::new(),
            status: String::new(),
            issue_type: None,
            assignee: None,
            key: None,
            url: None,
            start: String::new(),
            duration: 1,
            depends_on: "-".into(),
            progress: 0,
            sync: None,
            extra: Vec::new(),
            description: String::new(),
            activity: Vec::new(),
            preamble: String::new(),
            sections: String::new(),
        }
    }
}

/// Un identificador que sirve de nombre de archivo en cualquier sistema. `index` queda para los
/// índices de carpeta.
pub fn is_valid_task_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 100
        && !id.eq_ignore_ascii_case("index")
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

const SEPARATOR: &str = " · ";

/// Un dato cabe en una línea.
fn one_line(value: &str) -> String {
    value
        .replace("\r\n", " ")
        .replace(['\r', '\n'], " ")
        .trim()
        .to_string()
}

fn entry_header(line: &str) -> Option<ActivityEntry> {
    let rest = line.strip_prefix("- `")?;
    let (at, rest) = rest.split_once("` ")?;
    let mut parts = rest.split(SEPARATOR);
    let kind = match parts.next()?.trim() {
        "comment" => EntryKind::Comment,
        "history" => EntryKind::History,
        _ => return None,
    };
    let mut entry = ActivityEntry {
        at: at.to_string(),
        kind,
        author: None,
        id: None,
        edited: false,
        text: String::new(),
    };
    // Las marcas se reconocen por lo que dicen: el autor puede faltar
    for part in parts.map(str::trim) {
        if let Some(id) = part.strip_prefix("id:") {
            entry.id = Some(id.to_string());
        } else if part == "edited" {
            entry.edited = true;
        } else if !part.is_empty() {
            entry.author = Some(part.to_string());
        }
    }
    Some(entry)
}

fn parse_activity(lines: &[&str]) -> Vec<ActivityEntry> {
    let mut entries: Vec<(ActivityEntry, Vec<&str>)> = Vec::new();
    let mut blank = 0;
    for line in lines {
        if let Some(entry) = entry_header(line) {
            entries.push((entry, Vec::new()));
            blank = 0;
        } else if let Some((_, text)) = entries.last_mut() {
            if line.trim().is_empty() && !line.starts_with("  ") {
                // Cuenta solo si el texto sigue: un editor puede haber quitado su sangría
                blank += 1;
            } else {
                text.extend(std::iter::repeat_n("", blank));
                blank = 0;
                text.push(line.strip_prefix("  ").unwrap_or(line));
            }
        }
    }
    entries
        .into_iter()
        .map(|(mut entry, text)| {
            entry.text = text.join("\n").trim_end_matches('\n').to_string();
            entry
        })
        .collect()
}

fn parse_description(lines: &[&str]) -> String {
    // Los huecos que separan la cita de los encabezados no son parte de ella
    let start = lines
        .iter()
        .position(|l| !l.is_empty())
        .unwrap_or(lines.len());
    let end = lines
        .iter()
        .rposition(|l| !l.is_empty())
        .map_or(start, |i| i + 1);
    lines[start..end]
        .iter()
        .map(|line| match line.strip_prefix('>') {
            Some(quoted) => quoted.strip_prefix(' ').unwrap_or(quoted),
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn parse(content: &str) -> Result<TaskFile, BoardError> {
    let content = content.replace("\r\n", "\n");
    let mut task = TaskFile::default();
    let lines: Vec<&str> = content.lines().collect();
    let first_section = lines
        .iter()
        .position(|l| l.starts_with("## "))
        .unwrap_or(lines.len());

    let mut preamble = Vec::new();
    let mut titled = false;
    for line in &lines[..first_section] {
        if !titled && let Some(title) = line.strip_prefix("# ") {
            let title = title.trim();
            // `\#` al final: CommonMark quitaría una almohadilla de cierre
            task.title = match title.strip_suffix("\\#") {
                Some(rest) => format!("{rest}#"),
                None => title.to_string(),
            };
            titled = true;
        } else if let Some((key, value)) = line.strip_prefix("- ").and_then(|l| l.split_once(':')) {
            let (key, value) = (key.trim(), value.trim());
            let text = || Some(value.to_string()).filter(|v| !v.is_empty());
            let number = |suffix: char| value.trim_end_matches(suffix).trim().parse().ok();
            match key {
                "status" => task.status = value.to_string(),
                "type" => task.issue_type = text(),
                "assignee" => task.assignee = text(),
                "key" => task.key = text(),
                "url" => task.url = text(),
                "start" => task.start = value.to_string(),
                "duration" => task.duration = number('d').unwrap_or(1).max(1),
                "depends" => task.depends_on = text().unwrap_or_else(|| "-".into()),
                "progress" => task.progress = number('%').unwrap_or(0).min(100) as u8,
                "sync" => {
                    task.sync = Some(
                        serde_json::from_str(value)
                            .map_err(|e| BoardError::new(BoardErrorKind::Sync, e.to_string()))?,
                    )
                }
                _ => task.extra.push((key.to_string(), value.to_string())),
            }
        } else {
            preamble.push(*line);
        }
    }
    task.preamble = preamble.join("\n").trim_matches('\n').to_string();

    let mut sections = Vec::new();
    let mut start = first_section;
    while start < lines.len() {
        let end = lines[start + 1..]
            .iter()
            .position(|l| l.starts_with("## "))
            .map_or(lines.len(), |i| start + 1 + i);
        let body = &lines[start + 1..end];
        match lines[start].trim_end() {
            "## Description" => task.description = parse_description(body),
            "## Activity" => task.activity = parse_activity(body),
            _ => sections.push(lines[start..end].join("\n").trim_end().to_string()),
        }
        start = end;
    }
    task.sections = sections.join("\n\n");
    Ok(task)
}

pub fn serialize(task: &TaskFile) -> String {
    let mut title = one_line(&task.title);
    if title.ends_with('#') {
        title.insert(title.len() - 1, '\\');
    }
    let mut blocks = vec![format!("# {title}")];

    let mut data = Vec::new();
    let mut add = |key: &str, value: &str| data.push(format!("- {key}: {}", one_line(value)));
    add("status", &task.status);
    for (key, value) in [
        ("type", &task.issue_type),
        ("assignee", &task.assignee),
        ("key", &task.key),
        ("url", &task.url),
    ] {
        if let Some(value) = value {
            add(key, value);
        }
    }
    add("start", &task.start);
    add("duration", &format!("{}d", task.duration));
    add("depends", &task.depends_on);
    add("progress", &format!("{}%", task.progress));
    for (key, value) in &task.extra {
        add(key, value);
    }
    if let Some(sync) = &task.sync {
        // JSON en una línea: sus comillas y saltos ya van escapados
        let json = serde_json::to_string(sync).expect("TaskSync siempre se puede escribir");
        data.push(format!("- sync: {json}"));
    }
    blocks.push(data.join("\n"));

    if !task.preamble.is_empty() {
        blocks.push(task.preamble.clone());
    }
    let description = task.description.replace("\r\n", "\n");
    if !description.is_empty() {
        let quoted: Vec<String> = description
            .split('\n')
            .map(|line| {
                if line.is_empty() {
                    ">".to_string()
                } else {
                    format!("> {line}")
                }
            })
            .collect();
        blocks.push(format!("## Description\n\n{}", quoted.join("\n")));
    }
    if !task.activity.is_empty() {
        let entries: Vec<String> = task
            .activity
            .iter()
            .map(|entry| {
                let kind = match entry.kind {
                    EntryKind::Comment => "comment",
                    EntryKind::History => "history",
                };
                let mut header = format!("- `{}` {kind}", one_line(&entry.at).replace('`', "'"));
                if let Some(author) = &entry.author {
                    header.push_str(SEPARATOR);
                    header.push_str(&one_line(author).replace(SEPARATOR, " - "));
                }
                if let Some(id) = &entry.id {
                    header.push_str(&format!("{SEPARATOR}id:{}", one_line(id)));
                }
                if entry.edited {
                    header.push_str(&format!("{SEPARATOR}edited"));
                }
                let text = entry.text.replace("\r\n", "\n");
                let text = text.trim_end_matches('\n');
                if !text.is_empty() {
                    for line in text.split('\n') {
                        header.push_str("\n  ");
                        header.push_str(line);
                    }
                }
                header
            })
            .collect();
        blocks.push(format!("## Activity\n\n{}", entries.join("\n")));
    }
    if !task.sections.is_empty() {
        blocks.push(task.sections.clone());
    }
    format!("{}\n", blocks.join("\n\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: EntryKind, author: Option<&str>, text: &str) -> ActivityEntry {
        ActivityEntry {
            at: "2026-01-02T03:04:05".into(),
            kind,
            author: author.map(str::to_string),
            id: None,
            edited: false,
            text: text.into(),
        }
    }

    fn round_trip(task: &TaskFile) {
        let written = serialize(task);
        let read = parse(&written).unwrap();
        assert_eq!(&read, task, "\n{written}");
        assert_eq!(serialize(&read), written);
    }

    #[test]
    fn una_tarea_completa_va_y_vuelve_igual() {
        let mut synced = entry(
            EntryKind::History,
            Some("Ana"),
            "Estado: To Do → In Progress",
        );
        synced.id = Some("rev-5".into());
        synced.edited = true;
        round_trip(&TaskFile {
            title: "Publicar el servicio: \"v2\" #".into(),
            status: "doing".into(),
            issue_type: Some("Bug".into()),
            assignee: Some("Ana".into()),
            key: Some("AB#123".into()),
            url: Some("https://ejemplo.test/a#b".into()),
            start: "2026-01-02".into(),
            duration: 3,
            depends_on: "t1".into(),
            progress: 40,
            sync: Some(TaskSync {
                id: "123".into(),
                rev: Some("4".into()),
                title: "Con \"comillas\", {llaves} y\nsalto".into(),
                state: "In Progress".into(),
                column: "doing".into(),
                sent: vec!["2026-01-01T10:00:00".into()],
                outside: true,
                description: Some("0a1b2c3d".into()),
                issue_type: Some("Bug".into()),
                assignee: Some(String::new()),
            }),
            extra: vec![("Prioridad".into(), "alta: ya".into()), ("equipo".into(), "".into())],
            description: "  con sangría\n\n# Encabezado\n## Description\n---\n> cita\n> > anidada\n```\n- `x` comment\n```".into(),
            activity: vec![
                synced,
                entry(EntryKind::Comment, None, "Primera\n\n- no es otra entrada\n- `tampoco` comment\n## ni una sección\n\n  sangrada"),
                entry(EntryKind::Comment, Some("Luis"), ""),
            ],
            preamble: "Nota suelta".into(),
            sections: "## Mis apuntes\n\ntexto\n\n## Otra\n- a: b".into(),
        });
    }

    #[test]
    fn lo_minimo_tambien() {
        round_trip(&TaskFile {
            title: "Solo".into(),
            ..Default::default()
        });
        round_trip(&TaskFile {
            title: "A".into(),
            description: "línea\n".into(),
            ..Default::default()
        });
    }

    #[test]
    fn admite_lo_escrito_a_mano() {
        let text = "# T\r\n- status: done\r\n- duration: 5\r\n- progress: 150%\r\n\r\n## Description\r\n\r\nsin cita\r\n\r\n## Activity\r\n- `a` comment · id:7 · Ana\r\n  uno\r\n\r\n  dos\r\n";
        let task = parse(text).unwrap();
        assert_eq!(
            (task.status.as_str(), task.duration, task.progress),
            ("done", 5, 100)
        );
        assert_eq!(task.description, "sin cita");
        let comment = &task.activity[0];
        assert_eq!(
            (comment.author.as_deref(), comment.id.as_deref()),
            (Some("Ana"), Some("7"))
        );
        assert_eq!(comment.text, "uno\n\ndos");
    }

    #[test]
    fn una_linea_sync_rota_no_se_pierde_en_silencio() {
        let error = parse("# T\n- sync: {roto\n").unwrap_err();
        assert_eq!(error.kind, BoardErrorKind::Sync);
    }

    #[test]
    fn solo_valen_ids_que_son_nombres_de_archivo() {
        for ok in ["ado-7", "t3_42", "ado-7_", "PROJ-12"] {
            assert!(is_valid_task_id(ok), "{ok}");
        }
        for bad in ["", "index", "INDEX", "../x", "a/b", "a.md", "ñ", "a b"] {
            assert!(!is_valid_task_id(bad), "{bad}");
        }
    }
}
