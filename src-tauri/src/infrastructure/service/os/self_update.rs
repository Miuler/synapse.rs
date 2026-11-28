//! Autoactualización del binario suelto que publica la release.
//!
//! El instalador del plugin updater de Tauri solo sabe instalar AppImage/deb/rpm (Linux) y
//! NSIS/MSI (Windows); la release publica el binario dentro de un `tar.gz` (Linux) o un `zip`
//! (Windows). El plugin descarga y verifica la firma; aquí se extrae el ejecutable del paquete
//! ya verificado y se reemplaza el que está en marcha.

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

/// Extrae el ejecutable de un paquete de la release (`.tar.gz` o `.zip`).
/// Busca la entrada cuyo nombre de archivo es `exe_name` (p. ej. `synapsers` o `synapsers.exe`).
pub fn extract_binary(archive: &[u8], exe_name: &str) -> Result<Vec<u8>, String> {
    if archive.starts_with(b"PK") {
        let mut zip = zip::ZipArchive::new(Cursor::new(archive)).map_err(|e| format!("Invalid ZIP: {e}"))?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).map_err(|e| format!("Invalid ZIP: {e}"))?;
            let name = entry.name().map(|n| n.into_owned()).unwrap_or_default();
            if entry.is_file() && file_name(&name) == exe_name {
                let mut out = Vec::with_capacity(entry.size() as usize);
                entry.read_to_end(&mut out).map_err(|e| e.to_string())?;
                return Ok(out);
            }
        }
    } else if archive.starts_with(&[0x1f, 0x8b]) {
        let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(Cursor::new(archive)));
        for entry in tar.entries().map_err(|e| format!("Invalid tar.gz: {e}"))? {
            let mut entry = entry.map_err(|e| format!("Invalid tar.gz: {e}"))?;
            let path = entry.path().map_err(|e| e.to_string())?.to_string_lossy().into_owned();
            if entry.header().entry_type().is_file() && file_name(&path) == exe_name {
                let mut out = Vec::new();
                entry.read_to_end(&mut out).map_err(|e| e.to_string())?;
                return Ok(out);
            }
        }
    } else {
        return Err("downloaded package is neither .zip nor .tar.gz".into());
    }
    Err(format!("package does not contain «{exe_name}»"))
}

fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// Archivo que deja la sustitución en Windows (el `.exe` en marcha no se puede borrar).
pub fn old_binary_path(exe: &Path) -> PathBuf {
    let mut name = exe.file_name().unwrap_or_default().to_os_string();
    name.push(".old");
    exe.with_file_name(name)
}

/// Sustituye `exe` por `new_binary` sin dejarlo a medias: se escribe al lado y se renombra.
/// En Linux/macOS el renombrado sobre el binario en marcha es atómico; en Windows el `.exe` en
/// marcha se aparta antes como `.old` (se borra en el siguiente arranque).
pub fn replace_executable(exe: &Path, new_binary: &[u8]) -> Result<(), String> {
    let dir = exe.parent().ok_or("executable path has no parent directory")?;
    let mut tmp_name = exe.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(".new");
    let tmp = dir.join(tmp_name);
    std::fs::write(&tmp, new_binary).map_err(|e| {
        format!("Could not write to {} ({e}). Download the release manually from GitHub.", dir.display())
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(exe).map(|m| m.permissions().mode()).unwrap_or(0o755) | 0o111;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(mode)).map_err(|e| e.to_string())?;
    }
    #[cfg(windows)]
    {
        let old = old_binary_path(exe);
        let _ = std::fs::remove_file(&old);
        std::fs::rename(exe, &old).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("Could not move current executable aside: {e}")
        })?;
    }
    std::fs::rename(&tmp, exe).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("Could not replace executable: {e}")
    })
}

/// Borra el `.old` que dejó una actualización anterior en Windows.
pub fn cleanup_old_binary() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::fs::remove_file(old_binary_path(&exe));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tar_gz(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast()));
        for (name, data) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder.append_data(&mut header, name, *data).unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap()
    }

    fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, data) in entries {
            w.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            w.write_all(data).unwrap();
        }
        w.finish().unwrap().into_inner()
    }

    #[test]
    fn extrae_el_binario_de_un_tar_gz_y_de_un_zip() {
        let t = tar_gz(&[("LEEME.txt", b"x"), ("synapsers", b"ELF-nuevo")]);
        assert_eq!(extract_binary(&t, "synapsers").unwrap(), b"ELF-nuevo");
        let z = zip(&[("carpeta/synapsers.exe", b"MZ-nuevo")]);
        assert_eq!(extract_binary(&z, "synapsers.exe").unwrap(), b"MZ-nuevo");
    }

    #[test]
    fn falla_si_falta_el_binario_o_el_formato_es_otro() {
        assert!(extract_binary(&tar_gz(&[("otro", b"x")]), "synapsers").unwrap_err().contains("does not contain"));
        assert!(extract_binary(b"texto", "synapsers").is_err());
    }

    #[test]
    fn reemplaza_el_ejecutable_y_conserva_el_permiso_de_ejecucion() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("synapsers");
        std::fs::write(&exe, b"viejo").unwrap();
        replace_executable(&exe, b"nuevo").unwrap();
        assert_eq!(std::fs::read(&exe).unwrap(), b"nuevo");
        assert!(!dir.path().join("synapsers.new").exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_ne!(std::fs::metadata(&exe).unwrap().permissions().mode() & 0o111, 0);
        }
    }
}
