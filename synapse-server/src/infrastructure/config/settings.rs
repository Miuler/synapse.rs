//! Configuración en capas: valores por defecto → archivo TOML → variables de entorno `SYNAPSE_*`.
//! Se valida al arrancar y falla rápido.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::Parser;
use figment::Figment;
use figment::providers::{Env, Format, Serialized, Toml};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::tenant::RetentionPolicy;

/// Argumentos de línea de comandos del servidor.
#[derive(Debug, Parser)]
#[command(
    name = "synapse-server",
    version,
    about = "Servidor de sincronización de Synapse"
)]
pub struct Cli {
    /// Archivo de configuración TOML (opcional).
    #[arg(long, env = "SYNAPSE_CONFIG")]
    pub config: Option<PathBuf>,
}

/// Backend de almacenamiento de blobs.
///
/// Las credenciales S3/R2 no forman parte de la configuración: las lee `object_store` del entorno.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BlobBackend {
    /// Almacenamiento en disco local.
    Local {
        /// Directorio raíz donde se guardan los blobs.
        data_dir: PathBuf,
    },
    /// Almacenamiento compatible con S3/R2.
    S3 {
        /// Nombre del bucket.
        bucket: String,
        /// Región del bucket.
        region: String,
        /// Endpoint personalizado (opcional, p. ej. Cloudflare R2 o MinIO).
        endpoint: Option<String>,
    },
}

/// Configuración completa del servidor.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Config {
    /// Dirección del servidor gRPC.
    pub grpc_addr: String,
    /// Dirección del servidor HTTP (salud y gestión).
    pub http_addr: String,
    /// `json` en producción; cualquier otro valor, formato legible.
    pub log_format: String,
    /// URL de conexión a la base de datos SQLite.
    pub database_url: String,
    /// Backend de almacenamiento de blobs (`local` o `s3`).
    pub blob_backend: BlobBackend,
    /// `client_id` de Google OAuth esperado en el claim `aud` de los ID tokens.
    pub google_client_id: String,
    /// URL del conjunto de claves públicas JWKS de Google.
    pub google_jwks_url: String,
    /// Lista opcional de cuentas permitidas (`sub` o email verificado); vacía permite cualquiera.
    pub allowed_accounts: Vec<String>,
    /// Política de retención asignada a los tenants nuevos.
    #[serde(with = "retention_policy_serde")]
    pub default_retention: RetentionPolicy,
    /// Umbral a partir del cual el cliente trocea archivos grandes (bytes).
    pub chunk_threshold_bytes: u64,
    /// Tamaño máximo permitido para un artefacto (bytes).
    pub max_artifact_bytes: u64,
    /// Tamaño máximo de un mensaje gRPC (bytes).
    pub max_message_bytes: usize,
    /// Número máximo de nodos permitidos en un solo `PushDelta`.
    pub max_delta_nodes: usize,
    /// Intervalo entre ejecuciones del recolector de basura.
    #[serde(with = "humantime_serde")]
    pub gc_interval: Duration,
    /// Periodo de gracia sobre `last_seen_at` antes de recolectar blobs huérfanos.
    #[serde(with = "humantime_serde")]
    pub gc_grace_period: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            grpc_addr: "127.0.0.1:50051".into(),
            http_addr: "127.0.0.1:8080".into(),
            log_format: "text".into(),
            database_url: "sqlite://data/synapse.db?mode=rwc".into(),
            blob_backend: BlobBackend::Local {
                data_dir: PathBuf::from("data"),
            },
            google_client_id: String::new(),
            google_jwks_url: "https://www.googleapis.com/oauth2/v3/certs".into(),
            allowed_accounts: Vec::new(),
            default_retention: RetentionPolicy::Unlimited,
            chunk_threshold_bytes: 4 * 1024 * 1024,
            max_artifact_bytes: 2 * 1024 * 1024 * 1024,
            max_message_bytes: 4 * 1024 * 1024,
            max_delta_nodes: 10_000,
            gc_interval: Duration::from_secs(60 * 60),
            gc_grace_period: Duration::from_secs(24 * 60 * 60),
        }
    }
}

/// Error de validación de la configuración.
#[derive(Debug, Error)]
#[error("configuración inválida en '{key}': {reason}")]
pub struct ConfigValidationError {
    pub key: &'static str,
    pub reason: String,
}

impl ConfigValidationError {
    fn new(key: &'static str, reason: impl Into<String>) -> Self {
        Self {
            key,
            reason: reason.into(),
        }
    }
}

impl Config {
    /// Carga la configuración fusionando defaults → archivo TOML (si existe) → `SYNAPSE_*`.
    pub fn load(file: Option<&Path>) -> Result<Self, Box<figment::Error>> {
        let mut figment = Figment::from(Serialized::defaults(Self::default()));
        if let Some(file) = file {
            figment = figment.merge(Toml::file(file));
        }
        figment
            .merge(Env::prefixed("SYNAPSE_").split("__"))
            .extract()
            .map_err(Box::new)
    }

    /// Valida todas las claves de configuración y falla rápido con el nombre de la clave errónea.
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        if self.grpc_addr.parse::<SocketAddr>().is_err() {
            return Err(ConfigValidationError::new(
                "grpc_addr",
                format!("dirección de socket inválida: '{}'", self.grpc_addr),
            ));
        }
        if self.http_addr.parse::<SocketAddr>().is_err() {
            return Err(ConfigValidationError::new(
                "http_addr",
                format!("dirección de socket inválida: '{}'", self.http_addr),
            ));
        }
        if self.database_url.trim().is_empty() {
            return Err(ConfigValidationError::new(
                "database_url",
                "no puede estar vacía",
            ));
        }
        match &self.blob_backend {
            BlobBackend::Local { data_dir } => {
                if data_dir.as_os_str().is_empty() {
                    return Err(ConfigValidationError::new(
                        "blob_backend.data_dir",
                        "no puede estar vacío",
                    ));
                }
            }
            BlobBackend::S3 { bucket, region, .. } => {
                if bucket.trim().is_empty() {
                    return Err(ConfigValidationError::new(
                        "blob_backend.bucket",
                        "no puede estar vacío",
                    ));
                }
                if region.trim().is_empty() {
                    return Err(ConfigValidationError::new(
                        "blob_backend.region",
                        "no puede estar vacía",
                    ));
                }
            }
        }
        if self.google_client_id.trim().is_empty() {
            return Err(ConfigValidationError::new(
                "google_client_id",
                "es obligatorio y no puede estar vacío",
            ));
        }
        if self.google_jwks_url.trim().is_empty() {
            return Err(ConfigValidationError::new(
                "google_jwks_url",
                "no puede estar vacía",
            ));
        }
        if self.chunk_threshold_bytes == 0 {
            return Err(ConfigValidationError::new(
                "chunk_threshold_bytes",
                "debe ser mayor que cero",
            ));
        }
        if self.max_artifact_bytes == 0 {
            return Err(ConfigValidationError::new(
                "max_artifact_bytes",
                "debe ser mayor que cero",
            ));
        }
        if self.chunk_threshold_bytes > self.max_artifact_bytes {
            return Err(ConfigValidationError::new(
                "chunk_threshold_bytes",
                "no puede ser mayor que max_artifact_bytes",
            ));
        }
        if self.max_message_bytes == 0 {
            return Err(ConfigValidationError::new(
                "max_message_bytes",
                "debe ser mayor que cero",
            ));
        }
        if self.max_delta_nodes == 0 {
            return Err(ConfigValidationError::new(
                "max_delta_nodes",
                "debe ser mayor que cero",
            ));
        }
        if self.gc_interval.is_zero() {
            return Err(ConfigValidationError::new(
                "gc_interval",
                "debe ser mayor que cero",
            ));
        }
        if self.gc_grace_period.is_zero() {
            return Err(ConfigValidationError::new(
                "gc_grace_period",
                "debe ser mayor que cero",
            ));
        }
        Ok(())
    }
}

mod retention_policy_serde {
    use serde::{Deserialize, Deserializer, Serializer, de};

    use crate::domain::tenant::RetentionPolicy;

    pub fn serialize<S>(policy: &RetentionPolicy, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&policy.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<RetentionPolicy, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        raw.parse::<RetentionPolicy>().map_err(de::Error::custom)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::result_large_err)]
mod tests {
    use super::*;

    fn config_valida() -> Config {
        Config {
            google_client_id: "test-client-id".into(),
            ..Config::default()
        }
    }

    #[test]
    fn valores_por_defecto_coinciden_con_el_diseno() {
        let config = Config::default();
        assert_eq!(config.grpc_addr, "127.0.0.1:50051");
        assert_eq!(config.http_addr, "127.0.0.1:8080");
        assert_eq!(config.log_format, "text");
        assert_eq!(config.database_url, "sqlite://data/synapse.db?mode=rwc");
        assert_eq!(
            config.blob_backend,
            BlobBackend::Local {
                data_dir: PathBuf::from("data"),
            }
        );
        assert_eq!(
            config.google_jwks_url,
            "https://www.googleapis.com/oauth2/v3/certs"
        );
        assert!(config.allowed_accounts.is_empty());
        assert_eq!(config.default_retention, RetentionPolicy::Unlimited);
        assert_eq!(config.chunk_threshold_bytes, 4 * 1024 * 1024);
        assert_eq!(config.max_artifact_bytes, 2 * 1024 * 1024 * 1024);
        assert_eq!(config.max_message_bytes, 4 * 1024 * 1024);
        assert_eq!(config.max_delta_nodes, 10_000);
        assert_eq!(config.gc_interval, Duration::from_secs(3600));
        assert_eq!(config.gc_grace_period, Duration::from_secs(24 * 3600));
    }

    #[test]
    fn toml_sobrescribe_valores_por_defecto() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "synapse.toml",
                r#"
grpc_addr = "0.0.0.0:50052"
http_addr = "0.0.0.0:8081"
log_format = "json"
google_client_id = "mi-cliente-google"
default_retention = "versions:25"
gc_interval = "30m"
gc_grace_period = "48h"

[blob_backend]
type = "local"
data_dir = "/var/lib/synapse"
"#,
            )?;

            let config = Config::load(Some(Path::new("synapse.toml"))).unwrap();
            assert_eq!(config.grpc_addr, "0.0.0.0:50052");
            assert_eq!(config.http_addr, "0.0.0.0:8081");
            assert_eq!(config.log_format, "json");
            assert_eq!(config.google_client_id, "mi-cliente-google");
            assert_eq!(config.default_retention, RetentionPolicy::Versions(25));
            assert_eq!(config.gc_interval, Duration::from_secs(30 * 60));
            assert_eq!(config.gc_grace_period, Duration::from_secs(48 * 3600));
            assert_eq!(
                config.blob_backend,
                BlobBackend::Local {
                    data_dir: PathBuf::from("/var/lib/synapse"),
                }
            );
            config.validate().unwrap();
            Ok(())
        });
    }

    #[test]
    fn variables_de_entorno_sobrescriben_toml() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "synapse.toml",
                r#"
http_addr = "127.0.0.1:8081"
google_client_id = "desde-toml"
default_retention = "days:7"

[blob_backend]
type = "local"
data_dir = "desde-toml"
"#,
            )?;
            jail.set_env("SYNAPSE_HTTP_ADDR", "127.0.0.1:9090");
            jail.set_env("SYNAPSE_GOOGLE_CLIENT_ID", "desde-env");
            jail.set_env("SYNAPSE_DEFAULT_RETENTION", "days:14");
            jail.set_env("SYNAPSE_BLOB_BACKEND__DATA_DIR", "/tmp/synapse-blobs");

            let config = Config::load(Some(Path::new("synapse.toml"))).unwrap();
            assert_eq!(config.http_addr, "127.0.0.1:9090");
            assert_eq!(config.google_client_id, "desde-env");
            assert_eq!(config.default_retention, RetentionPolicy::Days(14));
            assert_eq!(
                config.blob_backend,
                BlobBackend::Local {
                    data_dir: PathBuf::from("/tmp/synapse-blobs"),
                }
            );
            config.validate().unwrap();
            Ok(())
        });
    }

    #[test]
    fn politicas_de_retencion_invalidas_fallan_al_cargar() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("SYNAPSE_DEFAULT_RETENTION", "versions:0");
            let err = Config::load(None).unwrap_err();
            assert!(
                err.to_string().to_lowercase().contains("default_retention"),
                "el error debería mencionar default_retention: {err}"
            );
            Ok(())
        });
    }

    #[test]
    fn validate_rechaza_cada_clave_invalida_con_su_nombre() {
        type Mutador = fn(&mut Config);
        let casos: &[(&str, Mutador)] = &[
            ("grpc_addr", |c| c.grpc_addr = "no-es-una-direccion".into()),
            ("http_addr", |c| c.http_addr = "999.999.0.1".into()),
            ("database_url", |c| c.database_url = "   ".into()),
            ("blob_backend.data_dir", |c| {
                c.blob_backend = BlobBackend::Local {
                    data_dir: PathBuf::new(),
                };
            }),
            ("blob_backend.bucket", |c| {
                c.blob_backend = BlobBackend::S3 {
                    bucket: "".into(),
                    region: "auto".into(),
                    endpoint: None,
                };
            }),
            ("blob_backend.region", |c| {
                c.blob_backend = BlobBackend::S3 {
                    bucket: "synapse".into(),
                    region: "".into(),
                    endpoint: None,
                };
            }),
            ("google_client_id", |c| c.google_client_id = "".into()),
            ("google_jwks_url", |c| c.google_jwks_url = "".into()),
            ("chunk_threshold_bytes", |c| c.chunk_threshold_bytes = 0),
            ("max_artifact_bytes", |c| c.max_artifact_bytes = 0),
            ("chunk_threshold_bytes", |c| {
                c.chunk_threshold_bytes = 200;
                c.max_artifact_bytes = 100;
            }),
            ("max_message_bytes", |c| c.max_message_bytes = 0),
            ("max_delta_nodes", |c| c.max_delta_nodes = 0),
            ("gc_interval", |c| c.gc_interval = Duration::ZERO),
            ("gc_grace_period", |c| c.gc_grace_period = Duration::ZERO),
        ];

        for &(clave_esperada, mutar) in casos {
            let mut config = config_valida();
            mutar(&mut config);
            let err = config.validate().unwrap_err();
            assert_eq!(
                err.key, clave_esperada,
                "error inesperado para {clave_esperada}: {err}"
            );
            assert!(
                err.to_string().contains(clave_esperada),
                "el mensaje debe incluir la clave '{clave_esperada}': {err}"
            );
        }
    }
}
