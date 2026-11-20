# P06 — Autenticación con Google y tenants

**Tareas OpenSpec:** 3.1. **Depende de:** P03 y P05.

## Objetivo

1. **Contrato** `IdTokenVerifier` en `application/integration/service/id_token_verifier.rs`:
   `async fn verify(&self, token: &str) -> Result<VerifiedIdentity, AuthError>` donde `VerifiedIdentity` tiene `sub`,
   `email` y `email_verified`. `AuthError`: `Missing`, `Invalid`, `Expired`, `WrongAudience`, `Unavailable`.
2. **Implementación** `GoogleIdTokenVerifier` en `infrastructure/integration/service/google/` con `jsonwebtoken`
   (comprueba en docs.rs si la versión actual exige elegir proveedor criptográfico por *feature*; prefiere el de Rust
   puro, `rust_crypto`, salvo que `cargo deny` lo impida) y `reqwest` con `rustls` y timeouts explícitos:
   - Solo RS256; `kid` obligatorio; `iss` ∈ {`accounts.google.com`, `https://accounts.google.com`}; `aud` =
     `google_client_id`; `exp` con 60 s de tolerancia.
   - Caché del JWKS en memoria (`tokio::sync::RwLock`) con caducidad según `Cache-Control: max-age`; si llega un `kid`
     desconocido y la caché tiene más de 5 min, refresca una vez (rotación de claves). Una sola descarga simultánea
     aunque lleguen muchas llamadas a la vez.
   - Si la caché venció y la descarga falla, `Unavailable` (nunca aceptes sin verificar). Si la caché sigue vigente,
     úsala aunque Google no responda.
   - URL del JWKS configurable (`google_jwks_url`), así las pruebas usan un servidor local.
3. **Caso de uso** `ResolveTenantUseCase` (`application/use_case/`): con la identidad verificada y la lista
   `allowed_accounts` (coincide si contiene el `sub`, o el email **solo si** `email_verified`), busca el tenant por
   `sub` o lo crea con `default_retention`; cuenta no permitida → error `PermissionDenied` **sin crear tenant**. Alta
   concurrente del mismo `sub`: dos llamadas simultáneas acaban en un solo tenant (maneja el conflicto de `UNIQUE`
   releyendo).
4. **Capa `tower`** (`infrastructure/entrypoint/controller/auth_layer.rs`) aplicada **solo** al servicio `Sync` (no a
   salud ni reflexión): extrae `authorization: Bearer …` de los metadatos, llama al verificador y al caso de uso, e
   inserta `AuthenticatedTenant` en las extensiones de la petición. `AuthenticatedTenant` tiene campo privado y
   constructor visible solo dentro del módulo de autenticación; los controllers obtienen el `TenantId` con un helper
   `tenant_of(&request) -> Result<TenantId, Status>`.
   - El token se valida **al iniciar** la llamada; un stream ya aceptado no se corta si el token caduca a mitad.
   - Nunca registres el token; registra `tenant_id` y el resultado.
5. Mapeo de `AuthError`/`PermissionDenied` en `status.rs` según la tabla del `design.md`.
6. Amplía el test de arquitectura: `TenantId` solo se construye en el módulo de autenticación, en
   `infrastructure/repository/` (al leer filas) y en pruebas.

## Arnés de pruebas

- Clave RSA 2048 **solo de pruebas** generada una vez con `openssl` y versionada en
  `tests/integration/support/fixtures/` (`test_rsa_private.pem` y el JWKS público correspondiente), con un `README.md`
  que diga que no es un secreto y cómo regenerarla. Dilo en el cierre: el usuario decide si prefiere generarla en cada
  ejecución.
- `support::auth`: servidor `axum` local que sirve el JWKS (con `Cache-Control` configurable y un contador de
  peticiones) y un emisor de tokens de prueba (`sub`, `email`, `aud`, `iss`, `exp` y `kid` configurables).
- `TestServer` gana `google_jwks_url` apuntando a ese servidor y un cliente gRPC con interceptor que añade el token.
- Para tocar algún método de `Sync` antes de que exista su lógica, usa `CheckState` sobre una bóveda inexistente:
  con token válido debe llegar a `UNIMPLEMENTED`/`NOT_FOUND`, nunca a `UNAUTHENTICATED`.

## Pruebas exigidas (una por escenario de «Autenticación con Google» y «Aislamiento por tenant»)

- `auth::sin_token_responde_unauthenticated`, `auth::token_caducado_…`, `auth::audiencia_distinta_…`,
  `auth::emisor_distinto_…`, `auth::firma_con_otra_clave_…`, `auth::kid_desconocido_…`.
- `auth::cuenta_no_permitida_responde_permission_denied_y_no_crea_tenant`.
- `auth::email_no_verificado_no_cuenta_para_la_lista`.
- `auth::primer_acceso_crea_tenant_con_retencion_por_defecto` (y el segundo acceso no crea otro).
- `auth::altas_simultaneas_del_mismo_sub_crean_un_tenant`.
- `auth::jwks_inaccesible_y_caducado_responde_unavailable`; `auth::jwks_vigente_se_usa_aunque_google_no_responda`;
  `auth::jwks_se_descarga_una_vez_con_cache_control`.
- `auth::salud_grpc_no_exige_token`.
- El escenario «Bóveda de otro tenant» se prueba en P07 y P09 (aquí aún no hay bóvedas).

## Fuera de alcance

Flujo OAuth del cliente, refresco de tokens, tokens propios.

## Hecho cuando

Verificación en verde, 3.1 marcada (indicando que «bóveda ajena → `NOT_FOUND`» se cubre en P07/P09), CHANGELOG.
