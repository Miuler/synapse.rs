//! Tableros de carpeta en disco, con un índice en memoria de sus tareas.
//!
//! El índice se valida con la fecha y el tamaño de cada archivo: leer un tablero solo vuelve a
//! interpretar lo que cambió, venga el cambio de Synapse o de fuera.

use crate::domain::board::board_error::{BoardError, BoardErrorKind as Kind};
use crate::domain::board::board_file::{self, BoardFile};
use crate::domain::board::board_repository::{
    BoardRepository, BoardSnapshot, StoredTask, TaskCard, TaskProblem,
};
use crate::domain::board::protection;
use crate::domain::board::table_board;
use crate::domain::board::task_file::{self, TaskFile, is_valid_task_id};
use crate::domain::issue::issue::EntryKind;
use crate::domain::note::note_content::NoteContent;
use dashmap::DashMap;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::SystemTime;

/// Lo que se sabe de un archivo ya interpretado.
#[derive(Clone)]
struct Indexed {
    modified: Option<SystemTime>,
    size: u64,
    card: Result<TaskCard, String>,
}

/// Por carpeta de tablero, cada archivo por su nombre.
static INDEX: LazyLock<DashMap<PathBuf, Arc<HashMap<String, Indexed>>>> =
    LazyLock::new(DashMap::new);

#[derive(Debug, Clone, Copy, Default)]
pub struct BoardRepositoryImpl;

fn io(path: &Path, error: std::io::Error) -> BoardError {
    let kind = if error.kind() == std::io::ErrorKind::NotFound {
        Kind::NotFound
    } else {
        Kind::Io
    };
    BoardError::new(kind, format!("{}: {error}", path.display()))
}

/// La carpeta de tareas de `Nombre.board.md`.
fn folder(board: &Path) -> Result<PathBuf, BoardError> {
    protection::folder_of(board)
        .ok_or_else(|| BoardError::new(Kind::Path, board.display().to_string()))
}

fn task_path(board: &Path, id: &str) -> Result<PathBuf, BoardError> {
    if !is_valid_task_id(id) {
        return Err(BoardError::new(Kind::TaskId, id));
    }
    Ok(folder(board)?.join(format!("{id}.md")))
}

/// Comprueba que el archivo sigue como se leyó (o que no existe, para crearlo).
fn check(path: &Path, expected: Option<&str>) -> Result<(), BoardError> {
    let current = match std::fs::read_to_string(path) {
        Ok(content) => Some(NoteContent::hash(&content)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(io(path, e)),
    };
    if current.as_deref() == expected {
        Ok(())
    } else {
        Err(BoardError::new(Kind::Conflict, path.display().to_string()))
    }
}

/// Un nombre que no repite ningún otro de este proceso.
fn unique() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let turn = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{turn}", std::process::id())
}

/// Guarda el original junto al tablero (`Nombre.board.md.bak`), sin pisar copias anteriores.
fn back_up(board: &Path, content: &str) -> Result<PathBuf, BoardError> {
    let name = board
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("tablero");
    for n in 0..1000 {
        let suffix = if n == 0 {
            String::new()
        } else {
            format!(".{n}")
        };
        let path = board.with_file_name(format!("{name}{suffix}.bak"));
        match std::fs::File::create_new(&path) {
            Ok(mut file) => {
                return file
                    .write_all(content.as_bytes())
                    .map(|()| path.clone())
                    .map_err(|e| {
                        let _ = std::fs::remove_file(&path);
                        io(&path, e)
                    });
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(io(&path, e)),
        }
    }
    Err(BoardError::new(Kind::Io, format!("{name}.bak")))
}

/// Escribe a un temporal oculto de la misma carpeta y lo renombra: nadie ve el archivo a medias.
fn replace(path: &Path, content: &str) -> Result<(), BoardError> {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("tarea");
    let temporary = path.with_file_name(format!(".{name}.{}.tmp", unique()));
    std::fs::write(&temporary, content)
        .and_then(|()| std::fs::rename(&temporary, path))
        .map_err(|e| {
            let _ = std::fs::remove_file(&temporary);
            io(path, e)
        })
}

fn card(id: &str, content: &str) -> Result<TaskCard, String> {
    let task = task_file::parse(content).map_err(|e| e.detail)?;
    let comments = task
        .activity
        .into_iter()
        .filter(|a| a.kind == EntryKind::Comment)
        .collect();
    Ok(TaskCard {
        id: id.to_string(),
        hash: NoteContent::hash(content),
        title: task.title,
        status: task.status,
        issue_type: task.issue_type,
        assignee: task.assignee,
        key: task.key,
        url: task.url,
        start: task.start,
        duration: task.duration,
        depends_on: task.depends_on,
        progress: task.progress,
        sync: task.sync,
        extra: task.extra,
        comments,
    })
}

/// Interpreta lo que cambió en la carpeta desde la última vez.
fn index(folder: &Path) -> Result<Arc<HashMap<String, Indexed>>, BoardError> {
    let entries = match std::fs::read_dir(folder) {
        Ok(entries) => entries,
        // Un tablero sin tareas todavía no tiene carpeta
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Arc::default()),
        Err(e) => return Err(io(folder, e)),
    };
    let files: Vec<(String, Option<SystemTime>, u64)> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let id = name.strip_suffix(".md")?;
            if !is_valid_task_id(id) {
                return None;
            }
            let metadata = entry.metadata().ok().filter(|m| m.is_file())?;
            Some((name, metadata.modified().ok(), metadata.len()))
        })
        .collect();
    let known = INDEX.get(folder).map(|i| i.clone()).unwrap_or_default();
    let fresh: HashMap<String, Indexed> = files
        .into_par_iter()
        .map(|(name, modified, size)| {
            let indexed = match known.get(&name) {
                Some(old) if old.modified == modified && old.size == size => old.clone(),
                _ => {
                    let card = std::fs::read_to_string(folder.join(&name))
                        .map_err(|e| e.to_string())
                        .and_then(|content| card(name.trim_end_matches(".md"), &content));
                    Indexed {
                        modified,
                        size,
                        card,
                    }
                }
            };
            (name, indexed)
        })
        .collect();
    let fresh = Arc::new(fresh);
    INDEX.insert(folder.to_path_buf(), fresh.clone());
    Ok(fresh)
}

/// Tras escribir, el archivo se vuelve a leer aunque conserve fecha y tamaño.
fn forget(path: &Path) {
    if let (Some(folder), Some(name)) = (path.parent(), path.file_name().and_then(|n| n.to_str()))
        && let Some(mut indexed) = INDEX.get_mut(folder)
    {
        Arc::make_mut(&mut indexed).remove(name);
    }
}

impl BoardRepository for BoardRepositoryImpl {
    fn read(&self, board: &Path) -> Result<BoardSnapshot, BoardError> {
        let folder = folder(board)?;
        let content = std::fs::read_to_string(board).map_err(|e| io(board, e))?;
        let file = board_file::parse(&content)?;
        let indexed = index(&folder)?;

        let mut names: Vec<&String> = indexed.keys().collect();
        names.sort();
        let mut cards = HashMap::new();
        let mut unordered = Vec::new();
        let mut problems = Vec::new();
        let mut seen = HashSet::new();
        for name in names {
            let problem = |detail: String| TaskProblem {
                file: name.clone(),
                detail,
            };
            // `T1.md` y `t1.md` son el mismo archivo en otros sistemas
            if !seen.insert(name.to_ascii_lowercase()) {
                problems.push(problem("duplicate name except for case".into()));
                continue;
            }
            match &indexed[name].card {
                Ok(card) => {
                    unordered.push(card.id.to_ascii_lowercase());
                    cards.insert(card.id.to_ascii_lowercase(), card.clone());
                }
                Err(detail) => problems.push(problem(detail.clone())),
            }
        }
        let tasks = file
            .order
            .iter()
            .map(|id| id.to_ascii_lowercase())
            .chain(unordered)
            .filter_map(|id| cards.remove(&id))
            .collect();
        Ok(BoardSnapshot {
            hash: NoteContent::hash(&content),
            board: file,
            tasks,
            problems,
        })
    }

    fn write(
        &self,
        board: &Path,
        file: &BoardFile,
        expected: Option<&str>,
    ) -> Result<String, BoardError> {
        folder(board)?;
        let content = board_file::serialize(file)?;
        check(board, expected)?;
        replace(board, &content)?;
        Ok(NoteContent::hash(&content))
    }

    fn read_task(&self, board: &Path, id: &str) -> Result<StoredTask, BoardError> {
        let path = task_path(board, id)?;
        let content = std::fs::read_to_string(&path).map_err(|e| io(&path, e))?;
        Ok(StoredTask {
            task: task_file::parse(&content)?,
            hash: NoteContent::hash(&content),
        })
    }

    fn write_task(
        &self,
        board: &Path,
        id: &str,
        task: &TaskFile,
        expected: Option<&str>,
    ) -> Result<String, BoardError> {
        let path = task_path(board, id)?;
        let folder = folder(board)?;
        std::fs::create_dir_all(&folder).map_err(|e| io(&folder, e))?;
        if expected.is_none() {
            // Otro con el mismo nombre salvo por las mayúsculas lo pisaría en otros sistemas
            let name = format!("{id}.md");
            let taken = index(&folder)?
                .keys()
                .any(|k| k.eq_ignore_ascii_case(&name));
            if taken {
                return Err(BoardError::new(Kind::Conflict, name));
            }
        }
        let content = task_file::serialize(task);
        check(&path, expected)?;
        replace(&path, &content)?;
        forget(&path);
        Ok(NoteContent::hash(&content))
    }

    fn delete_task(&self, board: &Path, id: &str, expected: &str) -> Result<(), BoardError> {
        let path = task_path(board, id)?;
        check(&path, Some(expected))?;
        std::fs::remove_file(&path).map_err(|e| io(&path, e))?;
        forget(&path);
        Ok(())
    }

    fn convert(&self, board: &Path, expected: &str) -> Result<(), BoardError> {
        let folder = folder(board)?;
        let content = std::fs::read_to_string(board).map_err(|e| io(board, e))?;
        let lines = |text: &str| text.replace("\r\n", "\n");
        if lines(&content) != lines(expected) {
            return Err(BoardError::new(Kind::Conflict, board.display().to_string()));
        }
        let converted = table_board::convert(&content)?;
        let file = board_file::serialize(&converted.board)?;
        // Nada se toca si el resultado no se pudiera volver a leer
        board_file::parse(&file)?;
        let empty = match std::fs::read_dir(&folder) {
            Ok(mut entries) => entries.next().is_none(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
            Err(_) => false,
        };
        if !empty {
            return Err(BoardError::new(
                Kind::FolderExists,
                folder.display().to_string(),
            ));
        }

        // Las tareas se escriben aparte y la carpeta aparece entera
        let name = folder
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("board");
        let staging = folder.with_file_name(format!(".{name}.{}.tmp", unique()));
        std::fs::create_dir(&staging).map_err(|e| io(&staging, e))?;
        let backup = converted
            .tasks
            .iter()
            .try_for_each(|(id, task)| {
                let path = staging.join(format!("{id}.md"));
                std::fs::write(&path, task_file::serialize(task)).map_err(|e| io(&path, e))
            })
            .and_then(|()| back_up(board, &content))
            .and_then(|backup| {
                // Vacía, si estaba
                let _ = std::fs::remove_dir(&folder);
                std::fs::rename(&staging, &folder).map_err(|e| {
                    let _ = std::fs::remove_file(&backup);
                    io(&folder, e)
                })?;
                Ok(backup)
            })
            .inspect_err(|_| {
                let _ = std::fs::remove_dir_all(&staging);
            })?;
        INDEX.remove(&folder);
        // El tablero, al final y solo si sigue como estaba
        check(board, Some(&NoteContent::hash(&content)))
            .and_then(|()| replace(board, &file))
            .inspect_err(|_| {
                let _ = std::fs::remove_dir_all(&folder);
                let _ = std::fs::remove_file(&backup);
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(title: &str) -> TaskFile {
        TaskFile {
            title: title.into(),
            status: "todo".into(),
            description: "texto".into(),
            ..Default::default()
        }
    }

    fn setup() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let board = dir.path().join("Equipo.board.md");
        let file = BoardFile {
            order: vec!["b".into(), "a".into(), "falta".into()],
            ..Default::default()
        };
        BoardRepositoryImpl.write(&board, &file, None).unwrap();
        (dir, board)
    }

    fn titles(board: &Path) -> Vec<String> {
        let snapshot = BoardRepositoryImpl.read(board).unwrap();
        snapshot.tasks.into_iter().map(|t| t.title).collect()
    }

    #[test]
    fn lee_las_tareas_en_el_orden_del_tablero_y_las_demas_al_final() {
        let (dir, board) = setup();
        let repository = BoardRepositoryImpl;
        assert!(titles(&board).is_empty());
        for id in ["c", "a", "b"] {
            repository.write_task(&board, id, &task(id), None).unwrap();
        }
        let folder = dir.path().join("Equipo.board");
        std::fs::write(folder.join("index.md"), "# índice").unwrap();
        std::fs::write(folder.join("rota.md"), "# R\n- sync: {\n").unwrap();
        std::fs::write(folder.join("apuntes.txt"), "x").unwrap();
        let snapshot = repository.read(&board).unwrap();
        let ids: Vec<&str> = snapshot.tasks.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["b", "a", "c"]);
        assert_eq!(snapshot.problems.len(), 1);
        assert_eq!(snapshot.problems[0].file, "rota.md");
        // Sin temporales a la vista
        let hidden = std::fs::read_dir(&folder)
            .unwrap()
            .filter(|e| {
                e.as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with('.')
            })
            .count();
        assert_eq!(hidden, 0);
    }

    #[test]
    fn ve_lo_que_cambia_por_fuera() {
        let (dir, board) = setup();
        BoardRepositoryImpl
            .write_task(&board, "a", &task("antes"), None)
            .unwrap();
        assert_eq!(titles(&board), ["antes"]);
        let path = dir.path().join("Equipo.board/a.md");
        std::fs::write(&path, "# después, y más largo\n").unwrap();
        assert_eq!(titles(&board), ["después, y más largo"]);
        std::fs::remove_file(&path).unwrap();
        assert!(titles(&board).is_empty());
    }

    #[test]
    fn no_pisa_lo_que_cambio_desde_que_se_leyo() {
        let (_dir, board) = setup();
        let repository = BoardRepositoryImpl;
        let first = repository
            .write_task(&board, "a", &task("uno"), None)
            .unwrap();
        // Mismo tamaño y quizá la misma fecha: aun así se ve el cambio propio
        let second = repository
            .write_task(&board, "a", &task("dos"), Some(&first))
            .unwrap();
        assert_eq!(titles(&board), ["dos"]);
        let kind = |result: Result<String, BoardError>| result.unwrap_err().kind;
        assert_eq!(
            kind(repository.write_task(&board, "a", &task("x"), Some(&first))),
            Kind::Conflict
        );
        assert_eq!(
            kind(repository.write_task(&board, "a", &task("x"), None)),
            Kind::Conflict
        );
        assert_eq!(
            kind(repository.write_task(&board, "A", &task("x"), None)),
            Kind::Conflict
        );
        assert_eq!(
            repository
                .delete_task(&board, "a", &first)
                .unwrap_err()
                .kind,
            Kind::Conflict
        );
        assert_eq!(
            repository.read_task(&board, "a").unwrap().task.description,
            "texto"
        );
        repository.delete_task(&board, "a", &second).unwrap();
        assert_eq!(
            repository.read_task(&board, "a").unwrap_err().kind,
            Kind::NotFound
        );
    }

    const TABLE: &str = "---\ntype: board\n---\n\n| id | Tarea | Estado | Inicio | Duración | Depende | Progreso |\n|---|---|---|---|---|---|---|\n| t1 | Uno | todo | 2026-01-01 | 1d | - | 0% |\n| t2 | Dos | todo | 2026-01-01 | 1d | t1 | 0% |\n";

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn convierte_un_tablero_con_tabla_y_guarda_el_original() {
        let dir = tempfile::tempdir().unwrap();
        let board = dir.path().join("Equipo.board.md");
        std::fs::write(&board, TABLE).unwrap();
        std::fs::write(dir.path().join("Equipo.board.md.bak"), "de antes").unwrap();
        // Una carpeta vacía no estorba
        std::fs::create_dir(dir.path().join("Equipo.board")).unwrap();
        BoardRepositoryImpl.convert(&board, TABLE).unwrap();
        assert_eq!(titles(&board), ["Uno", "Dos"]);
        assert_eq!(
            names(dir.path()),
            [
                "Equipo.board",
                "Equipo.board.md",
                "Equipo.board.md.1.bak",
                "Equipo.board.md.bak"
            ]
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("Equipo.board.md.1.bak")).unwrap(),
            TABLE
        );
        let again = std::fs::read_to_string(&board).unwrap();
        let error = BoardRepositoryImpl.convert(&board, &again).unwrap_err();
        assert_eq!(error.kind, Kind::NotTableBoard);
    }

    #[test]
    fn no_convierte_si_algo_no_cuadra_y_no_deja_rastro() {
        let dir = tempfile::tempdir().unwrap();
        let board = dir.path().join("Equipo.board.md");
        std::fs::write(&board, TABLE).unwrap();
        let kind = |expected: &str| {
            BoardRepositoryImpl
                .convert(&board, expected)
                .unwrap_err()
                .kind
        };
        assert_eq!(kind("otra cosa"), Kind::Conflict);
        let folder = dir.path().join("Equipo.board");
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(folder.join("mio.md"), "# mío").unwrap();
        assert_eq!(kind(TABLE), Kind::FolderExists);
        assert_eq!(names(dir.path()), ["Equipo.board", "Equipo.board.md"]);
        assert_eq!(names(&folder), ["mio.md"]);
        assert_eq!(std::fs::read_to_string(&board).unwrap(), TABLE);
    }

    #[test]
    fn solo_escribe_dentro_de_la_carpeta_de_un_tablero() {
        let (dir, board) = setup();
        let repository = BoardRepositoryImpl;
        let error = repository
            .write_task(&board, "../fuera", &task("x"), None)
            .unwrap_err();
        assert_eq!(error.kind, Kind::TaskId);
        for other in ["nota.md", ".board.md", "Equipo.board"] {
            let error = repository
                .write_task(&dir.path().join(other), "a", &task("x"), None)
                .unwrap_err();
            assert_eq!(error.kind, Kind::Path, "{other}");
        }
        assert!(!dir.path().join("fuera.md").exists());
    }
}
