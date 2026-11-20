//! Política de retención de versiones por tenant.

use std::fmt;
use std::str::FromStr;

use thiserror::Error;

/// Política de retención de versiones de un tenant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RetentionPolicy {
    /// Conserva todas las versiones sin límite.
    #[default]
    Unlimited,
    /// Conserva por ruta las `N` versiones más recientes (contando la vigente), con `N >= 1`.
    Versions(u32),
    /// Conserva las versiones de los últimos `N` días (siempre respetando la vigente), con `N >= 1`.
    Days(u32),
}

/// Error al parsear una política de retención.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error(
    "política de retención inválida '{raw}': debe ser 'unlimited', 'versions:N' o 'days:N' con N >= 1"
)]
pub struct InvalidRetentionPolicy {
    raw: String,
}

impl FromStr for RetentionPolicy {
    type Err = InvalidRetentionPolicy;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        if trimmed == "unlimited" {
            return Ok(Self::Unlimited);
        }
        if let Some(rest) = trimmed.strip_prefix("versions:") {
            let n: u32 = rest
                .parse()
                .map_err(|_| InvalidRetentionPolicy { raw: s.to_owned() })?;
            if n >= 1 {
                return Ok(Self::Versions(n));
            }
        }
        if let Some(rest) = trimmed.strip_prefix("days:") {
            let n: u32 = rest
                .parse()
                .map_err(|_| InvalidRetentionPolicy { raw: s.to_owned() })?;
            if n >= 1 {
                return Ok(Self::Days(n));
            }
        }
        Err(InvalidRetentionPolicy { raw: s.to_owned() })
    }
}

impl fmt::Display for RetentionPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unlimited => f.write_str("unlimited"),
            Self::Versions(n) => write!(f, "versions:{n}"),
            Self::Days(n) => write!(f, "days:{n}"),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn parsea_politicas_validas() {
        assert_eq!(
            "unlimited".parse::<RetentionPolicy>().unwrap(),
            RetentionPolicy::Unlimited
        );
        assert_eq!(
            "versions:1".parse::<RetentionPolicy>().unwrap(),
            RetentionPolicy::Versions(1)
        );
        assert_eq!(
            "versions:50".parse::<RetentionPolicy>().unwrap(),
            RetentionPolicy::Versions(50)
        );
        assert_eq!(
            "days:30".parse::<RetentionPolicy>().unwrap(),
            RetentionPolicy::Days(30)
        );
        assert_eq!(RetentionPolicy::Unlimited.to_string(), "unlimited");
        assert_eq!(RetentionPolicy::Versions(10).to_string(), "versions:10");
        assert_eq!(RetentionPolicy::Days(7).to_string(), "days:7");
    }

    #[test]
    fn rechaza_politicas_invalidas() {
        for invalida in [
            "",
            "versions:0",
            "days:0",
            "versions:-1",
            "days:abc",
            "forever",
            "versions:",
        ] {
            assert!(
                invalida.parse::<RetentionPolicy>().is_err(),
                "debería rechazar '{invalida}'"
            );
        }
    }
}
