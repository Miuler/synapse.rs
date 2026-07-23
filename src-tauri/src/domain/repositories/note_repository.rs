use crate::domain::models::note::Note;
use crate::domain::value_objects::note_path::NoteRelativePath;
use std::path::Path;

/// Puerto (Trait) del repositorio de notas dentro de la Capa de Dominio.
///
/// Define las operaciones fundamentales de persistencia y lectura de notas
/// sin depender de ningún detalle de infraestructura o sistema de archivos concreto.
///
/// Nota sobre `rustdoc`: Las implementaciones (`impl NoteRepository for Struct`)
/// heredan automáticamente esta documentación en la generación de `cargo doc`.
pub trait NoteRepository: Send + Sync {
    /// Escanea y lista todas las notas válidas dentro del directorio de la bóveda filtradas por extensiones.
    ///
    /// # Parámetros
    /// * `vault_path`: Referencia a la ruta base absoluta de la bóveda (`Path`) en el sistema operativo.
    /// * `extensions`: Slice de cadenas (`&[String]`) con las extensiones permitidas (ej. `["md", "markdown"]`).
    ///
    /// # Retorno
    /// * `Ok(Vec<Note>)`: Lista de notas ordenadas alfabéticamente si la lectura fue exitosa.
    /// * `Err(String)`: Mensaje descriptivo si ocurrió un error al acceder al directorio.
    fn list_notes(&self, vault_path: &Path, extensions: &[String]) -> Result<Vec<Note>, String>;

    /// Lee y recupera el contenido de una nota específica a partir de su ruta relativa.
    ///
    /// # Parámetros
    /// * `vault_path`: Referencia a la ruta base absoluta de la bóveda (`Path`) en el sistema operativo.
    /// * `relative_path`: Value object `NoteRelativePath` validado que representa la ubicación relativa de la nota.
    ///
    /// # Retorno
    /// * `Ok(Note)`: La entidad de dominio `Note` instanciada con su contenido y metadatos.
    /// * `Err(String)`: Mensaje descriptivo si el archivo no existe o no pudo ser leído.
    fn read_note(&self, vault_path: &Path, relative_path: &NoteRelativePath) -> Result<Note, String>;

    /// Guarda o actualiza una nota en el medio de almacenamiento persistente.
    ///
    /// # Parámetros
    /// * `vault_path`: Referencia a la ruta base absoluta de la bóveda (`Path`) en el sistema operativo.
    /// * `note`: Referencia a la entidad de dominio `Note` que contiene el título, ruta relativa y contenido a guardar.
    ///
    /// # Retorno
    /// * `Ok(())`: Si la nota fue persistida correctamente.
    /// * `Err(String)`: Mensaje descriptivo si ocurrió un error de escritura o permisos.
    fn save_note(&self, vault_path: &Path, note: &Note) -> Result<(), String>;
}
