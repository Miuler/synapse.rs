# P09 — PushDelta, PullState y registro de versiones

**Tareas OpenSpec:** 3.5, 3.6, 4.1. **Depende de:** P04, P07 (y P08 para los archivos en bloques).

Es la petición más delicada: transacción, notario Merkle y concurrencia. Antes de escribir código, presenta un plan
corto (orden de comprobaciones, qué se carga de la base, cómo se usa `Tree` de P04 con un subconjunto de nodos, qué
consultas) y espera el visto bueno.

## Objetivo

1. **Repositorios** (`tree_nodes`, `snapshots`, `file_versions`) con los métodos que necesite el caso de uso, dentro de
   la «unidad de trabajo» de P05.
2. **`PushDeltaUseCase`**, todo en **una** transacción SQLite (`BEGIN IMMEDIATE`, para que dos deltas de la misma
   bóveda se serialicen y el segundo vea la raíz nueva), en el orden del `design.md`:
   1. Propiedad: bóveda de otro tenant → `NOT_FOUND`. Bóveda inexistente: solo se acepta si
      `base_root_hash == EMPTY_ROOT` (la crea); si no, `NOT_FOUND`.
   2. `base_root_hash` ≠ vigente → `ABORTED` con `ErrorInfo` (`STALE_BASE`) y metadato `current_root_hash`.
   3. Forma: rutas (`VaultPath`), tipos (`UNSPECIFIED` inválido; `DIR` sin hash ni tamaño; hojas con hash), número de
      nodos ≤ `max_delta_nodes` (`RESOURCE_EXHAUSTED`), conflictos de `Tree::apply` → `INVALID_ARGUMENT` con
      `BadRequest`.
   4. Contenidos: todos los `content_hash` de hojas existen en `blobs` del tenant (dentro de la transacción) → si no,
      `FAILED_PRECONDITION` con `PreconditionFailure` (`MISSING_CONTENT`, todos los faltantes).
   5. Carga los nodos necesarios (hermanos de cada ancestro tocado y subárboles borrados), aplica con `Tree`, compara la
      raíz con `new_root_hash` → si no, `FAILED_PRECONDITION` con `ErrorInfo` (`ROOT_MISMATCH`).
   6. Escribe: `tree_nodes` (upserts, borrados, hashes de `DIR` recalculados), `file_versions` (4.1: una fila por
      cambio de hoja con `CREATE`/`MODIFY`/`DELETE`/`MOVE`, `version_id` UUID v7, `DELETE` con el último hash; los
      `DIR` no generan versiones), `snapshots.root_hash` y `updated_at`; `last_seen_at` de los blobs usados.
   7. `COMMIT`. Cualquier error antes: `ROLLBACK` y nada cambia.
   - Un upsert de una hoja con el mismo hash y tamaño que ya tiene no genera versión (no es un cambio).
3. **`PullStateUseCase`** (3.6): lectura coherente (una transacción de lectura) que emite primero `root_hash` y luego
   los nodos ordenados por ruta, con filtro opcional por `node_types`; en streaming desde la base (no cargues el árbol
   entero en memoria si puedes paginar por ruta). Bóveda inexistente o ajena → `NOT_FOUND`.
4. Registra métricas útiles en `tracing` (nodos aplicados, duración), nunca rutas completas a nivel `info` (pueden ser
   datos personales): rutas solo a `debug`.

## Pruebas exigidas

`sync-ingest`:
- `delta::primera_subida_sobre_raiz_vacia_crea_la_boveda`.
- `delta::delta_valido_aplica_todo_y_cambia_la_raiz` (el cliente de prueba calcula la raíz con el `Tree` del dominio,
  como lo hará el cliente real).
- `delta::artefacto_no_subido_rechaza_todo_e_indica_hashes` (`FAILED_PRECONDITION`, árbol intacto).
- `delta::raiz_inconsistente_failed_precondition_sin_cambios`.
- `delta::base_vieja_aborted_con_raiz_vigente_sin_cambios`.
- `delta::boveda_de_otro_tenant_not_found_sin_cambios`.
- `delta::ruta_invalida_invalid_argument` (`..`, `/` inicial, segmento vacío, no NFC).
- `delta::directorios_implicitos` y `delta::borrado_de_directorio_borra_el_subarbol`.
- `delta::mismo_contenido_en_dos_rutas_un_solo_blob`.
- `delta::deltas_concurrentes_sobre_la_misma_base_uno_gana_y_otro_aborted` (lanza los dos a la vez con `tokio::join!`).
- `estado_completo::reconciliacion_tras_aborted` (el cliente recibe `ABORTED`, hace `PullState`, rehace el delta sobre
  la raíz vigente y lo confirma).
- `estado_completo::filtro_por_tipo`.
- `delta::archivo_en_bloques_referenciado_en_un_delta` (si P08 está hecho).

`sync-versioning` (4.1):
- `versiones::dos_ediciones_dejan_tres_versiones` (y los blobs solo crecen por los contenidos nuevos).
- `versiones::borrado_conserva_el_historial`.
- `versiones::movimiento_registra_move_y_conserva_versiones_previas`.
- `versiones::delta_rechazado_no_deja_versiones`.

## Hecho cuando

Verificación en verde, 3.5, 3.6 y 4.1 marcadas, CHANGELOG.
