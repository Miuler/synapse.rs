/// Servicio de dominio puro para calcular el nombre de un elemento duplicado.
///
/// Reglas:
/// * Archivos: se inserta `.copy` antes de la extensión (`nota.md` -> `nota.copy.md`).
/// * Archivos sin extensión o dotfiles: se concatena al final (`.gitignore` -> `.gitignore.copy`).
/// * Carpetas: se concatena al final (`docs` -> `docs.copy`).
pub fn copy_name(name: &str, is_folder: bool) -> String {
    if is_folder {
        return format!("{name}.copy");
    }
    match name.rfind('.') {
        Some(idx) if idx > 0 => format!("{}.copy{}", &name[..idx], &name[idx..]),
        _ => format!("{name}.copy"),
    }
}

/// Devuelve un nombre libre en el destino aplicando `.copy` repetidamente
/// mientras `exists` indique que el nombre ya está ocupado.
pub fn next_available_copy_name(name: &str, is_folder: bool, exists: impl Fn(&str) -> bool) -> String {
    let mut candidate = copy_name(name, is_folder);
    while exists(&candidate) {
        candidate = copy_name(&candidate, is_folder);
    }
    candidate
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_with_extension() {
        assert_eq!(copy_name("nota.md", false), "nota.copy.md");
        assert_eq!(copy_name("a.b.txt", false), "a.b.copy.txt");
    }

    #[test]
    fn file_without_extension_and_dotfile() {
        assert_eq!(copy_name("README", false), "README.copy");
        assert_eq!(copy_name(".gitignore", false), ".gitignore.copy");
    }

    #[test]
    fn folder() {
        assert_eq!(copy_name("docs.v1", true), "docs.v1.copy");
    }

    #[test]
    fn repeats_until_free() {
        let taken = ["nota.copy.md"];
        let name = next_available_copy_name("nota.md", false, |n| taken.contains(&n));
        assert_eq!(name, "nota.copy.copy.md");
    }
}
