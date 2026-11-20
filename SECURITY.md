# Política de seguridad

## Versiones con soporte

Synapse está en beta (`1.0.0-beta.x`). Solo se corrigen vulnerabilidades en la **última release publicada**.

## Reportar una vulnerabilidad

**No abras un issue público.** Usa el reporte privado de GitHub:

1. Entra en la pestaña **Security** del repositorio → **Report a vulnerability**
   (<https://github.com/Miuler/synapse.rs/security/advisories/new>).
2. Describe el problema, los pasos para reproducirlo y la versión afectada.

Respuesta inicial esperada en unos días; es un proyecto personal mantenido por una sola persona. Si el problema se confirma, se publica la corrección y un aviso (*security advisory*) con el crédito a quien lo reportó, si lo desea.

> Requisito del repositorio: el reporte privado debe estar activado en **Settings → Code security → Private vulnerability reporting**.

## Qué se considera una vulnerabilidad

Ejemplos: ejecución de código a través de una nota, un plugin o una respuesta de red; acceso a archivos fuera de la bóveda; fallos en el servidor MCP local o en la verificación de actualizaciones.

El mapa de conexiones de red de la app, con lo que se envía en cada una, está en el [README](README.md#-seguridad-y-privacidad). Los límites conocidos que figuran ahí (plugins comunitarios descargados sin verificar hash, servidor MCP local sin autenticación, CSP desactivado) son decisiones de diseño actuales y se aceptan como reportes de mejora, no como secreto.

## Verificar un binario

Cada release publica, junto a los paquetes (`.tar.gz` / `.zip`), estos archivos:

| Archivo | Para qué sirve |
|---|---|
| `SHA256SUMS.txt` | Sumas SHA-256 de los paquetes, sus firmas, el SBOM y `latest.json` |
| `*.cdx.json` | SBOM CycloneDX del backend Rust y del frontend |
| `*.sig` | Firma del actualizador (minisign de Tauri) de cada paquete |

### 1. Integridad: `sha256sum`

Descarga el paquete y `SHA256SUMS.txt` en la misma carpeta y comprueba:

```bash
sha256sum --check --ignore-missing SHA256SUMS.txt
```

Debe mostrar `OK` para cada archivo descargado. En macOS usa `shasum -a 256 -c SHA256SUMS.txt`.

### 2. Procedencia: `gh attestation verify`

Cada paquete tiene una *attestation* de procedencia generada por GitHub Actions y firmada con Sigstore. Con la [CLI de GitHub](https://cli.github.com/):

```bash
gh attestation verify synapsers-linux-x86_64.tar.gz --repo Miuler/synapse.rs
```

Confirma que el archivo lo construyó el workflow de release de este repositorio a partir del tag publicado. No necesitas instalar Cosign.

### 3. Firma del actualizador (minisign)

Los `.sig` los genera `tauri signer sign` con la clave privada del proyecto. La clave pública que usa el actualizador está en [`src-tauri/tauri.conf.json`](src-tauri/tauri.conf.json) (`plugins.updater.pubkey`), en el formato de Tauri: el archivo de clave pública de minisign codificado en base64. El `.sig` es, igualmente, una firma minisign en base64. Para comprobarla a mano con [minisign](https://jedisct1.github.io/minisign/):

```bash
jq -r .plugins.updater.pubkey src-tauri/tauri.conf.json | base64 -d > synapse.pub
base64 -d synapsers-linux-x86_64.tar.gz.sig > synapsers-linux-x86_64.tar.gz.minisig
minisign -V -p synapse.pub -m synapsers-linux-x86_64.tar.gz -x synapsers-linux-x86_64.tar.gz.minisig
```

## Cómo se protege el proceso de release

- Workflows con permisos `contents: read` por defecto; solo el job que publica la release los eleva.
- Acciones de terceros fijadas por SHA de commit y actualizadas por Dependabot.
- Auditoría semanal y en cada PR: `cargo audit`, `cargo deny` (avisos, licencias, fuentes), `bun audit` y CodeQL.
- OpenSSF Scorecard publicado en el README.
