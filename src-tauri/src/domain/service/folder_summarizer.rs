use crate::domain::vault::folder::FolderMeta;
use std::collections::HashMap;

/// Resume una carpeta a partir de sus notas (servicio de dominio: involucra varias notas).
pub struct FolderSummarizer;

impl FolderSummarizer {
    /// Calcula la huella semántica de una carpeta:
    /// - Conteo de notas
    /// - Conteo de subcarpetas inmediatas
    /// - Dominant tags (las 5 etiquetas más frecuentes)
    /// - Muestra de notas (hasta 3 títulos o nombres de archivo)
    pub fn summarize(
        folder_path: &str,
        all_folders: &[String],
        all_files: &[(&str, &str, &[String])], // (rel_path, title_or_name, tags)
    ) -> FolderMeta {
        let clean_folder = folder_path.trim_matches('/').replace('\\', "/");
        let name = clean_folder
            .split('/')
            .last()
            .unwrap_or(&clean_folder)
            .to_string();

        let parent = if clean_folder.contains('/') {
            clean_folder.rsplit_once('/').map(|(p, _)| p.to_string())
        } else {
            None
        };

        let folder_prefix = if clean_folder.is_empty() {
            String::new()
        } else {
            format!("{}/", clean_folder)
        };

        // Subcarpetas inmediatas
        let subfolders_count = all_folders
            .iter()
            .filter(|f| {
                let f_clean = f.trim_matches('/').replace('\\', "/");
                if f_clean == clean_folder || f_clean.is_empty() {
                    return false;
                }
                if clean_folder.is_empty() {
                    !f_clean.contains('/')
                } else if f_clean.starts_with(&folder_prefix) {
                    let remainder = &f_clean[folder_prefix.len()..];
                    !remainder.contains('/')
                } else {
                    false
                }
            })
            .count();

        // Archivos bajo esta carpeta
        let mut notes_count = 0;
        let mut tag_counts: HashMap<String, usize> = HashMap::new();
        let mut sample_notes = Vec::new();

        for (file_path, title_or_name, tags) in all_files {
            let f_clean = file_path.trim_matches('/').replace('\\', "/");
            let matches = if clean_folder.is_empty() {
                true
            } else {
                f_clean.starts_with(&folder_prefix)
            };

            if matches {
                notes_count += 1;
                if sample_notes.len() < 3 {
                    let display_title = if !title_or_name.trim().is_empty() {
                        title_or_name.to_string()
                    } else {
                        f_clean.split('/').last().unwrap_or(&f_clean).to_string()
                    };
                    sample_notes.push(display_title);
                }
                for t in *tags {
                    let clean_t = t.trim().trim_start_matches('#').to_string();
                    if !clean_t.is_empty() {
                        *tag_counts.entry(clean_t).or_insert(0) += 1;
                    }
                }
            }
        }

        let mut sorted_tags: Vec<(String, usize)> = tag_counts.into_iter().collect();
        sorted_tags.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let dominant_tags = sorted_tags
            .into_iter()
            .take(5)
            .map(|(t, _)| format!("#{}", t))
            .collect();

        FolderMeta {
            relative_path: clean_folder,
            name,
            parent,
            notes_count,
            subfolders_count,
            dominant_tags,
            sample_notes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::vault::folder::FolderPath;

    #[test]
    fn test_extract_ancestor_folders() {
        let files = vec![
            "infra/cloud/azure/aks.md",
            "infra/cloud/azure/cli.md",
            "finanzas/facturas/recibo.pdf",
            "root_note.md",
        ];
        let ancestors = FolderPath::ancestors_of(files);
        assert_eq!(
            ancestors,
            vec![
                "finanzas".to_string(),
                "finanzas/facturas".to_string(),
                "infra".to_string(),
                "infra/cloud".to_string(),
                "infra/cloud/azure".to_string(),
            ]
        );
    }

    #[test]
    fn test_summarize_folder() {
        let folders = vec![
            "infra".to_string(),
            "infra/cloud".to_string(),
            "infra/cloud/azure".to_string(),
        ];
        let t1 = vec!["azure".to_string(), "cloud".to_string()];
        let t2 = vec!["azure".to_string(), "cli".to_string()];
        let files = vec![
            ("infra/cloud/azure/aks.md", "AKS Guide", t1.as_slice()),
            ("infra/cloud/azure/cli.md", "Azure CLI", t2.as_slice()),
        ];

        let summary = FolderSummarizer::summarize("infra/cloud/azure", &folders, &files);
        assert_eq!(summary.name, "azure");
        assert_eq!(summary.parent.as_deref(), Some("infra/cloud"));
        assert_eq!(summary.notes_count, 2);
        assert_eq!(summary.subfolders_count, 0);
        assert_eq!(summary.dominant_tags, vec!["#azure", "#cli", "#cloud"]);
        assert_eq!(summary.sample_notes, vec!["AKS Guide", "Azure CLI"]);
    }
}
