pub struct CheckPluginUpdatesQuery {
    /// (id, versión actual, repo)
    pub plugins: Vec<(String, String, String)>,
}
