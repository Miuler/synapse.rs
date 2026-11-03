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

## Abiertos (en orden de trabajo)

1. **Cómo se migra el código existente**: orden y límites de cada etapa; el test
   de arquitectura crece en cada paso. Antes de empezar se miden tres operaciones
   (abrir la bóveda, búsqueda rápida, guardar una nota grande) y se repite las
   medición al final de cada etapa para detectar regresiones en la etapa que las causa.
2. **Fachada del API de Obsidian (front, `src/shared/obsidian-api`)**. Propuesta aceptada
   como prueba, a validar con mediciones:
   - Un solo índice de la bóveda en el front: `Map` plano de ruta a `{mtime, size}`, sin
     contenido, compartido por el explorador de Synapse y la fachada. Reemplaza a
     `vaultItems` (`$state` profundo) y a `_cachedFiles` de `SynapseVault`. Se llena al abrir
     la bóveda, antes de cargar plugins, y se actualiza con los eventos del vigilante. El
     `DashMap` de Rust sigue siendo la fuente de verdad.
   - `getAbstractFileByPath`, `getActiveFile` y `metadataCache.getFileCache` pasan a ser
     síncronos (como en Obsidian); `TFile` se crea al pedirlo. El contenido sigue asíncrono.
   - Con 6 500 notas y rutas de ~150 caracteres el índice ocupa ~1,6 a 2 MB, frente a una
     estimación de 5 a 8 MB actual. Medir con perfil de memoria del webview antes y después.
     Tabla de carpetas (carpeta una vez + nombre por archivo) solo si se superan ~50 000 notas.
   - Verificar firmas contra los tipos oficiales del paquete `obsidian` (solo tipos).
   - Sacar lo específico de Excalidraw del núcleo de la fachada a su propia carpeta.
   - Documentar en `verified-plugins.jsonc` el límite de Tauri: sin Node ni Electron en el
     webview.

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
- **Quick Open (previo):** al buscar, los separadores "Recientes"/"Archivos" asumen que los
  recientes van primero, pero los resultados se ordenan por puntaje y se mezclan, y aparece
  "Archivos" dos veces (`QuickOpen.svelte`, bloque de separadores de sección).
- **Memoria real del webview:** medir el heap con la app abierta (lo medido es sintético en
  JavaScriptCore).
- **Quick Open:** los empates con el mismo nombre y puntaje dependen del orden del `DashMap`.
- **`read_pdf_document`:** si falta `search_query` y además el PDF no se puede leer, ahora
  responde primero el error de argumento (antes, el de lectura).
- **`serde` en el dominio:** sigue como excepción temporal (fuera de esta ronda).
- **Exportaciones de Obsidian que faltan:** 59 de 123, listadas en la instantánea de
  `obsidian-compat.test.ts`.
