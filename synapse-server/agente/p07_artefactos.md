# P07 — Estado y transferencia de artefactos

**Tareas OpenSpec:** 3.2, 3.3. **Depende de:** P06.

## Objetivo

Casos de uso en `application/use_case/` y su traducción en `SyncController`:

1. **`CheckState`** (3.2): busca la bóveda por `vault_id`; inexistente **o** de otro tenant → `NOT_FOUND` con el mismo
   mensaje (no filtres cuál de los dos es); si existe, `in_sync` y `current_root_hash`. Valida que `vault_id` es un
   UUID y `root_hash` un hash bien formado (`INVALID_ARGUMENT` si no).
2. **`FindMissing`**: valida cada hash (`INVALID_ARGUMENT` con el índice del primero inválido), límite de 10 000
   (`RESOURCE_EXHAUSTED`), devuelve los que **ese tenant** no tiene, en el orden de la petición, y actualiza
   `last_seen_at` de los que sí tiene (una sola sentencia, no una por hash).
3. **`UploadArtifact`** (streaming de entrada):
   - El primer mensaje debe ser `header` (si no, `INVALID_ARGUMENT`); `size_bytes` por encima de `max_artifact_bytes` →
     `RESOURCE_EXHAUSTED` antes de leer datos.
   - Escribe con `ContentRepository::begin_write` mientras llegan los trozos; si los bytes recibidos superan el tamaño
     declarado, aborta con `INVALID_ARGUMENT`; si el stream termina o se corta, `abort()` (nada queda).
   - Al terminar: hash o tamaño distinto → `INVALID_ARGUMENT` con `BadRequest` en `content_hash` y nada guardado.
   - Inserta la fila de `blobs` con `ON CONFLICT DO NOTHING` y `last_seen_at = ahora`; `already_present` indica si ya
     existía. Si ya existía, igualmente consume y verifica el stream (más simple y seguro); no reescribe el objeto.
4. **`DownloadArtifact`** (streaming de salida): hash que el tenant no tiene → `NOT_FOUND`; primer mensaje `header` con
   hash y tamaño, luego trozos de 256 KiB. Los blobs con `chunked = true` aún no existen (P08): deja el punto de
   extensión y una prueba pendiente para P08 **no** marcada `#[ignore]` (simplemente no la escribas todavía).

## Pruebas exigidas (escenarios de «Comprobación del estado» y «Transferencia de artefactos…»)

- `estado::cliente_al_dia`, `estado::cliente_desactualizado_recibe_raiz_vigente`, `estado::boveda_desconocida_not_found`,
  `estado::boveda_de_otro_tenant_not_found` (crea la bóveda insertando con el repositorio en la prueba: aún no hay
  `PushDelta`), `estado::vault_id_invalido`.
- `artefactos::hash_que_no_coincide_invalid_argument_y_no_guarda` (comprueba que no hay fila ni objeto).
- `artefactos::subida_repetida_exitosa_y_una_sola_copia` (cuenta objetos en el directorio de blobs).
- `artefactos::mismo_contenido_en_dos_rutas_una_copia` (aquí: mismo contenido subido dos veces con distintos nombres
  lógicos; la parte de rutas se completa en P09).
- `artefactos::descarga_devuelve_bytes_originales` (10 MB aleatorios, varios trozos).
- `artefactos::artefacto_vacio` (0 bytes: hash de la entrada vacía).
- `artefactos::otro_tenant_no_descarga_ni_ve_el_contenido` (`DownloadArtifact` → `NOT_FOUND`; `FindMissing` lo da
  como faltante).
- `artefactos::sin_cabecera_invalid_argument`, `artefactos::mas_bytes_que_los_declarados`,
  `artefactos::por_encima_del_limite_resource_exhausted`, `artefactos::stream_cortado_no_deja_nada`.
- `artefactos::find_missing_actualiza_last_seen`.

## Hecho cuando

Verificación en verde, 3.2 y 3.3 marcadas, CHANGELOG.
