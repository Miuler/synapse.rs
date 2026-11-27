//! La carpeta de un tablero (`Nombre.board/` con su `Nombre.board.md` al lado) solo la escribe el
//! tablero. La protección se deduce de la ruta: no se guarda ni se puede quitar. Es una defensa
//! contra errores dentro de Synapse, no un control de acceso.

use std::path::{Path, PathBuf};

const SUFFIX: &str = ".board";

/// La carpeta de las tareas de `Nombre.board.md`.
pub fn folder_of(board: &Path) -> Option<PathBuf> {
    let name = board.file_name()?.to_str()?.strip_suffix(".md")?;
    (name.ends_with(SUFFIX) && name.len() > SUFFIX.len()).then(|| board.with_file_name(name))
}

/// Los archivos de tablero que protegerían `path` si existen: uno por cada tramo `Nombre.board`
/// de la ruta, incluido el último.
pub fn guards(path: &Path) -> impl Iterator<Item = PathBuf> {
    path.ancestors().filter_map(|folder| {
        let name = folder.file_name()?.to_str()?;
        (name.ends_with(SUFFIX) && name.len() > SUFFIX.len())
            .then(|| folder.with_file_name(format!("{name}.md")))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(path: &str) -> Vec<PathBuf> {
        guards(Path::new(path)).collect()
    }

    #[test]
    fn protegen_la_carpeta_y_todo_lo_de_dentro() {
        let board = vec![PathBuf::from("a/Equipo.board.md")];
        for path in [
            "a/Equipo.board",
            "a/Equipo.board/t1.md",
            "a/Equipo.board/sub/x.png",
        ] {
            assert_eq!(all(path), board, "{path}");
        }
        for free in [
            "a/Equipo.board.md",
            "a/.board/x.md",
            "a/Equipo/t1.md",
            "board/x.md",
        ] {
            assert!(all(free).is_empty(), "{free}");
        }
        assert_eq!(all("A.board/B.board/x.md").len(), 2);
    }

    #[test]
    fn cada_tablero_tiene_su_carpeta() {
        assert_eq!(
            folder_of(Path::new("a/Equipo.board.md")),
            Some(PathBuf::from("a/Equipo.board"))
        );
        for other in ["a/nota.md", "a/.board.md", "a/Equipo.board"] {
            assert_eq!(folder_of(Path::new(other)), None, "{other}");
        }
    }
}
