pub struct SaveVaultUiStateCommand {
    pub sidebar_width: Option<u32>,
    pub expanded_folders: Option<Vec<String>>,
    pub diff_visual_mode: Option<String>,
    pub diff_text_mode: Option<String>,
    pub locked_files: Option<Vec<String>>,
}
