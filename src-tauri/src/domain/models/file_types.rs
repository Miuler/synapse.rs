use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SupportedFileTypes {
    pub images: Vec<String>,
    pub markdown: Vec<String>,
    pub diagrams: Vec<String>,
    pub drawings: Vec<String>,
    pub code: Vec<String>,
}

impl Default for SupportedFileTypes {
    fn default() -> Self {
        Self {
            images: vec![
                "png".to_string(),
                "webp".to_string(),
                "jpg".to_string(),
                "jpeg".to_string(),
                "gif".to_string(),
                "bmp".to_string(),
                "svg".to_string(),
                "ico".to_string(),
                "avif".to_string(),
            ],
            markdown: vec![
                "md".to_string(),
                "markdown".to_string(),
            ],
            diagrams: vec![
                "mmd".to_string(),
                "mermaid".to_string(),
            ],
            drawings: vec![
                "excalidraw".to_string(),
                "excalidraw.json".to_string(),
            ],
            code: vec![
                "rs".to_string(),
            ],
        }
    }
}

impl SupportedFileTypes {
    /// Obtiene una lista plana con todas las extensiones soportadas sin duplicados
    pub fn all_extensions(&self) -> Vec<String> {
        let mut all = Vec::new();
        let groups = [
            &self.images,
            &self.markdown,
            &self.diagrams,
            &self.drawings,
            &self.code,
        ];
        for group in groups {
            for ext in group {
                if !all.contains(ext) {
                    all.push(ext.clone());
                }
            }
        }
        all
    }

    /// Verifica si una extensión dada (insensible a mayúsculas) corresponde a una imagen
    pub fn is_image_extension(&self, ext: &str) -> bool {
        self.images.iter().any(|e| e.eq_ignore_ascii_case(ext))
    }

    /// Verifica si un nombre de archivo o ruta coincide con alguno de los tipos soportados
    pub fn is_supported_file(&self, filename: &str) -> bool {
        let lower = filename.to_lowercase();
        self.all_extensions().iter().any(|ext| {
            if ext.contains('.') {
                lower.ends_with(&format!(".{}", ext.to_lowercase()))
            } else {
                lower.ends_with(&format!(".{}", ext.to_lowercase()))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_file_types_contains_svg() {
        let types = SupportedFileTypes::default();
        assert!(types.images.contains(&"svg".to_string()));
        assert!(types.is_image_extension("svg"));
        assert!(types.is_image_extension("SVG"));
        assert!(types.is_supported_file("grafico.svg"));
        assert!(types.is_supported_file("path/to/icon.SVG"));
    }

    #[test]
    fn test_all_extensions_uniqueness() {
        let types = SupportedFileTypes::default();
        let all = types.all_extensions();
        assert!(all.contains(&"svg".to_string()));
        assert!(all.contains(&"png".to_string()));
        assert!(all.contains(&"md".to_string()));
        assert!(all.contains(&"mermaid".to_string()));
    }
}
