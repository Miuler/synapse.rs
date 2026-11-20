# P10 — Historial, retención y recolección de basura

**Tareas OpenSpec:** 4.2, 4.3, 4.4. **Depende de:** P09.

## Objetivo

1. **`ListVersions` y `GetVersion`** (4.2): validan `vault_id`, ruta y propiedad de la bóveda (`NOT_FOUND` si es ajena
   o no existe); `ListVersions` de la más reciente a la más antigua (`created_at DESC, version_id DESC`), paginado
   (`page_size` 0 → 100, máximo 1000; `page_token` opaco, p. ej. el último `version_id` en base64). Una ruta borrada
   sigue teniendo historial. `GetVersion` de otra bóveda o inexistente → `NOT_FOUND`. El contenido se descarga con
   `DownloadArtifact` (el blob sigue existiendo mientras la versión esté retenida).
2. **Retención** (4.3): caso de uso `ApplyRetentionUseCase` por tenant con su `RetentionPolicy`:
   - `versions:N`: por ruta, conserva las N más recientes **contando la vigente**.
   - `days:N`: conserva las de los últimos N días.
   - `unlimited`: no borra nada.
   - En todas: la versión vigente de cada ruta del árbol vivo se conserva siempre. Ruta borrada: sin versión vigente.
   - Borra filas de `file_versions` en lotes (no una transacción gigante por tenant).
3. **GC** (4.4): `CollectGarbageUseCase`, tras la retención:
   - Referenciado = aparece en `tree_nodes`, en `file_versions` o como `chunk_hash` de un archivo referenciado.
   - Candidatos: no referenciados **y** `last_seen_at` anterior a `ahora - gc_grace_period`.
   - Por candidato: en una transacción vuelve a comprobar que sigue sin referencias y borra la fila de `blobs` (y sus
     `file_chunks` si es un archivo en bloques); tras el `COMMIT` borra el objeto físico. Si el borrado físico falla,
     regístralo y sigue (quedará un huérfano físico que un barrido posterior de objetos sin fila puede limpiar:
     implementa ese barrido simple por prefijo de tenant).
   - Un bloque compartido por otro archivo vivo no se borra.
4. **Tarea periódica**: en `run`, un `tokio::spawn` con `tokio::time::interval(gc_interval)` que ejecuta retención y
   GC para todos los tenants; nunca dos ejecuciones solapadas; se detiene con el apagado ordenado. El reloj se inyecta
   (trait `Clock` en `application`) para poder probar periodos de gracia y días sin `sleep`.

## Pruebas exigidas (escenarios de `sync-versioning` y del `design.md`)

- `historial::lista_de_la_mas_reciente_a_la_mas_antigua`, `historial::paginacion`,
  `historial::contenido_de_version_antigua_identico` (también de un archivo en bloques),
  `historial::boveda_ajena_not_found`.
- `retencion::plan_de_5_versiones_con_7_conserva_5`.
- `retencion::version_vigente_antigua_se_conserva_con_30_dias` (vigente de hace 90 días).
- `retencion::ilimitada_no_borra`.
- `retencion::ruta_borrada_queda_sujeta_a_la_politica`.
- `gc::contenido_sin_referencias_se_borra_pasada_la_gracia`.
- `gc::blob_compartido_no_se_borra`.
- `gc::subida_sin_confirmar_no_se_borra_antes_de_la_gracia`.
- `gc::find_missing_reciente_protege_el_blob` (`last_seen_at`).
- `gc::bloques_de_archivo_vivo_no_se_borran` y `gc::bloques_de_archivo_recolectado_si`.
- `gc::delta_tras_gc_no_apunta_a_blob_borrado` (el delta que llega después del GC recibe `MISSING_CONTENT`).
- `gc::tarea_periodica_no_se_solapa`.

## Hecho cuando

Verificación en verde, 4.2–4.4 marcadas, CHANGELOG.
