use std::path::PathBuf;

pub struct MoveNotesCommand {
    pub vault_path: PathBuf,
    pub notes: Vec<String>,
    pub target_folder: String,
    pub create_target_folder: bool,
    pub overwrite: bool,
    /// Reescribe los `[[enlaces]]` de otras notas hacia la nueva ruta.
    pub update_wikilinks: bool,
}

pub struct MoveNotesResult {
    /// (origen, destino) de cada nota movida.
    pub moved: Vec<(String, String)>,
    /// (origen tal como se pidió, motivo).
    pub errors: Vec<(String, String)>,
}
