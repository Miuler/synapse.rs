use crate::domain::mcp::mcp::{McpPortRegistry, McpPortRegistryEntry, McpVaultConfig};
use std::collections::HashSet;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_MCP_PORT: u16 = 27182;
pub const MAX_PORT_SEARCH_RANGE: u16 = 28000;

/// Resuelve las rutas de almacenamiento global para el registro de puertos MCP.
/// Retorna `~/.config/synapse/mcp-ports.json` como principal y `~/.config/synapsers/mcp-ports.json` como compatibilidad.
pub fn get_global_registry_paths() -> Vec<PathBuf> {
    let mut base_dirs = Vec::new();
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        let trimmed = xdg.trim();
        if !trimmed.is_empty() {
            base_dirs.push(PathBuf::from(trimmed));
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            base_dirs.push(PathBuf::from(trimmed).join(".config"));
        }
    }
    if let Ok(up) = std::env::var("USERPROFILE") {
        let trimmed = up.trim();
        if !trimmed.is_empty() {
            base_dirs.push(PathBuf::from(trimmed).join(".config"));
        }
    }
    if base_dirs.is_empty() {
        base_dirs.push(PathBuf::from(".config"));
    }

    let mut result = Vec::new();
    for base in base_dirs {
        let p1 = base.join("synapse").join("mcp-ports.json");
        let p2 = base.join("synapsers").join("mcp-ports.json");
        if !result.contains(&p1) {
            result.push(p1);
        }
        if !result.contains(&p2) {
            result.push(p2);
        }
    }
    result
}

/// Lee el registro global de puertos MCP desde disco.
pub fn read_global_registry() -> McpPortRegistry {
    for path in get_global_registry_paths() {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(reg) = serde_json::from_str::<McpPortRegistry>(&content) {
                    return reg;
                }
            }
        }
    }
    McpPortRegistry::default()
}

/// Escribe el registro global de puertos MCP en `~/.config/synapse` y `~/.config/synapsers`.
pub fn write_global_registry(registry: &McpPortRegistry) {
    if let Ok(json_str) = serde_json::to_string_pretty(registry) {
        for path in get_global_registry_paths() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&path, &json_str);
        }
    }
}

/// Ruta de configuración per-vault: `<vault>/.synapse/mcp.json`
pub fn get_vault_mcp_config_path(vault_path: &Path) -> PathBuf {
    vault_path.join(".synapse").join("mcp.json")
}

/// Lee la configuración específica de MCP de la bóveda
pub fn read_vault_config(vault_path: &Path) -> Option<McpVaultConfig> {
    let p = get_vault_mcp_config_path(vault_path);
    if p.exists() {
        if let Ok(content) = std::fs::read_to_string(&p) {
            if let Ok(cfg) = serde_json::from_str::<McpVaultConfig>(&content) {
                return Some(cfg);
            }
        }
    }
    None
}

/// Persiste la configuración específica de MCP de la bóveda
pub fn write_vault_config(vault_path: &Path, config: &McpVaultConfig) {
    let p = get_vault_mcp_config_path(vault_path);
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json_str) = serde_json::to_string_pretty(config) {
        let _ = std::fs::write(&p, json_str);
    }
}

/// Obtiene el puerto asignado para la bóveda (desde la bóveda o desde el registro global)
pub fn get_vault_port(vault_path: &Path) -> Option<u16> {
    if let Some(cfg) = read_vault_config(vault_path) {
        return Some(cfg.port);
    }
    let canonical = canonical_vault_key(vault_path);
    let reg = read_global_registry();
    reg.vaults.get(&canonical).map(|e| e.port)
}

/// Verifica si MCP debe estar activo para esta bóveda
pub fn is_vault_enabled(vault_path: &Path) -> bool {
    if let Some(cfg) = read_vault_config(vault_path) {
        return cfg.enabled;
    }
    let canonical = canonical_vault_key(vault_path);
    let reg = read_global_registry();
    reg.vaults
        .get(&canonical)
        .map(|e| e.enabled)
        .unwrap_or(false)
}

/// Comprueba si un proceso sigue vivo dado su PID en el sistema
pub fn is_process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    let current_pid = std::process::id();
    if pid == current_pid {
        return true;
    }

    #[cfg(target_os = "linux")]
    {
        Path::new(&format!("/proc/{}", pid)).exists()
    }
    #[cfg(unix)]
    #[cfg(not(target_os = "linux"))]
    {
        unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Convierte la ruta de la bóveda a una clave canónica absoluta
pub fn canonical_vault_key(vault_path: &Path) -> String {
    vault_path
        .canonicalize()
        .unwrap_or_else(|_| vault_path.to_path_buf())
        .to_string_lossy()
        .to_string()
}

/// Timestamp actual en milisegundos desde UNIX_EPOCH
pub fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Asigna y enlaza un puerto TCP disponible para la bóveda actual, evitando
/// conflictos con otras instancias de Synapse y otros vaults.
pub fn allocate_and_bind_port(
    vault_path: &Path,
    preferred_port: Option<u16>,
) -> Result<(TcpListener, u16), String> {
    let mut registry = read_global_registry();
    let canonical_key = canonical_vault_key(vault_path);

    // 1. Limpiar procesos muertos de instancias caídas en el registro global
    let mut cleaned_any = false;
    for entry in registry.vaults.values_mut() {
        if entry.running {
            if let Some(pid) = entry.pid {
                if !is_process_alive(pid) {
                    entry.running = false;
                    entry.pid = None;
                    cleaned_any = true;
                }
            }
        }
    }
    if cleaned_any {
        write_global_registry(&registry);
    }

    // 2. Determinar puertos actualmente ocupados por otras bóvedas activas
    let occupied_ports: HashSet<u16> = registry
        .vaults
        .iter()
        .filter(|(k, v)| *k != &canonical_key && v.running)
        .map(|(_, v)| v.port)
        .collect();

    // 3. Determinar puerto preferido para esta bóveda
    let preferred = preferred_port
        .or_else(|| read_vault_config(vault_path).map(|c| c.port))
        .or_else(|| registry.vaults.get(&canonical_key).map(|e| e.port))
        .unwrap_or(DEFAULT_MCP_PORT);

    // 4. Intentar primero con el puerto preferido si no está reclamado por otra bóveda activa
    if !occupied_ports.contains(&preferred) {
        if let Ok(listener) = TcpListener::bind(format!("127.0.0.1:{}", preferred)) {
            return Ok((listener, preferred));
        }
    }

    // 5. Si el preferido está ocupado, buscar el siguiente puerto libre en el rango
    for p in DEFAULT_MCP_PORT..=MAX_PORT_SEARCH_RANGE {
        if p == preferred || occupied_ports.contains(&p) {
            continue;
        }
        if let Ok(listener) = TcpListener::bind(format!("127.0.0.1:{}", p)) {
            return Ok((listener, p));
        }
    }

    // 6. Si todos los puertos conocidos están en uso, intentar puerto libre dinámico del SO
    match TcpListener::bind("127.0.0.1:0") {
        Ok(l) => {
            let actual_port = l.local_addr().map_err(|e| e.to_string())?.port();
            Ok((l, actual_port))
        }
        Err(e) => Err(format!(
            "No se pudo asignar ningún puerto TCP disponible para MCP: {}",
            e
        )),
    }
}

/// Indica si otra instancia viva de Synapse ya sirve MCP para esta bóveda según el registro global.
fn owned_by_other_instance(registry: &McpPortRegistry, canonical_key: &str) -> bool {
    registry.vaults.get(canonical_key).is_some_and(|e| {
        e.running
            && e.pid
                .is_some_and(|pid| pid != std::process::id() && is_process_alive(pid))
    })
}

/// Indica si el puerto configurado en `.synapse/mcp.json` sigue ocupado aunque esta instancia
/// enlazó otro: lo normal es que lo tenga la instancia principal de la bóveda (p. ej. abierta con
/// otro HOME, que no comparte el registro global).
fn configured_port_held_elsewhere(vault_path: &Path, bound_port: u16) -> bool {
    read_vault_config(vault_path).is_some_and(|cfg| {
        cfg.enabled
            && cfg.port != bound_port
            && TcpListener::bind(format!("127.0.0.1:{}", cfg.port)).is_err()
    })
}

/// Registra el inicio de MCP para una bóveda tanto en su `.synapse/mcp.json` como en el registro global.
/// Una segunda instancia sobre la misma bóveda no persiste nada: los clientes MCP deben seguir
/// apuntando al puerto de la instancia dueña.
pub fn record_mcp_started(vault_path: &Path, port: u16) {
    let canonical_key = canonical_vault_key(vault_path);
    let endpoint = format!("http://127.0.0.1:{}/mcp", port);
    let mut registry = read_global_registry();

    if owned_by_other_instance(&registry, &canonical_key)
        || configured_port_held_elsewhere(vault_path, port)
    {
        log::info!(
            "MCP en el puerto {port}: otra instancia es dueña de la bóveda, no se persiste el puerto"
        );
        return;
    }

    // 1. Guardar en .synapse/mcp.json de la bóveda
    let vault_config = McpVaultConfig {
        enabled: true,
        port,
        last_endpoint: Some(endpoint.clone()),
    };
    write_vault_config(vault_path, &vault_config);

    // 2. Guardar en ~/.config/synapse/mcp-ports.json
    registry.vaults.insert(
        canonical_key.clone(),
        McpPortRegistryEntry {
            vault_path: canonical_key,
            port,
            enabled: true,
            running: true,
            pid: Some(std::process::id()),
            endpoint,
            updated_at: current_timestamp_ms(),
        },
    );
    write_global_registry(&registry);
}

/// Registra la desactivación explícita de MCP para una bóveda por decisión del usuario.
/// Persiste `enabled: false` en `.synapse/mcp.json` y marca `enabled: false, running: false` en el registro global.
pub fn record_mcp_disabled(vault_path: &Path, port: u16) {
    let canonical_key = canonical_vault_key(vault_path);
    let endpoint = format!("http://127.0.0.1:{}/mcp", port);

    // 1. Guardar en .synapse/mcp.json de la bóveda
    let vault_config = McpVaultConfig {
        enabled: false,
        port,
        last_endpoint: Some(endpoint.clone()),
    };
    write_vault_config(vault_path, &vault_config);

    // 2. Actualizar en ~/.config/synapse/mcp-ports.json
    let mut registry = read_global_registry();
    if let Some(entry) = registry.vaults.get_mut(&canonical_key) {
        entry.running = false;
        entry.enabled = false;
        entry.pid = None;
        entry.updated_at = current_timestamp_ms();
    } else {
        registry.vaults.insert(
            canonical_key.clone(),
            McpPortRegistryEntry {
                vault_path: canonical_key,
                port,
                enabled: false,
                running: false,
                pid: None,
                endpoint,
                updated_at: current_timestamp_ms(),
            },
        );
    }
    write_global_registry(&registry);
}

/// Registra el apagado técnico del proceso/socket en ejecución (al cerrar la aplicación,
/// cambiar de workspace o limpiar el runtime), manteniendo intacta la preferencia
/// `enabled: true` del vault en `.synapse/mcp.json`.
pub fn record_mcp_runtime_stopped(vault_path: &Path) {
    let canonical_key = canonical_vault_key(vault_path);
    let mut registry = read_global_registry();
    // Solo la instancia dueña marca la bóveda como detenida.
    if let Some(entry) = registry.vaults.get_mut(&canonical_key) {
        if entry.pid.is_some_and(|pid| pid != std::process::id()) {
            return;
        }
        entry.running = false;
        entry.pid = None;
        entry.updated_at = current_timestamp_ms();
        write_global_registry(&registry);
    }
}

/// Registra el apagado voluntario de MCP para una bóveda (alias de compatibilidad con `record_mcp_disabled`)
pub fn record_mcp_stopped(vault_path: &Path, port: u16) {
    record_mcp_disabled(vault_path, port);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_vault_mcp_config_roundtrip() {
        let temp = tempdir().unwrap();
        let vault = temp.path();

        assert_eq!(read_vault_config(vault), None);
        assert!(!is_vault_enabled(vault));

        let cfg = McpVaultConfig {
            enabled: true,
            port: 27190,
            last_endpoint: Some("http://127.0.0.1:27190/mcp".to_string()),
        };
        write_vault_config(vault, &cfg);

        let loaded = read_vault_config(vault).expect("Debe leer config guardada");
        assert_eq!(loaded.port, 27190);
        assert!(loaded.enabled);
        assert!(is_vault_enabled(vault));
        assert_eq!(get_vault_port(vault), Some(27190));
    }

    #[test]
    fn test_runtime_stopped_preserves_vault_enabled_state() {
        let temp = tempdir().unwrap();
        let vault = temp.path();

        // 1. Iniciar MCP para la bóveda (debe marcar enabled: true en bóveda y registro global)
        record_mcp_started(vault, 27195);
        assert!(
            is_vault_enabled(vault),
            "Debe estar habilitado tras record_mcp_started"
        );
        let cfg = read_vault_config(vault).expect("Debe tener .synapse/mcp.json");
        assert!(cfg.enabled);

        // 2. Apagar técnicamente el runtime (ej. cierre de app o ventana)
        record_mcp_runtime_stopped(vault);

        // 3. La bóveda debe permanecer enabled: true para que al reabrir el workspace se active automáticamente
        assert!(
            is_vault_enabled(vault),
            "Debe seguir habilitado tras apagado del runtime"
        );
        let cfg_after = read_vault_config(vault).expect("Debe preservar .synapse/mcp.json");
        assert!(
            cfg_after.enabled,
            "La configuración en disco de la bóveda no debe sobreescribirse"
        );

        // 4. Si el usuario explícitamente lo desactiva, ahí sí debe pasar a false
        record_mcp_disabled(vault, 27195);
        assert!(
            !is_vault_enabled(vault),
            "Debe quedar deshabilitado tras desactivación voluntaria"
        );
        let cfg_disabled = read_vault_config(vault).expect("Debe leer config actualizada");
        assert!(!cfg_disabled.enabled);
    }

    #[test]
    fn test_second_instance_does_not_overwrite_vault_port() {
        let temp = tempdir().unwrap();
        let vault = temp.path();

        // La instancia principal tiene su puerto configurado y ocupado.
        let owner = TcpListener::bind("127.0.0.1:0").unwrap();
        let owner_port = owner.local_addr().unwrap().port();
        write_vault_config(
            vault,
            &McpVaultConfig {
                enabled: true,
                port: owner_port,
                last_endpoint: None,
            },
        );

        // Una segunda instancia enlaza otro puerto y no debe reescribir mcp.json.
        let (other, other_port) = allocate_and_bind_port(vault, None).unwrap();
        assert_ne!(other_port, owner_port);
        record_mcp_started(vault, other_port);
        assert_eq!(read_vault_config(vault).unwrap().port, owner_port);

        // Al cerrarse la principal, la siguiente instancia sí persiste su puerto.
        drop(owner);
        drop(other);
        record_mcp_started(vault, other_port);
        assert_eq!(read_vault_config(vault).unwrap().port, other_port);
    }

    #[test]
    fn test_allocate_and_bind_port() {
        let temp = tempdir().unwrap();
        let vault = temp.path();

        let (listener, port) = allocate_and_bind_port(vault, None).expect("Debe enlazar puerto");
        assert!(port >= DEFAULT_MCP_PORT);
        drop(listener);
    }
}
