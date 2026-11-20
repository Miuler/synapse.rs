//! Resolución de enlaces (`[[destino]]`, `[texto](destino)`) entre archivos de la bóveda.
//!
//! Regla única, sin adivinar:
//! 1. Si el destino existe tal cual (relativo a la carpeta del archivo de origen, o a la raíz de la
//!    bóveda), se usa ese. Para imágenes y adjuntos se mira además `attachments/` junto al origen,
//!    antes que la raíz.
//! 2. Si no, se busca por nombre (o por la parte final de la ruta) en toda la bóveda: si hay uno
//!    solo, se usa ese.
//! 3. Si hay varios, el enlace es **ambiguo**: no se elige ninguno (quien lo usa pregunta al usuario
//!    o lo marca), para no abrir nunca el archivo equivocado sin avisar.
use crate::domain::graph::wikilink::LinkTarget;

/// Carpeta de imágenes y adjuntos junto a cada nota
pub const ATTACHMENTS_DIR: &str = "attachments";

/// Resultado de resolver un enlace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkResolution {
    /// Ruta relativa a la bóveda (con el `#ancla` si el enlace la tenía)
    Resolved(String),
    /// Varios archivos con ese nombre (rutas sin ancla, ordenadas)
    Ambiguous(Vec<String>),
    NotFound,
}

pub struct LinkResolver;

impl LinkResolver {
    /// Ruta del destino, o `None` si no existe o es ambiguo.
    pub fn resolve<E, V>(raw_target: &str, base_file: Option<&str>, find_exact: E, visit_paths: V) -> Option<String>
    where
        E: Fn(&str) -> Option<String>,
        V: Fn(&mut dyn FnMut(&str)),
    {
        match Self::resolve_detailed(raw_target, base_file, find_exact, visit_paths) {
            LinkResolution::Resolved(p) => Some(p),
            _ => None,
        }
    }

    /// - `find_exact(ruta)`: ruta del archivo con esa ruta exacta, si existe (consulta O(1)).
    /// - `visit_paths(f)`: llama a `f` con la ruta de cada archivo de la bóveda.
    pub fn resolve_detailed<E, V>(
        raw_target: &str,
        base_file: Option<&str>,
        find_exact: E,
        visit_paths: V,
    ) -> LinkResolution
    where
        E: Fn(&str) -> Option<String>,
        V: Fn(&mut dyn FnMut(&str)),
    {
        let (target_with_ext, anchor) = LinkTarget::normalize(raw_target);
        if target_with_ext.is_empty() {
            return LinkResolution::NotFound;
        }
        let attach_anchor = |path: &str| -> String {
            match &anchor {
                Some(a) if !path.contains('#') => format!("{}#{}", path, a),
                _ => path.to_string(),
            }
        };

        // 1. Tal cual: relativo a la carpeta del archivo de origen y, si no, a la raíz de la bóveda.
        //    Para imágenes y adjuntos (todo lo que no es `.md`), entre medias la carpeta `attachments/`
        //    junto al archivo de origen (`a/b/c/NOTA.md` → `a/b/c/attachments/imagen.png`)
        let is_note = target_with_ext.to_lowercase().ends_with(".md");
        if let Some(base) = base_file {
            let base_dir = LinkTarget::parent_directory(base);
            if !base_dir.is_empty() {
                if let Some(path) = Self::join(&base_dir, &target_with_ext).and_then(|p| find_exact(&p)) {
                    return LinkResolution::Resolved(attach_anchor(&path));
                }
            }
            if !is_note {
                let in_attachments = Self::join(&base_dir, &format!("{ATTACHMENTS_DIR}/{target_with_ext}"));
                if let Some(path) = in_attachments.and_then(|p| find_exact(&p)) {
                    return LinkResolution::Resolved(attach_anchor(&path));
                }
            }
        }
        if let Some(path) = find_exact(target_with_ext.trim_start_matches("./")) {
            return LinkResolution::Resolved(attach_anchor(&path));
        }

        // 2. Por nombre (o por el final de la ruta) en toda la bóveda; sin distinguir mayúsculas
        //    solo si no hay ninguno que coincida exactamente
        let name = target_with_ext.trim_start_matches("./").trim_start_matches('/');
        let suffix = format!("/{}", name);
        let mut candidates = Vec::new();
        visit_paths(&mut |p| {
            if p == name || p.ends_with(&suffix) {
                candidates.push(p.to_string());
            }
        });
        if candidates.is_empty() {
            let name_lower = name.to_lowercase();
            let suffix_lower = format!("/{}", name_lower);
            visit_paths(&mut |p| {
                let lower = p.to_lowercase();
                if lower == name_lower || lower.ends_with(&suffix_lower) {
                    candidates.push(p.to_string());
                }
            });
        }
        candidates.sort();
        candidates.dedup();
        match candidates.len() {
            0 => LinkResolution::NotFound,
            1 => LinkResolution::Resolved(attach_anchor(&candidates[0])),
            // 3. Varios: ambiguo
            _ => LinkResolution::Ambiguous(candidates),
        }
    }

    /// Une `dir` y `rel` normalizando `.` y `..`; `None` si sale de la bóveda.
    fn join(dir: &str, rel: &str) -> Option<String> {
        let mut out: Vec<&str> = Vec::new();
        for part in dir.split('/').chain(rel.split('/')) {
            match part {
                "" | "." => {}
                ".." => {
                    out.pop()?;
                }
                p => out.push(p),
            }
        }
        Some(out.join("/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILES: &[&str] = &[
        "Glosario.docx",
        "proyecto/notas/plan.md",
        "proyecto/notas/Glosario.docx",
        "proyecto/AS41/Glosario.docx",
        "otro/unico.md",
        "proyecto/notas/sub/tabla.xlsx",
    ];

    fn resolve(target: &str, base: Option<&str>) -> LinkResolution {
        LinkResolver::resolve_detailed(
            target,
            base,
            |p| FILES.iter().find(|f| **f == p).map(|f| f.to_string()),
            |visit| {
                for f in FILES {
                    visit(f);
                }
            },
        )
    }

    #[test]
    fn la_ruta_tal_cual_gana_relativa_a_la_nota_y_luego_a_la_raiz() {
        // Existe junto a la nota
        assert_eq!(resolve("Glosario.docx", Some("proyecto/notas/plan.md")), LinkResolution::Resolved("proyecto/notas/Glosario.docx".into()));
        // Relativa con subcarpeta y con `..`
        assert_eq!(resolve("sub/tabla.xlsx", Some("proyecto/notas/plan.md")), LinkResolution::Resolved("proyecto/notas/sub/tabla.xlsx".into()));
        assert_eq!(resolve("../AS41/Glosario.docx", Some("proyecto/notas/plan.md")), LinkResolution::Resolved("proyecto/AS41/Glosario.docx".into()));
        // Desde la raíz de la bóveda
        assert_eq!(resolve("proyecto/AS41/Glosario.docx", Some("otro/unico.md")), LinkResolution::Resolved("proyecto/AS41/Glosario.docx".into()));
        assert_eq!(resolve("Glosario.docx", None), LinkResolution::Resolved("Glosario.docx".into()));
    }

    #[test]
    fn un_solo_archivo_con_ese_nombre_se_resuelve_con_ancla_y_sin_extension() {
        assert_eq!(resolve("unico", Some("proyecto/notas/plan.md")), LinkResolution::Resolved("otro/unico.md".into()));
        assert_eq!(resolve("unico#Título", None), LinkResolution::Resolved("otro/unico.md#Título".into()));
        assert_eq!(resolve("UNICO.md", None), LinkResolution::Resolved("otro/unico.md".into()));
        assert_eq!(resolve("no-existe.md", None), LinkResolution::NotFound);
    }

    #[test]
    fn varios_con_el_mismo_nombre_es_ambiguo_sin_elegir_por_cercania() {
        // «informe.md» está en dos carpetas y en ninguna junto a la nota ni en la raíz
        let files = ["a/informe.md", "a/b/informe.md", "c/nota.md"];
        let r = |t: &str, base: Option<&str>| {
            LinkResolver::resolve_detailed(
                t,
                base,
                |p| files.iter().find(|f| **f == p).map(|f| f.to_string()),
                |visit| {
                    for f in &files {
                        visit(f);
                    }
                },
            )
        };
        // Aunque «a/b/» esté más cerca de «a/b/c/x.md», no se elige por cercanía
        assert_eq!(r("informe", Some("a/b/c/x.md")), LinkResolution::Ambiguous(vec!["a/b/informe.md".into(), "a/informe.md".into()]));
        assert_eq!(r("informe", Some("c/nota.md")), LinkResolution::Ambiguous(vec!["a/b/informe.md".into(), "a/informe.md".into()]));
        // `resolve` (grafo, backlinks…) no cuenta los ambiguos
        assert_eq!(
            LinkResolver::resolve("informe", Some("c/nota.md"), |p| files.iter().find(|f| **f == p).map(|f| f.to_string()), |v| {
                for f in &files {
                    v(f);
                }
            }),
            None
        );
        // Con parte de la ruta deja de ser ambiguo
        assert_eq!(r("b/informe", Some("c/nota.md")), LinkResolution::Resolved("a/b/informe.md".into()));
    }

    #[test]
    fn imagenes_y_adjuntos_tambien_en_attachments_junto_a_la_nota() {
        let files = ["a/b/c/nota.md", "a/b/c/attachments/imagen.png", "otra/imagen.png", "a/b/c/attachments/notas.md", "a/b/c/junto.png", "a/b/c/attachments/junto.png"];
        let r = |t: &str| {
            LinkResolver::resolve_detailed(
                t,
                Some("a/b/c/nota.md"),
                |p| files.iter().find(|f| **f == p).map(|f| f.to_string()),
                |visit| {
                    for f in &files {
                        visit(f);
                    }
                },
            )
        };
        // Junto a la nota gana; después attachments/ (aunque haya otra igual en la bóveda)
        assert_eq!(r("junto.png"), LinkResolution::Resolved("a/b/c/junto.png".into()));
        assert_eq!(r("imagen.png"), LinkResolution::Resolved("a/b/c/attachments/imagen.png".into()));
        // Las notas no se buscan en attachments/: «notas» sigue la regla de siempre (única por nombre)
        assert_eq!(r("notas"), LinkResolution::Resolved("a/b/c/attachments/notas.md".into()));
        // Desde otra carpeta no aplica: hay dos imagen.png → ambiguo
        let other = LinkResolver::resolve_detailed(
            "imagen.png",
            Some("x/y.md"),
            |p| files.iter().find(|f| **f == p).map(|f| f.to_string()),
            |visit| {
                for f in &files {
                    visit(f);
                }
            },
        );
        assert!(matches!(other, LinkResolution::Ambiguous(_)));
    }
}
