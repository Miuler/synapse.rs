//! Secretos de 1Password con su CLI (`op read`). Se piden cuando hacen falta: no pasan por el
//! entorno de la aplicación (que heredan los agentes) ni se escriben en disco.

use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::sync::Mutex;

/// Una acción hace varias peticiones seguidas: sin esto, cada una volvería a llamar a `op`.
const KEEP: Duration = Duration::from_secs(60);
/// Margen para que la persona autorice la lectura en 1Password.
const AUTHORIZATION_TIMEOUT: Duration = Duration::from_secs(120);

/// Valores leídos hace poco, por programa y referencia, con su último uso.
type Recent = HashMap<(String, String), (String, Instant)>;
static RECENT: LazyLock<Mutex<Recent>> = LazyLock::new(Default::default);

/// El valor de `reference` (`op://bóveda/elemento/campo`). El error es el que da `op`.
pub async fn read(program: &str, reference: &str) -> Result<String, String> {
    // El candado se mantiene durante la lectura: dos peticiones a la vez no piden dos permisos
    let mut recent = RECENT.lock().await;
    recent.retain(|_, (_, used)| used.elapsed() < KEEP);
    let key = (program.to_string(), reference.to_string());
    if let Some((value, used)) = recent.get_mut(&key) {
        *used = Instant::now();
        return Ok(value.clone());
    }
    let mut command = Command::new(program);
    command
        .args(["read", "--no-newline", reference])
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let output = tokio::time::timeout(AUTHORIZATION_TIMEOUT, command.output())
        .await
        .map_err(|_| format!("{program}: no answer"))?
        .map_err(|e| format!("{program}: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        return Err(format!("{reference}: empty value"));
    }
    recent.insert(key, (value.clone(), Instant::now()));
    Ok(value)
}

/// Olvida un valor que dejó de valer: la próxima lectura vuelve a 1Password.
pub async fn forget(program: &str, reference: &str) {
    RECENT
        .lock()
        .await
        .remove(&(program.to_string(), reference.to_string()));
}
