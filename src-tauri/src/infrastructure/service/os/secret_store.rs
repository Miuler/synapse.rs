//! Secretos en el llavero del sistema (Secret Service en Linux, Llavero en macOS, Administrador de
//! credenciales en Windows). Las llamadas bloquean y pueden pedir que se desbloquee el llavero.

use crate::application::service::credential_store::CredentialStore;
#[cfg(debug_assertions)]
use std::path::PathBuf;

#[cfg(any(target_os = "linux", target_os = "macos", windows))]
const SERVICE: &str = "synapsers";

#[derive(Debug, Clone)]
pub enum SecretStore {
    System,
    /// Solo en compilaciones de depuración (pruebas): un archivo en lugar del llavero real.
    #[cfg(debug_assertions)]
    File(PathBuf),
}

impl SecretStore {
    pub fn from_env() -> Self {
        #[cfg(debug_assertions)]
        if let Some(path) = std::env::var_os("SYNAPSE_KEYRING_FILE") {
            return Self::File(path.into());
        }
        Self::System
    }

    pub fn get(&self, name: &str) -> Result<Option<String>, String> {
        match self {
            Self::System => system::get(name),
            #[cfg(debug_assertions)]
            Self::File(path) => match std::fs::read_to_string(path.join(name)) {
                Ok(value) => Ok(Some(value)),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(e.to_string()),
            },
        }
    }

    pub fn set(&self, name: &str, value: &str) -> Result<(), String> {
        match self {
            Self::System => system::set(name, value),
            #[cfg(debug_assertions)]
            Self::File(path) => std::fs::create_dir_all(path)
                .and_then(|()| std::fs::write(path.join(name), value))
                .map_err(|e| e.to_string()),
        }
    }

    /// Borrar lo que no está no es un error.
    pub fn delete(&self, name: &str) -> Result<(), String> {
        match self {
            Self::System => system::delete(name),
            #[cfg(debug_assertions)]
            Self::File(path) => match std::fs::remove_file(path.join(name)) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
                _ => Ok(()),
            },
        }
    }
}

impl CredentialStore for SecretStore {
    fn get(&self, name: &str) -> Result<Option<String>, String> {
        SecretStore::get(self, name)
    }

    fn set(&self, name: &str, value: &str) -> Result<(), String> {
        SecretStore::set(self, name, value)
    }

    fn delete(&self, name: &str) -> Result<(), String> {
        SecretStore::delete(self, name)
    }
}

#[cfg(any(target_os = "linux", target_os = "macos", windows))]
mod system {
    use keyring::{Entry, Error};

    fn entry(name: &str) -> Result<Entry, String> {
        Entry::new(super::SERVICE, name).map_err(|e| e.to_string())
    }

    pub fn get(name: &str) -> Result<Option<String>, String> {
        match entry(name)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn set(name: &str, value: &str) -> Result<(), String> {
        entry(name)?.set_password(value).map_err(|e| e.to_string())
    }

    pub fn delete(name: &str) -> Result<(), String> {
        match entry(name)?.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod system {
    const NONE: &str = "no keyring on this system";

    pub fn get(_: &str) -> Result<Option<String>, String> {
        Err(NONE.into())
    }

    pub fn set(_: &str, _: &str) -> Result<(), String> {
        Err(NONE.into())
    }

    pub fn delete(_: &str) -> Result<(), String> {
        Err(NONE.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Usa el llavero real con una entrada de prueba que borra al terminar:
    /// `cargo test llavero_real -- --ignored`.
    #[test]
    #[ignore = "escribe en el llavero del sistema"]
    fn llavero_real_guarda_lee_y_borra() {
        let name = "prueba-temporal";
        let store = SecretStore::System;
        store.set(name, "valor-de-prueba").unwrap();
        let read = store.get(name);
        store.delete(name).unwrap();
        assert_eq!(read.unwrap().as_deref(), Some("valor-de-prueba"));
        assert_eq!(store.get(name).unwrap(), None);
        store.delete(name).unwrap();
    }
}
