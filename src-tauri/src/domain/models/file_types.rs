use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SupportedFileTypes {
    pub images: Vec<String>,
    pub markdown: Vec<String>,
    pub diagrams: Vec<String>,
    pub drawings: Vec<String>,
    #[serde(default)]
    pub pdf: Vec<String>,
    pub code: Vec<String>,
    #[serde(default)]
    pub word: Vec<String>,
    #[serde(default)]
    pub excel: Vec<String>,
}

impl Default for SupportedFileTypes {
    fn default() -> Self {
        crate::domain::models::config::AppConfig::default().supported_files
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
            &self.pdf,
            &self.code,
            &self.word,
            &self.excel,
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

    /// Verifica si un nombre de archivo o ruta coincide con un archivo de imagen
    pub fn is_image_file(&self, filename: &str) -> bool {
        let lower = filename.to_lowercase();
        self.images.iter().any(|ext| {
            let clean = ext.trim_start_matches('.').to_lowercase();
            lower.ends_with(&format!(".{}", clean))
        })
    }

    /// Verifica si una extensión dada (insensible a mayúsculas) corresponde a un PDF
    pub fn is_pdf_extension(&self, ext: &str) -> bool {
        self.pdf.iter().any(|e| e.eq_ignore_ascii_case(ext))
    }

    /// Verifica si un nombre de archivo o ruta coincide con un archivo PDF
    pub fn is_pdf_file(&self, filename: &str) -> bool {
        let lower = filename.to_lowercase();
        self.pdf.iter().any(|ext| {
            let clean = ext.trim_start_matches('.').to_lowercase();
            lower.ends_with(&format!(".{}", clean))
        })
    }

    /// Verifica si un nombre de archivo o ruta coincide con un archivo de dibujo (Excalidraw)
    pub fn is_drawing_file(&self, filename: &str) -> bool {
        let lower = filename.to_lowercase();
        self.drawings.iter().any(|ext| {
            let clean = ext.trim_start_matches('.').to_lowercase();
            lower.ends_with(&format!(".{}", clean))
        })
    }

    /// Verifica si una extensión dada (insensible a mayúsculas) corresponde a Word
    pub fn is_word_extension(&self, ext: &str) -> bool {
        self.word.iter().any(|e| e.eq_ignore_ascii_case(ext))
    }

    /// Verifica si un nombre de archivo o ruta coincide con un archivo Word (.docx, .doc, etc.)
    pub fn is_word_file(&self, filename: &str) -> bool {
        let lower = filename.to_lowercase();
        self.word.iter().any(|ext| {
            let clean = ext.trim_start_matches('.').to_lowercase();
            lower.ends_with(&format!(".{}", clean))
        })
    }

    /// Verifica si una extensión dada (insensible a mayúsculas) corresponde a Excel
    pub fn is_excel_extension(&self, ext: &str) -> bool {
        self.excel.iter().any(|e| e.eq_ignore_ascii_case(ext))
    }

    /// Verifica si un nombre de archivo o ruta coincide con un archivo Excel (.xlsx, .xls, etc.)
    pub fn is_excel_file(&self, filename: &str) -> bool {
        let lower = filename.to_lowercase();
        self.excel.iter().any(|ext| {
            let clean = ext.trim_start_matches('.').to_lowercase();
            lower.ends_with(&format!(".{}", clean))
        })
    }

    /// Verifica si un nombre de archivo o ruta coincide con un archivo de Office (Word o Excel)
    pub fn is_office_file(&self, filename: &str) -> bool {
        self.is_word_file(filename) || self.is_excel_file(filename)
    }

    /// Verifica si un nombre de archivo o ruta coincide con alguno de los tipos soportados
    pub fn is_supported_file(&self, filename: &str) -> bool {
        let lower = filename.to_lowercase();
        self.all_extensions().iter().any(|ext| {
            let clean = ext.trim_start_matches('.').to_lowercase();
            lower.ends_with(&format!(".{}", clean))
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
    fn test_supported_file_types_contains_pdf() {
        let types = SupportedFileTypes::default();
        assert!(types.pdf.contains(&"pdf".to_string()));
        assert!(types.is_pdf_extension("pdf"));
        assert!(types.is_pdf_extension("PDF"));
        assert!(types.is_pdf_file("documento.pdf"));
        assert!(types.is_pdf_file("sub/folder/manual.PDF"));
        assert!(types.is_supported_file("manual.pdf"));
    }

    #[test]
    fn test_supported_file_types_contains_word_and_excel() {
        let types = SupportedFileTypes::default();
        assert!(types.word.contains(&"docx".to_string()));
        assert!(types.word.contains(&"doc".to_string()));
        assert!(types.excel.contains(&"xlsx".to_string()));
        assert!(types.excel.contains(&"xls".to_string()));

        assert!(types.is_word_file("informe.docx"));
        assert!(types.is_word_file("carta.DOC"));
        assert!(types.is_excel_file("presupuesto.xlsx"));
        assert!(types.is_excel_file("datos.XLS"));

        assert!(types.is_office_file("informe.docx"));
        assert!(types.is_office_file("presupuesto.xlsx"));
        assert!(!types.is_office_file("nota.md"));

        assert!(types.is_supported_file("informe.docx"));
        assert!(types.is_supported_file("presupuesto.xlsx"));
    }

    #[test]
    fn test_all_extensions_uniqueness() {
        let types = SupportedFileTypes::default();
        let all = types.all_extensions();
        assert!(all.contains(&"svg".to_string()));
        assert!(all.contains(&"png".to_string()));
        assert!(all.contains(&"md".to_string()));
        assert!(all.contains(&"mermaid".to_string()));
        assert!(all.contains(&"cbl".to_string()));
        assert!(types.is_supported_file("program.cbl"));
        assert!(types.is_supported_file("payroll.CBL"));
    }
}
