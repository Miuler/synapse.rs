//! Conexiones a gestores de issues: cada una con su servidor y su credencial, que solo viaja a él.

use crate::domain::issue::issue_error::{IssueError, IssueErrorKind as Kind};
use url::Url;

#[derive(Debug, Clone, PartialEq)]
pub struct Connection {
    /// Estable aunque cambie el nombre: `[a-z0-9-]`, da nombre a su entrada del llavero.
    pub id: String,
    pub provider: String,
    pub name: String,
    /// Dirección base, normalizada: la credencial solo se envía a ella.
    pub server: String,
    pub user: Option<String>,
    /// Referencia `op://` de 1Password. Un token pegado no se guarda aquí: va al llavero.
    pub secret: Option<String>,
}

impl Connection {
    /// Nombre de su credencial en el llavero.
    pub fn keyring_name(&self) -> String {
        format!("issues-{}", self.id)
    }
}

pub fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Un id a partir del nombre que no esté en `taken`.
pub fn new_id(name: &str, taken: &[&str]) -> String {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' => 'a',
            'é' => 'e',
            'í' => 'i',
            'ó' => 'o',
            'ú' | 'ü' => 'u',
            'ñ' => 'n',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        })
        .collect();
    let slug = slug
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let base = if slug.is_empty() {
        "conexion".to_string()
    } else {
        slug
    };
    let mut id = base.clone();
    let mut n = 2;
    while taken.contains(&id.as_str()) {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

/// La dirección tal como se guarda y se compara: HTTPS (HTTP solo hacia este equipo), sin
/// credenciales, consulta ni barra final.
pub fn normalize_server(server: &str) -> Result<String, IssueError> {
    let invalid = || IssueError::new(Kind::InvalidUrl, server);
    let url = Url::parse(server.trim()).map_err(|_| invalid())?;
    let host = url.host_str().ok_or_else(invalid)?;
    let local = matches!(host, "localhost" | "127.0.0.1" | "[::1]");
    let secure = url.scheme() == "https" || (url.scheme() == "http" && local);
    if !secure
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(invalid());
    }
    Ok(url.as_str().trim_end_matches('/').to_string())
}

fn segments(url: &Url) -> Vec<&str> {
    url.path_segments()
        .map(|s| s.filter(|s| !s.is_empty()).collect())
        .unwrap_or_default()
}

/// Si `scope` (la dirección de un proyecto) está dentro de `server`; devuelve cuánto lo concreta.
pub fn covers(server: &str, scope: &str) -> Option<usize> {
    let (server, scope) = (Url::parse(server).ok()?, Url::parse(scope.trim()).ok()?);
    if server.origin() != scope.origin() || !scope.username().is_empty() {
        return None;
    }
    let (base, path) = (segments(&server), segments(&scope));
    // Los nombres de organización y proyecto no distinguen mayúsculas
    (path.len() >= base.len()
        && base
            .iter()
            .zip(&path)
            .all(|(a, b)| a.eq_ignore_ascii_case(b)))
    .then_some(base.len())
}

/// La conexión de un tablero. `Ok(None)` si el proveedor no tiene ninguna configurada: se usa la
/// implícita con la credencial global anterior.
pub fn choose<'a>(
    connections: &'a [Connection],
    provider: &str,
    id: Option<&str>,
    scope: &str,
) -> Result<Option<&'a Connection>, IssueError> {
    let own: Vec<&Connection> = connections
        .iter()
        .filter(|c| c.provider == provider)
        .collect();
    if let Some(id) = id {
        return own
            .into_iter()
            .find(|c| c.id == id)
            .map(Some)
            .ok_or_else(|| IssueError::new(Kind::NoConnection, id));
    }
    if own.is_empty() {
        return Ok(None);
    }
    let matches: Vec<(usize, &Connection)> = own
        .into_iter()
        .filter_map(|c| covers(&c.server, scope).map(|depth| (depth, c)))
        .collect();
    let best = matches.iter().map(|(depth, _)| *depth).max();
    let mut found = matches.iter().filter(|(depth, _)| Some(*depth) == best);
    match (found.next(), found.next()) {
        (Some((_, connection)), None) => Ok(Some(connection)),
        (Some(_), Some(_)) => Err(IssueError::new(Kind::AmbiguousConnection, scope)),
        _ => Err(IssueError::new(Kind::NoConnection, scope)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connection(id: &str, server: &str) -> Connection {
        Connection {
            id: id.into(),
            provider: "azure_devops".into(),
            name: id.into(),
            server: server.into(),
            user: None,
            secret: None,
        }
    }

    #[test]
    fn el_servidor_exige_https_salvo_en_este_equipo() {
        assert_eq!(
            normalize_server(" https://dev.azure.com/org/ ").unwrap(),
            "https://dev.azure.com/org"
        );
        assert!(normalize_server("http://127.0.0.1:8080/org").is_ok());
        assert!(normalize_server("http://localhost/org").is_ok());
        for bad in [
            "http://tfs.local/org",
            "ftp://ejemplo.test",
            "https://usuario@ejemplo.test",
            "https://ejemplo.test/?x=1",
            "ejemplo.test",
        ] {
            assert_eq!(
                normalize_server(bad).unwrap_err().kind,
                Kind::InvalidUrl,
                "{bad}"
            );
        }
    }

    #[test]
    fn los_ids_son_seguros_y_no_se_repiten() {
        assert_eq!(new_id("Cliente Ñandú / 2", &[]), "cliente-nandu-2");
        assert_eq!(new_id("../..", &[]), "conexion");
        assert_eq!(new_id("Trabajo", &["trabajo", "trabajo-2"]), "trabajo-3");
        assert!(is_valid_id("trabajo-3"));
        assert!(!is_valid_id("../x") && !is_valid_id("") && !is_valid_id("A"));
    }

    #[test]
    fn un_servidor_solo_cubre_lo_que_esta_dentro() {
        let server = "https://dev.azure.com/org";
        assert_eq!(
            covers(server, "https://dev.azure.com/ORG/Proyecto"),
            Some(1)
        );
        for outside in [
            "https://dev.azure.com/otra/Proyecto",
            "https://dev.azure.com/organizacion/Proyecto",
            "http://dev.azure.com/org/Proyecto",
            "https://dev.azure.com:8443/org/Proyecto",
            "https://dev.azure.com.ejemplo.test/org/Proyecto",
            "https://dev.azure.com@ejemplo.test/org/Proyecto",
        ] {
            assert_eq!(covers(server, outside), None, "{outside}");
        }
    }

    #[test]
    fn cada_tablero_usa_la_conexion_de_su_servidor() {
        let all = [
            connection("amplia", "https://dev.azure.com"),
            connection("trabajo", "https://dev.azure.com/org"),
            connection("cliente", "https://dev.azure.com/otra"),
        ];
        let pick =
            |id, scope| choose(&all, "azure_devops", id, scope).map(|c| c.unwrap().id.clone());
        assert_eq!(
            pick(None, "https://dev.azure.com/org/P").unwrap(),
            "trabajo"
        );
        assert_eq!(
            pick(None, "https://dev.azure.com/otra/P").unwrap(),
            "cliente"
        );
        assert_eq!(
            pick(None, "https://dev.azure.com/tercera/P").unwrap(),
            "amplia"
        );
        assert_eq!(pick(Some("cliente"), "x").unwrap(), "cliente");
        let kind = |id, scope| pick(id, scope).unwrap_err().kind;
        assert_eq!(kind(Some("borrada"), "x"), Kind::NoConnection);
        assert_eq!(kind(None, "https://ejemplo.test/org/P"), Kind::NoConnection);
        let twins = [
            connection("a", "https://dev.azure.com/org"),
            connection("b", "https://dev.azure.com/org"),
        ];
        let error =
            choose(&twins, "azure_devops", None, "https://dev.azure.com/org/P").unwrap_err();
        assert_eq!(error.kind, Kind::AmbiguousConnection);
        // Sin conexiones de ese proveedor, la implícita
        assert_eq!(choose(&all, "github", None, "x").unwrap(), None);
    }
}
