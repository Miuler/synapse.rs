# P08 — Archivos grandes en bloques

**Tareas OpenSpec:** 3.4. **Depende de:** P07.

## Objetivo

1. **`DeclareManifest`** (caso de uso + controller), según «Chunking» del `design.md`:
   - Valida hashes y que la lista no esté vacía; más de `max_artifact_bytes / 256 KiB` bloques → `RESOURCE_EXHAUSTED`.
   - Bloques que el tenant no tiene → `FAILED_PRECONDITION` con `PreconditionFailure` (tipo `MISSING_CONTENT`, un hash
     por violación, todos los faltantes, no solo el primero).
   - Reensambla en streaming (sin cargar el archivo en memoria) calculando BLAKE3 y tamaño; si no coinciden con lo
     declarado → `INVALID_ARGUMENT`.
   - En una transacción: fila de `blobs` (`chunked = 1`, `size_bytes` del total, `stored_size` 0) y filas de
     `file_chunks` en orden. Idempotente: declarar dos veces el mismo manifiesto no duplica nada; declarar el mismo
     `content_hash` con otra lista de bloques también es válido si verifica (conserva la primera).
   - Actualiza `last_seen_at` del archivo y de sus bloques.
2. **Descarga reensamblada**: `DownloadArtifact` de un blob `chunked` concatena en streaming los bloques en orden.
3. El umbral `chunk_threshold_bytes` es una recomendación para el cliente que se documenta en el README (P11); el
   servidor solo impone `max_artifact_bytes` a subidas enteras.
4. Añade `fastcdc` **solo como dependencia de desarrollo** y un helper `support::chunking` que corte como lo hará el
   cliente (mínimo 256 KiB, promedio 1 MiB, máximo 4 MiB).

## Pruebas exigidas (escenarios de «Archivos grandes en bloques…»)

- `bloques::edicion_pequena_en_archivo_de_50_mb_solo_sube_bloques_cambiados`: genera 50 MB pseudoaleatorios con
  semilla fija, córtalos, sube, declara; modifica unos bytes en medio, vuelve a cortar, `FindMissing` devuelve solo
  1–3 bloques; tras subirlos y declarar, el almacenamiento creció en esos bloques (mide bytes en el directorio de
  blobs), no en 50 MB.
- `bloques::manifiesto_con_bloques_faltantes_los_indica_todos`.
- `bloques::manifiesto_que_no_da_el_hash_declarado_invalid_argument` (bloques en otro orden).
- `bloques::tamano_declarado_distinto_invalid_argument`.
- `bloques::descarga_de_archivo_en_bloques_devuelve_bytes_completos`.
- `bloques::declarar_dos_veces_es_idempotente`.
- `proptest` del helper: cortar y concatenar devuelve el original.

Si la prueba de 50 MB tarda más de ~10 s en `cargo test` (perfil de pruebas), dilo y propón cómo acotarla (no la
marques `#[ignore]` por tu cuenta).

## Hecho cuando

Verificación en verde, 3.4 marcada, CHANGELOG.
