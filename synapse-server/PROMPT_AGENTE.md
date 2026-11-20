# Pilotar a Antigravity con `synapse-server`

Esta guía es para ti, no para el agente. Al agente le pasas un mensaje corto que le apunta a dos archivos:
`agente/00_contexto.md` (reglas comunes, igual en todas las sesiones) y la petición de esa sesión
(`agente/pNN_*.md`). La fuente de verdad del comportamiento es el OpenSpec
`openspec/changes/add-sync-server/` (`design.md`, `specs/`, `tasks.md`).

## Las peticiones

Una petición = una conversación nueva de Antigravity = un commit (o dos) que revisas antes de aprobar.

| Petición                                         | Tareas OpenSpec     | Depende de     | Riesgo   | Modelo sugerido |
|--------------------------------------------------|---------------------|----------------|----------|-----------------|
| [P01 Base](agente/p01_base.md)                   | 1.1, 1.2, 1.3       | —              | bajo     | estándar        |
| [P02 CI y seguridad](agente/p02_ci.md)           | 1.4                 | P01            | bajo     | estándar        |
| [P03 Contrato gRPC](agente/p03_contrato_grpc.md) | 2.1                 | P01            | medio    | el más capaz    |
| [P04 Merkle](agente/p04_merkle.md)               | 2.2                 | P01            | **alto** | el más capaz    |
| [P05 Persistencia](agente/p05_persistencia.md)   | 2.3, 2.4            | P01, P04       | medio    | el más capaz    |
| [P06 Autenticación](agente/p06_autenticacion.md) | 3.1                 | P03, P05       | **alto** | el más capaz    |
| [P07 Artefactos](agente/p07_artefactos.md)       | 3.2, 3.3            | P06            | medio    | estándar        |
| [P08 Bloques](agente/p08_chunking.md)            | 3.4                 | P07            | medio    | estándar        |
| [P09 Push/Pull](agente/p09_push_pull.md)         | 3.5, 3.6, 4.1       | P04, P07, P08  | **alto** | el más capaz    |
| [P10 Historial y GC](agente/p10_historial_gc.md) | 4.2, 4.3, 4.4       | P09            | **alto** | el más capaz    |
| [P11 Índices, e2e, docs](agente/p11_indices_e2e_docs.md) | 5.1, 5.2, 6.1, 3.7, 6.3 | P10 | medio    | estándar        |
| [P12 S3/R2](agente/p12_s3.md)                    | 6.2                 | P05 (opcional) | bajo     | estándar        |

Orden recomendado: P01 → P02 → P03 → P04 → P05 → P06 → P07 → P08 → P09 → P10 → P11; P12 cuando vayas a desplegar.
P02 y P04 son independientes de P03: si quieres paralelizar, solo en otra rama o *worktree*, nunca dos agentes
editando la misma carpeta.

## Mensaje de arranque (cópialo en cada conversación nueva, cambiando NN)

```text
Vas a trabajar en el repo synapse.rs, en la carpeta synapse-server/.

1. Lee entero synapse-server/agente/00_contexto.md: son tus reglas para toda la sesión.
2. Lee synapse-server/agente/pNN_<nombre>.md: es tu única petición en esta conversación.
3. Lee las secciones del OpenSpec (openspec/changes/add-sync-server/) que la petición menciona.
4. Antes de escribir código, preséntame un plan corto: archivos que vas a crear o tocar, dependencias que vas a
   añadir (con versión) y la lista de pruebas que vas a escribir, cada una con el escenario de la spec que cubre.
   Espera mi visto bueno.
5. Implementa, pasa la verificación obligatoria y cierra con el formato de cierre de 00_contexto.md.
   No hagas commit sin mi aprobación.
```

## Cómo pilotar (recomendaciones)

1. **Conversación nueva por petición.** El contexto limpio evita que arrastre suposiciones de la petición anterior; lo
   que debe recordar está en los archivos, no en el chat.
2. **Modo de planificación activado** y revisa el plan antes de aprobar: es el momento más barato para corregir.
   Mira sobre todo las dependencias nuevas, que la lista de pruebas cubra **todos** los escenarios de la petición y
   que no toque archivos fuera de lo permitido.
3. **Permisos del terminal en «pedir revisión»** para comandos que no sean `cargo`/`rg`/`fd`/`git status`/`git diff`.
   Sobre todo `git commit`, `git push`, `cargo install` y cualquier `rm -rf`.
4. **Revisión entre peticiones.** Antes de aprobar el commit:
   - Ejecuta tú la verificación en `synapse-server/` (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
     `cargo test`).
   - Pide a Claude Code una revisión del diff (`/code-review` sobre los cambios sin commitear) y, en P04, P06, P09 y
     P10, una segunda lectura de que el código cumple el `design.md`. Son las piezas donde un error silencioso
     (raíz mal calculada, tenant mal aislado, GC que borra de más) cuesta caro.
   - Comprueba que `tasks.md` y `CHANGELOG.md` se actualizaron en el mismo commit.
5. **Si se desvía del OpenSpec**, que lo cambie primero en el OpenSpec (`openspec validate add-sync-server --strict`)
   y después en el código. Si la desviación es grande, para la petición y tráela a Claude Code para decidir.
6. **Si se atasca** (la misma prueba falla tras dos intentos, o empieza a añadir `sleep`, `#[ignore]` o a debilitar
   comprobaciones), corta la conversación, apunta lo aprendido en la petición y empieza una nueva.
7. **Al terminar P11**, archiva el cambio de OpenSpec (`openspec archive add-sync-server`, o la skill
   `openspec-archive-change`) para que las specs pasen a `openspec/specs/`.

## Decisiones tomadas al revisar el OpenSpec (2026-11-20)

El OpenSpec validaba, pero dejaba huecos que el agente habría rellenado inventando. Se decidió (todo está ya en
`design.md`, `specs/` y `tasks.md`):

- `vault_id` único en todo el servidor: así «bóveda de otro tenant → `NOT_FOUND`» es posible; leer una bóveda ajena o
  inexistente da la misma respuesta.
- Regla Merkle exacta (rutas, codificación de hijos, `EMPTY_ROOT`), directorios implícitos calculados por el servidor,
  borrado de subárbol y `moved_from` para registrar movimientos.
- Chunking hecho por el cliente con una RPC nueva, `DeclareManifest`; el servidor verifica el hash del total.
- Blobs separados por tenant en la ruta física (la deduplicación es por tenant; el GC no toca otros tenants y
  `FindMissing` no filtra contenido ajeno).
- `last_seen_at` en `blobs` para el periodo de gracia del GC.
- Tabla de códigos de error con detalles `google.rpc` (`tonic-types`).
- Autenticación como capa `tower` asíncrona (el interceptor de `tonic` es síncrono) más contrato `IdTokenVerifier`.
- Solo SQLite en este cambio; PostgreSQL, en uno posterior.
- `lib.rs` + `main.rs` para probar en el mismo proceso; la configuración completa (1.2) estaba sin hacer.
- Retención por defecto `unlimited` hasta fijar los planes; `versions:N` cuenta la versión vigente.
