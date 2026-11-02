//! Resolución de enlaces (`[[destino]]`) entre notas de la bóveda.
use crate::domain::graph::wikilink::{ProximityScore, LinkTarget};

/// Resuelve el destino de un enlace entre todas las notas, prefiriendo la más cercana al
/// archivo de origen: misma carpeta, luego subcarpetas, luego carpetas padre y luego el resto.
pub struct LinkResolver;

impl LinkResolver {
    /// - `find_exact(ruta)`: ruta de la nota con esa ruta exacta, si existe (consulta O(1)).
    /// - `visit_paths(f)`: llama a `f` con la ruta de cada nota de la bóveda.
    pub fn resolve<E, V>(raw_target: &str, base_file: Option<&str>, find_exact: E, visit_paths: V) -> Option<String>
    where
        E: Fn(&str) -> Option<String>,
        V: Fn(&mut dyn FnMut(&str)),
    {
        let (target_with_ext, anchor) = LinkTarget::normalize(raw_target);
        if target_with_ext.is_empty() {
            return None;
        }

        let attach_anchor = |path: &str| -> String {
            match &anchor {
                Some(a) if !path.contains('#') => format!("{}#{}", path, a),
                _ => path.to_string(),
            }
        };

        // 1. Si hay archivo base, buscar primero coincidencia directa en la misma carpeta (O(1))
        if let Some(base) = base_file {
            let base_dir = LinkTarget::parent_directory(base);
            let sibling_path = if base_dir.is_empty() {
                target_with_ext.clone()
            } else {
                format!("{}/{}", base_dir, target_with_ext)
            };
            if let Some(path) = find_exact(&sibling_path) {
                return Some(attach_anchor(&path));
            }
        }

        // 2. Recolectar todos los candidatos que coincidan con target_with_ext
        let target_suffix = format!("/{}", target_with_ext);
        let mut candidates = Vec::new();
        visit_paths(&mut |note_path| {
            if note_path == target_with_ext || note_path.ends_with(&target_suffix) {
                candidates.push(note_path.to_string());
            }
        });

        // 3. Fallback tolerante: búsqueda insensible a mayúsculas/minúsculas si no hubo coincidencias
        if candidates.is_empty() {
            let target_lower = target_with_ext.to_lowercase();
            let suffix_lower = format!("/{}", target_lower);
            visit_paths(&mut |note_path| {
                let path_lower = note_path.to_lowercase();
                if path_lower == target_lower || path_lower.ends_with(&suffix_lower) {
                    candidates.push(note_path.to_string());
                }
            });
        }

        if candidates.is_empty() {
            return None;
        }

        if candidates.len() == 1 {
            return Some(attach_anchor(&candidates[0]));
        }

        // 4. Varios archivos con el mismo nombre: ordenar por proximidad al archivo base
        candidates.sort_by(|a, b| {
            let score_a = ProximityScore::between(a, base_file);
            let score_b = ProximityScore::between(b, base_file);
            score_a.cmp(&score_b).then_with(|| a.cmp(b))
        });

        Some(attach_anchor(&candidates[0]))
    }
}
