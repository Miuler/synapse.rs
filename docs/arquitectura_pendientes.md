# Pendientes de la arquitectura (se cierran de a uno)

Documento de trabajo. Las reglas ya cerradas están en `arquitectura_cebolla.md`.

## Cerrados

- Repository vive en `domain`; `application/service` solo para servicios externos.
- Regla de propiedad para distinguir repository de service.
- Nombres: contrato + `Impl`; segunda implementación con calificador delante.
- Casos de uso: una clase por intención, sufijo `UseCase`, método `execute`.
- Capacidades intercambiables (ej. búsqueda difusa) son contratos.
- Dos destinos para los mismos datos: dos instancias del mismo contrato si el negocio
  debe enterarse del fallo.
- `Command` (escritura) o `Query` (lectura) obligatorio en cada caso de uso, aunque
  lleve un solo dato; la firma es siempre `execute(command)`.
- Nombres del dominio para el versionado (opción B, vocabulario de producto):
  `GitFileStatus`→`FileVersionState`, `VaultGitStatus`→`VaultVersionState`,
  `GitCommitLogItem`→`VersionEntry`, `GitFileLogResult`→`FileVersionHistory`,
  `GitBranchItem`→`Branch`, `GitDiffResponse`→`FileDiff`. El contrato `VersionControl`
  usa `track`, `revert`, `save_version`. "git" solo queda en `infrastructure`
  (`GitCli`) y en los nombres de comandos Tauri que usa el front.
- Documento unificado: `arquitectura_cebolla.md` (base: la última versión del autor, más
  la regla de propiedad, nombres, dos destinos y capacidades intercambiables).
- Sin la palabra `adapter`: en `infrastructure` van `entrypoint/`, `repository/`,
  `publisher/` y `service/`, con la tecnología en la subcarpeta.
- `Impl` para todas las implementaciones, incluidos los servicios externos.
- No existe `application/handler`: los suscriptores de eventos son puntos de entrada en
  `infrastructure/entrypoint/subscriber` que llaman directamente al caso de uso.
- `EventPublisher` es un contrato en `domain/publisher`.
- Observador de cambios: `VaultChangeObserver` desaparece. El vigilante de archivos es un
  `subscriber` en `infrastructure/entrypoint/subscriber` que llama directamente a los casos
  de uso (`IndexVaultChangeUseCase`, `RefreshNavigationUseCase`). Sin evento de dominio ni
  `EventPublisher` hasta que un caso de uso necesite anunciar un hecho de negocio.
- Rendimiento: sección 11 del documento (mappers que consumen en vez de clonar, contratos
  por lotes cuando el volumen es grande, bucles críticos enteros dentro de su `Impl`).
- Librerías permitidas en `domain` y `application`: las que solo eliminan código repetitivo
  (Lombok, `thiserror`) y las puras que implementan un estándar, con lista explícita en el test.
  `pulldown_cmark` queda permitida por ahora, con un `TODO:` en `tag_service.rs` que explica
  la duda (opciones A y B para sacarla).
- Servicio de dominio (`domain/service`): solo si la regla involucra más de una entidad;
  sin estado, sin I/O ni repositorios, nombrado por la operación. En Synapse quedan
  `FolderSummarizer` (de `folder_service::summarize_folder`) y `LinkResolver` (resolución de
  destino y `calculate_proximity_score`). El resto pasa a objetos de valor y entidades:
  `copy_naming` al nombre de archivo, `extract_ancestor_folders` a `NotePath::ancestors`,
  `frontmatter_service` a `Note`, wikilinks/destinos/extensiones a `Wikilink`, `LinkTarget`
  y `AssetPath`, y `tag_service` a `Tag` y al contenido de la nota.
- `serde` en el dominio: se permite por ahora como excepción temporal de Synapse; la regla
  del documento no cambia. Queda un `TODO:` en `src-tauri/src/domain/mod.rs` con la duda
  (migrar por etapas a DTOs en `infrastructure` sin copias extra en respuestas grandes).
- Motor de navegación (`navigation/engine.rs`): queda solo como implementación del
  repository de catálogo de notas (caché en memoria, disco y reconciliación). Sale de ahí:
  vigilante de archivos → `entrypoint/subscriber`; notas recientes, pestañas, modo de vista y
  estado de interfaz → repository de sesión con sus casos de uso; resolución de enlaces →
  `LinkResolver`; título de la nota (frontmatter, primer H1, nombre) → método de `Note`.
- Quick Open (opción A): la política de prioridad (pesos por campo y bonus de recencia) pasa
  al dominio como `QuickOpenRanking`; la coincidencia difusa con `nucleo` queda en la
  implementación. **Condición: conservar la regla exacta** (mismas constantes, misma fórmula,
  mismo orden de resultados). Antes de moverla se escriben pruebas de caracterización con
  notas y consultas fijas que capturen el orden actual, y deben pasar idénticas después.
- Servidor MCP: es un punto de entrada (Synapse es el servidor; los clientes de IA lo
  llaman). `mcp/server.rs` pasa a `infrastructure/entrypoint` (p. ej. `controller/mcp`), junto
  con `port_registry.rs` (mecánica del punto de entrada). Los tipos de `domain/models/mcp.rs`
  son del protocolo JSON-RPC: salen del dominio como DTOs del punto de entrada.
  `McpUseCases` se trocea en un caso de uso por herramienta, reutilizando los que ya usa el
  front, y su I/O directo (`std::fs`, `pdf_extractor`) pasa a contratos. Si en el futuro
  Synapse actúa como cliente MCP, ese cliente sí sería un servicio en `application/service`.
- Los casos de uso no tienen contrato de entrada (no va a cambiar).
- El arranque y el ensamblado de dependencias viven en `infrastructure`
  (en Synapse es una app de escritorio con Tauri). `domain` y `application` solo
  contienen negocio y casos de uso.

## Abiertos

Ninguno del plan de migración: la medición por etapas y la fachada de Obsidian se cerraron (ver
«Deuda cerrada en la segunda ronda» y `arquitectura_mediciones.md`). Las tareas de rendimiento que
quedan están en beads: virtualizar el árbol del explorador e `indexed_versions` de Tantivy.

## Postergado (fuera del plan hasta terminar el refactor)

- **Agente ACP.** No forma parte del plan de migración. Conclusiones ya tomadas, para no
  perderlas: Synapse→agente (iniciar, enviar prompt, cancelar) es un servicio externo en
  `application/service`; agente→Synapse (leer, escribir, permisos, avances) es un punto de
  entrada en `infrastructure/entrypoint` que llama a casos de uso.

## Deuda cerrada en la segunda ronda

- `log` permitido en casos de uso; prueba de arranque en frío estable (mediana de 5).
- Dominio sin funciones libres públicas (Tag, LinkTarget, AssetPath, WikilinkToken,
  ProximityScore, NoteFrontmatter, FileName, FolderPath, FolderSummarizer).
- `WorkspaceSessionRepository` con 8 casos de uso de sesión.
- `VaultChangeObserver` y `domain/event` eliminados: el índice se mantiene con
  `IndexVaultChangeUseCase` sobre `IndexedDocumentRepository`.
- MCP: 18 herramientas sobre casos de uso; el manejador solo traduce JSON (instantánea idéntica).
- Front: `vaultItems` como `$state.raw` (~11,9 → ~2,0 MB con 6 500 notas), `getActiveFile` y
  `getFileCache` síncronos, Excalidraw en `integrations/excalidraw`, mapa de compatibilidad con
  los tipos oficiales de `obsidian`.

## Deuda que queda

- **Índice único front (hecho):** el backend envía una sola lista (`VaultEntryDto`, con metadatos de
  archivo); el store de `entities/vault-item` la guarda y la fachada de Obsidian construye su
  índice desde ahí (`indexSource`). La fachada todavía mantiene su propio `Map` (~2 MB) para
  responder síncrono a los plugins.
- **`read_pdf_document`:** si falta `search_query` y además el PDF no se puede leer, ahora
  responde primero el error de argumento (antes, el de lectura).
- **`serde` en el dominio:** sigue como excepción temporal (fuera de esta ronda).
- **Exportaciones de Obsidian que faltan:** 59 de 123, listadas en la instantánea de
  `obsidian-compat.test.ts`.

## Revisión con la bóveda real (MisNotas, ~6 000 notas, `e2e/real-vault.test.ts`)

Memoria (RSS) de la app instalada (build de producción) con la misma bóveda:

| Proceso | RSS |
|---|---|
| Backend (`synapsers`) | 640–730 MB |
| Webview (`WebKitWebProcess`) | 460 MB – 1 GB por ventana |
| Red de WebKit | 90–100 MB |

Hallazgos, por prioridad:

1. **Imágenes por IPC en base64 (hecho):** las notas y el visor de imágenes usan el protocolo de
   assets de Tauri; la data URL queda como último recurso. Probado con rutas con emojis, espacios,
   `@`, acentos y `#` (`e2e/assets.test.ts`).
1b. **Comandos de lectura fuera del hilo principal (hecho):** 36 comandos de lectura pasaron a
   `#[tauri::command(async)]`; las escrituras siguen en el hilo principal, en orden. Mejora modesta
   (IPC acumulado al abrir: 16,5 s → 13,6–15,3 s), porque no era la causa principal.
1c. **Congestión al arrancar (resuelta en parte):** en reposo todas las llamadas del arranque suman
   ~130 ms; el resto era espera por el hilo de JS del webview (`e2e/startup-profile.test.ts`). Con 5
   pestañas restauradas el hilo se bloqueaba ~1,25 s; ahora cada pestaña se monta al activarse por
   primera vez y el explorador no pinta el árbol de respaldo: ~0,8 s. Queda el pintado del
   explorador con carpetas expandidas (~300 ms, tarea de virtualización en beads).
1d. **Estado de git (descartado):** en producción `get_vault_git_status` responde en ~22 ms en reposo;
   los ~330 ms eran del build debug.
2. **Memoria del backend (hecho):** ~370 MB eran memoria liberada que glibc retenía tras los
   recorridos paralelos al abrir la bóveda. `malloc_trim` al terminarlos deja el backend en ~400 MB
   de RSS (196–239 MB anónimos). Lo vivo de verdad: ~200 MB.
3. **Lecturas de notas lentas (descartado):** en reposo `read_note_content` tarda 1–3 ms y
   `render_markdown_wikilinks` ~7 ms; lo lento era la congestión (1c).
4. **Segunda instancia sobre la misma bóveda (hecho):** no persiste el puerto si otra instancia viva
   es dueña de la bóveda o si el puerto configurado sigue ocupado.
5. **Arranque en modo dev:** 34 s hasta ver el explorador, dominado por la compilación en frío de
   Vite. En producción el explorador aparece a los **1,7 s** (incluye arrancar la app): no es un
   problema del producto.

Quick Open con la bóveda real: 80–110 ms por búsqueda, medidos dentro de la página con la consulta
completa (la medición anterior contaba consultas a medias). Sin errores ni avisos de consola.
