pub struct SaveShortcutCommand {
    /// `welcome` o `workspace`.
    pub section: String,
    pub action: String,
    /// Una o varias combinaciones separadas por comas, p. ej. `Ctrl+O, Alt+O`.
    pub value: String,
}
