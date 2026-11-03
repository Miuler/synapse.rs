use crate::application::service::system_shell::SystemShell;
use crate::infrastructure::repository::file::vault_item_repository_impl::VaultItemRepositoryImpl;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct SystemShellImpl;

impl SystemShellImpl {
    /// Abre un archivo con la aplicación por defecto del sistema operativo (Word, Excel, etc.).
    pub fn open_file_with_default_app(
        vault_path: &Path,
        relative_path: &str,
    ) -> Result<(), String> {
        let abs_path = if Path::new(relative_path).is_absolute() {
            let p = PathBuf::from(relative_path);
            if !p.exists() {
                return Err(format!("El archivo no existe: {}", p.display()));
            }
            p
        } else {
            VaultItemRepositoryImpl::resolve(vault_path, relative_path)?
        };

        if !abs_path.exists() {
            return Err(format!("El archivo no existe: {}", abs_path.display()));
        }

        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("cmd")
                .args(["/C", "start", "", &abs_path.to_string_lossy()])
                .spawn()
                .map_err(|e| format!("Error al abrir archivo en Windows: {}", e))?;
        }

        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(&abs_path)
                .spawn()
                .map_err(|e| format!("Error al abrir archivo en macOS: {}", e))?;
        }

        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdg-open")
                .arg(&abs_path)
                .spawn()
                .map_err(|e| format!("Error al abrir archivo con xdg-open: {}", e))?;
        }

        Ok(())
    }

    /// Muestra y resalta un archivo o carpeta en el explorador de archivos nativo del sistema.
    pub fn show_item_in_folder(vault_path: &Path, relative_path: &str) -> Result<(), String> {
        let abs_path = if Path::new(relative_path).is_absolute() {
            let p = PathBuf::from(relative_path);
            if !p.exists() {
                return Err(format!("La ruta no existe: {}", p.display()));
            }
            p
        } else {
            VaultItemRepositoryImpl::resolve(vault_path, relative_path)?
        };

        if !abs_path.exists() {
            return Err(format!("La ruta no existe: {}", abs_path.display()));
        }

        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("explorer")
                .arg(format!("/select,\"{}\"", abs_path.to_string_lossy()))
                .spawn()
                .map_err(|e| format!("Error al mostrar en carpeta en Windows: {}", e))?;
        }

        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .args(["-R", &abs_path.to_string_lossy()])
                .spawn()
                .map_err(|e| format!("Error al mostrar en carpeta en macOS: {}", e))?;
        }

        #[cfg(target_os = "linux")]
        {
            let mut opened = false;

            // 1. Estándar FreeDesktop: D-Bus org.freedesktop.FileManager1.ShowItems
            // Es la especificación oficial soportada por Dolphin (KDE), Nautilus (GNOME), Nemo (Cinnamon), Thunar (XFCE), PCManFM-Qt (LXQt), etc.
            let uri = format!("file://{}", abs_path.to_string_lossy());

            // 1a. Intentar con busctl (systemd / moderno)
            if let Ok(mut child) = std::process::Command::new("busctl")
                .args([
                    "--user",
                    "call",
                    "org.freedesktop.FileManager1",
                    "/org/freedesktop/FileManager1",
                    "org.freedesktop.FileManager1",
                    "ShowItems",
                    "ass",
                    "1",
                    &uri,
                    "",
                ])
                .spawn()
            {
                if let Ok(status) = child.wait() {
                    if status.success() {
                        opened = true;
                    }
                }
            }

            // 1b. Si busctl falló o no está, intentar con dbus-send
            if !opened {
                if let Ok(mut child) = std::process::Command::new("dbus-send")
                    .args([
                        "--session",
                        "--dest=org.freedesktop.FileManager1",
                        "--type=method_call",
                        "/org/freedesktop/FileManager1",
                        "org.freedesktop.FileManager1.ShowItems",
                        &format!("array:string:\"{}\"", uri),
                        "string:\"\"",
                    ])
                    .spawn()
                {
                    if let Ok(status) = child.wait() {
                        if status.success() {
                            opened = true;
                        }
                    }
                }
            }

            // 2. Fallbacks directos por CLI si D-Bus no respondió o no está presente el servicio
            if !opened {
                let fallback_commands = [
                    (
                        "dolphin",
                        vec![
                            "--select".to_string(),
                            abs_path.to_string_lossy().to_string(),
                        ],
                    ),
                    (
                        "nautilus",
                        vec![
                            "--select".to_string(),
                            abs_path.to_string_lossy().to_string(),
                        ],
                    ),
                    (
                        "nemo",
                        vec![
                            "--no-desktop".to_string(),
                            abs_path.to_string_lossy().to_string(),
                        ],
                    ),
                    ("thunar", vec![abs_path.to_string_lossy().to_string()]),
                    ("pcmanfm", vec![abs_path.to_string_lossy().to_string()]),
                    ("pcmanfm-qt", vec![abs_path.to_string_lossy().to_string()]),
                ];

                for (cmd, args) in fallback_commands {
                    if let Ok(_) = std::process::Command::new(cmd).args(&args).spawn() {
                        opened = true;
                        break;
                    }
                }
            }

            if !opened {
                // Último fallback: abrir la carpeta contenedora con xdg-open
                let target_dir = if abs_path.is_dir() {
                    abs_path.clone()
                } else {
                    abs_path.parent().unwrap_or(&abs_path).to_path_buf()
                };
                std::process::Command::new("xdg-open")
                    .arg(&target_dir)
                    .spawn()
                    .map_err(|e| {
                        format!("Error al abrir carpeta contenedora con xdg-open: {}", e)
                    })?;
            }
        }

        Ok(())
    }
}

impl SystemShell for SystemShellImpl {
    fn open_with_default_app(&self, vault_path: &Path, relative_path: &str) -> Result<(), String> {
        Self::open_file_with_default_app(vault_path, relative_path)
    }

    fn show_in_folder(&self, vault_path: &Path, relative_path: &str) -> Result<(), String> {
        Self::show_item_in_folder(vault_path, relative_path)
    }
}
