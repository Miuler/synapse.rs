# Mediciones del refactor de arquitectura

Comando (modo release, contra una bóveda real; solo lee la bóveda):

```bash
SYNAPSE_BENCH_VAULT=/ruta/a/la/boveda cargo test --release --test baseline_measurements -- --ignored --nocapture
```

Bóveda: MisNotas (6 053 notas indexadas). Valores = mediana.

| Etapa | Abrir bóveda (frío) | Búsqueda rápida | Guardar nota 2 MB |
|---|---|---|---|
| 0 · línea base (main 3ac005a) | 139,1 ms | 13,8 ms | 1,29 ms |
| 1 · renombrados y movimientos | 133,3 ms | 13,7 ms | 1,49 ms |
| 2 · sin violaciones duras | 132,5 ms | 13,8 ms | 1,47 ms |
| 3a · casos de uso de archivos, versionado y sistema | 138,6 ms | 13,6 ms | 1,19 ms |
| 4 · motor de navegación (ranking, título, enlaces) | 139,5 ms | 14,4 ms | 1,27 ms |
| Ronda 2 completa (sesión, observador, MCP, VO) | 130,4–138,0 ms | 14,2 ms | 1,44–1,53 ms |

Hallazgo de la etapa 0: en el Quick Open, los empates entre archivos con el mismo nombre y el
mismo puntaje (p. ej. `a/notas.md` y `b/notas.md`) se ordenan por el orden de iteración del
`DashMap`, que cambia entre ejecuciones. No se corrige en el refactor (cambiaría el
comportamiento); queda fuera de las pruebas de caracterización.

## Front: memoria de la lista de la bóveda

Medido en Bun (JavaScriptCore, el mismo motor que el webview de Tauri en Linux) con 6 500
elementos y rutas de ~150 caracteres, leyendo cada propiedad como hace la interfaz:

| `vaultItems` | Memoria |
|---|---|
| `$state` profundo (antes) | 11,9 MB |
| `$state.raw` (después) | 2,0 MB |

El índice síncrono de la fachada de Obsidian suma ~1,6 a 2 MB aparte (estimado). Queda pendiente
medir el heap real del webview con la app abierta.

## Front: costo de las dos listas al abrir la bóveda

Bóveda real (6 045 elementos). JSON que envía el backend y tiempo de `JSON.parse` en
JavaScriptCore (Bun), mediana de 20:

| Lista | Tamaño | Serializar (Rust) | Parsear (JS) |
|---|---|---|---|
| Explorador (`activate_vault`) | 2,30 MB | 1,5 ms | 2,7 ms |
| Fachada de Obsidian (`get_vault_files`) | 2,26 MB | 2,2 ms | 3,5 ms |

**Medido en la app real** (WebDriver, `e2e/perf-ipc.test.ts`, bóveda sintética de 6 000 notas con
rutas largas; mediana de 7 llamadas desde el webview):

| Comando | JSON | Tiempo total por IPC |
|---|---|---|
| `get_vault_notes` (explorador) | 1 387 KB | 103 ms |
| `get_vault_files` (fachada de Obsidian) | 2 936 KB | 98 ms |

Casi todo el costo es el transporte del IPC de Tauri (serializar y parsear suman ~5 ms). Abrir la
bóveda gasta unos 200 ms en las dos listas y la mitad es la duplicada: **unificarlas (o no pedir
la segunda si no hay plugins habilitados) sí vale la pena.**

## Front: llamadas IPC al abrir una bóveda nueva (6 000 notas)

Medido en la app real con `e2e/perf-open.test.ts` (en modo dev, `invokeTauri` anota cada llamada en
`window.__synapseIpcLog`):

| | Llamadas | IPC acumulado |
|---|---|---|
| Antes | 7 848 (3 944 `read_note_content` + 3 946 `get_note_meta`) | 7 208 ms |
| Después | 45 | 1 976 ms |

La avalancha venía del vigilante de la fachada de Obsidian: al terminar el escaneo inicial, el
backend avisa que cambiaron todos los archivos y la fachada pedía metadatos y releía cada nota.
Ahora, con más de 20 cambios a la vez, refresca el índice con una consulta en bloque y solo
recalcula la caché de metadatos ya calculada. Además, la fachada construye su índice con la lista
del explorador (una sola lista por IPC al abrir).

## App de producción con la bóveda real (~6 000 notas)

Build release (`VITE_SYNAPSE_IPC_LOG=1 bun run tauri build --no-bundle`, que deja activo el registro
IPC) medido con `E2E_APP=src-tauri/target/release/synapsers SYNAPSE_REAL_VAULT=<bóveda> bun test
./e2e/real-vault.test.ts`, con la app instalada cerrada. Dos corridas:

| | Corrida 1 | Corrida 2 |
|---|---|---|
| Explorador visible (incluye arrancar la app) | 1,7 s | 1,7 s |
| Llamadas IPC al abrir / acumulado | 91 / 12 488 ms | 92 / 12 047 ms |
| `get_vault_directory_children` | ×19, 3 261 ms | ×18, 3 542 ms |
| `resolve_asset_file_path` | ×32, 2 164 ms | ×32, 2 076 ms |
| `read_note_content` | ×9, 1 822 ms | ×9, 1 931 ms |
| `render_markdown_wikilinks` | ×3, 1 122 ms | ×3, 1 142 ms |
| `get_vault_git_status` en reposo | 21 ms | 22 ms |
| Memoria: backend / webview / red | 807 / 583 / 106 MB | 748 / 485 / 108 MB |
| Quick Open (con ida y vuelta de WebDriver) | 244–412 ms | 276–440 ms |

En reposo los comandos responden en 0–2 ms; durante la apertura, cientos de ms. El costo está en la
congestión de los primeros segundos, no en cada comando.

## Después de los arreglos de arranque y memoria (copia de la bóveda real, build release)

| | Antes | Después |
|---|---|---|
| Bloqueo del hilo de JS al arrancar (5 pestañas) | ~1 250 ms | ~800 ms |
| Llamadas IPC al abrir / acumulado | 92 / 12 s | 71 / 5,1 s |
| Backend (RSS) | 748–807 MB | 397 MB |
| Total de la app (RSS) | 1 341–1 496 MB | 932 MB |
| Quick Open (dentro de la página) | — | 90–108 ms |
