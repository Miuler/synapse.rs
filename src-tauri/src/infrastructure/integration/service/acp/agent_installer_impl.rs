use super::registry::{BinaryTarget, RegistryAgent, RegistryClient};
use crate::application::integration::service::AgentInstaller;
use crate::domain::agent::AgentLaunch;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Descarga el binario del agente desde el registro de ACP, verifica su sha256 y lo
/// descomprime en la carpeta de datos de Synapse. Si no hay binario para la plataforma,
/// usa el paquete npm con `npx` (requiere Node instalado).
#[derive(Clone)]
pub struct AgentInstallerImpl {
    registry: RegistryClient,
}

impl AgentInstallerImpl {
    pub fn new(registry: RegistryClient) -> Self {
        Self { registry }
    }

    async fn install_binary(
        &self,
        agent: &RegistryAgent,
        target: &BinaryTarget,
    ) -> Result<AgentLaunch, String> {
        let bytes = self
            .registry
            .http()
            .get(&target.archive)
            .timeout(Duration::from_secs(15 * 60))
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| format!("No se pudo descargar {}: {e}", agent.name))?
            .bytes()
            .await
            .map_err(|e| format!("No se pudo descargar {}: {e}", agent.name))?;
        if let Some(expected) = &target.sha256 {
            verify_sha256(&bytes, expected)?;
        }
        let dir = agent.install_dir();
        let archive = target.archive.clone();
        let cmd = target.cmd.clone();
        let staging = dir.with_extension("partial");
        let final_dir = dir.clone();
        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let _ = std::fs::remove_dir_all(&staging);
            std::fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
            extract(&archive, &bytes, &cmd, &staging)?;
            let _ = std::fs::remove_dir_all(&final_dir);
            std::fs::rename(&staging, &final_dir).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())??;
        let command = dir.join(target.cmd.trim_start_matches("./"));
        make_executable(&command)?;
        let launch = AgentLaunch {
            command: command.to_string_lossy().into_owned(),
            args: target.args.clone(),
            env: target.env.clone(),
        };
        agent.mark_installed(&launch)?;
        Ok(launch)
    }
}

impl AgentInstaller for AgentInstallerImpl {
    async fn ensure_installed(&self, agent_id: &str) -> Result<AgentLaunch, String> {
        if let Some(cli) = super::local_cli::find(agent_id) {
            return cli.launch();
        }
        let agent = self.registry.verified(agent_id).await?;
        if let Some(launch) = agent.installed_launch()
            && Path::new(&launch.command).exists()
        {
            return Ok(launch);
        }
        if let Some(target) = agent.binary_for_platform() {
            return self.install_binary(&agent, target).await;
        }
        if let Some(pkg) = &agent.distribution.npx {
            let mut args = vec!["-y".to_string(), pkg.package.clone()];
            args.extend(pkg.args.iter().cloned());
            return Ok(AgentLaunch {
                command: "npx".into(),
                args,
                env: pkg.env.clone(),
            });
        }
        Err(format!(
            "{} no publica una versión para esta plataforma",
            agent.name
        ))
    }
}

fn verify_sha256(bytes: &[u8], expected: &str) -> Result<(), String> {
    let actual: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if actual.eq_ignore_ascii_case(expected.trim()) {
        Ok(())
    } else {
        Err("La descarga del agente no coincide con su sha256 del registro".into())
    }
}

/// Descomprime según la extensión del archivo; un binario suelto se guarda con el nombre de `cmd`.
fn extract(archive: &str, bytes: &[u8], cmd: &str, dest: &Path) -> Result<(), String> {
    let name = archive
        .split(['?', '#'])
        .next()
        .unwrap_or(archive)
        .to_ascii_lowercase();
    if name.ends_with(".zip") {
        let mut zip =
            zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
        for i in 0..zip.len() {
            let mut file = zip.by_index(i).map_err(|e| e.to_string())?;
            let Some(rel) = file.enclosed_name() else {
                continue;
            };
            let out = dest.join(rel);
            if file.is_dir() {
                std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
                continue;
            }
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut f = std::fs::File::create(&out).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut f).map_err(|e| e.to_string())?;
            #[cfg(unix)]
            if let Some(mode) = file.unix_mode() {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&out, std::fs::Permissions::from_mode(mode));
            }
        }
        Ok(())
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        let gz = flate2::read::GzDecoder::new(bytes);
        tar::Archive::new(gz)
            .unpack(dest)
            .map_err(|e| e.to_string())
    } else if [".tar.bz2", ".tbz2"].iter().any(|ext| name.ends_with(ext)) {
        Err("Formato .tar.bz2 aún no soportado para agentes".into())
    } else {
        let out: PathBuf = dest.join(cmd.trim_start_matches("./"));
        std::fs::write(&out, bytes).map_err(|e| e.to_string())
    }
}

fn make_executable(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!(
            "El paquete del agente no contiene {}",
            path.display()
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)
            .map_err(|e| e.to_string())?
            .permissions();
        perms.set_mode(perms.mode() | 0o755);
        std::fs::set_permissions(path, perms).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifica_el_sha256() {
        // sha256("hola")
        let ok = "b221d9dbb083a7f33428d7c2a3c3198ae925614d70210e28716ccaa7cd4ddb79";
        assert!(verify_sha256(b"hola", ok).is_ok());
        assert!(verify_sha256(b"chau", ok).is_err());
    }

    #[test]
    fn descomprime_un_tar_gz() {
        let dir = std::env::temp_dir().join(format!("synapse-acp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut tar_bytes = Vec::new();
        {
            let gz = flate2::write::GzEncoder::new(&mut tar_bytes, flate2::Compression::fast());
            let mut builder = tar::Builder::new(gz);
            let mut header = tar::Header::new_gnu();
            header.set_size(4);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, "agente", &b"#!sh"[..])
                .unwrap();
            builder.into_inner().unwrap().finish().unwrap();
        }
        extract("https://x/a.tar.gz", &tar_bytes, "./agente", &dir).unwrap();
        make_executable(&dir.join("agente")).unwrap();
        assert_eq!(std::fs::read(dir.join("agente")).unwrap(), b"#!sh");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
