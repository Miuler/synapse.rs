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
