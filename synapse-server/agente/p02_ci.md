# P02 — CI y controles de seguridad del servidor

**Tareas OpenSpec:** 1.4. **Depende de:** P01 (puede ir en paralelo con P03/P04 si se trabaja en otra rama o
*worktree*: solo toca `.github/` y `synapse-server/deny.toml`).

## Objetivo

1. Comprobar que `ci/release.py` y `.github/workflows/release.yml` **no** empaquetan ni publican `synapse-server`
   (léelos; si algo recorre el repo entero buscando binarios o `Cargo.toml`, dilo). Resultado en el cierre, con las
   líneas relevantes.
2. Job nuevo `server` en `.github/workflows/ci.yml`, con `defaults.run.working-directory: synapse-server`:
   - `actions/checkout`, `dtolnay/rust-toolchain` (con `toolchain: stable` y `components: rustfmt, clippy`) y
     `swatinem/rust-cache` (`workspaces: "synapse-server -> target"`, `shared-key: server`), **fijados por el mismo
     SHA y comentario `# vX`** que ya usa el resto de `ci.yml`. Cópialos de ahí; no busques SHA nuevos.
   - `sudo apt-get update` y `sudo apt-get install --no-install-recommends -y mold protobuf-compiler`.
   - `env: SQLX_OFFLINE: "true"`.
   - Pasos: `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`.
   - Permisos mínimos (`permissions: contents: read`), `timeout-minutes` razonable, mismos disparadores que el resto
     del workflow. Si el workflow filtra por rutas, añade `synapse-server/**`.
   - Las pruebas e2e de la app son locales: no se tocan.
3. Seguridad:
   - `.github/dependabot.yml`: entrada `cargo` con `directory: "/synapse-server"`, semanal, `minor` y `patch`
     agrupados (copia el estilo de la entrada existente de `src-tauri`).
   - `.github/workflows/security.yml`: `cargo audit` (`rustsec/audit-check`) y `cargo deny`
     (`EmbarkStudios/cargo-deny-action`) también para `synapse-server` (matriz o pasos extra con
     `manifest-path: synapse-server/Cargo.toml` / `working-directory`, según lo que admita cada acción: compruébalo en
     su README de la versión fijada).
   - `synapse-server/deny.toml` análogo a `src-tauri/deny.toml`: **misma lista cerrada de licencias**, mismas reglas
     de fuentes y avisos. Ajusta solo lo que sea específico de `src-tauri` (excepciones de crates que el servidor no
     usa), explicando cada diferencia en un comentario.
   - `synapse-server/Cargo.toml` tiene `license = ""` y `publish = false`, igual que `src-tauri`; conserva
     `[licenses.private] ignore = true` en el `deny.toml` para que el propio crate no falle.
4. Validar todos los YAML tocados con
   `python3 -c "import yaml,sys; [yaml.safe_load(open(f)) for f in sys.argv[1:]]" <archivos>`.

## Fuera de alcance

Ejecutar `cargo deny`/`cargo audit`/`actionlint` en local (no están instalados: corren en CI). Cambiar otros jobs.

## Hecho cuando

YAML válidos, diff mínimo en los workflows, 1.4 marcada, CHANGELOG («Añadido: CI del servidor de sincronización»).
En el cierre di qué no pudiste comprobar sin ejecutar CI y recuerda al usuario que el job solo se ve en verde tras el
push.
