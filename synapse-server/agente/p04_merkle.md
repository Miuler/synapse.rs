# P04 — Regla Merkle y rutas en el dominio

**Tareas OpenSpec:** 2.2. **Depende de:** P01 (independiente de P03).

## Objetivo

Implementar en `domain/tree/` (código puro, sin I/O, sin async) la regla normativa del `design.md` («Árbol Merkle»):

- `ContentHash` (value object, en `domain/content/`): 64 caracteres hex minúscula; constructor que valida; `EMPTY`
  = BLAKE3("") = `af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262` (pruébalo contra `blake3`).
- `VaultPath`: valida la regla de rutas (relativa, `/`, NFC, sin `/` inicial/final, sin segmentos vacíos, `.`, `..`,
  `\0`, `\`; máximo 1024 bytes). Para NFC, comprueba que la ruta **ya está** en NFC y rechaza si no (no normalices en
  silencio: el hash del cliente dependería de ello). Usa `unicode-normalization` (librería pura; añádela a la lista de
  permitidas del test de arquitectura con su comentario). Métodos: `parent()`, `name()`, `ancestors()`,
  `is_descendant_of()`.
- `NodeType` (`File`, `Dir`, `Index`) con su etiqueta ASCII para el cálculo.
- `dir_hash(children)`: BLAKE3 de, por hijo directo ordenado por los bytes UTF-8 del nombre,
  `nombre 0x00 TIPO 0x00 hash_hex 0x0A`.
- `Tree` en memoria (mapa ordenado ruta → nodo) con:
  - `apply(delta) -> Result<AppliedDelta, TreeError>`: aplica upserts (crea directorios intermedios), borrados (un `DIR`
    borra su subárbol; borrar una ruta inexistente es error `PathNotFound`), y recalcula **solo los ancestros de las
    rutas tocadas**; devuelve la nueva raíz y la lista de cambios por ruta (`Create`, `Modify`, `Delete`, `Move`) que
    P09 usará para `file_versions`.
  - Conflictos que son error: una hoja donde ya hay un `DIR` o al revés, un upsert y un borrado de la misma ruta en el
    mismo delta, `moved_from` que no existe o que no está en `deleted_paths`.
  - `root_hash()`; árbol vacío = `ContentHash::EMPTY`.
- `proto/MERKLE.md`: la regla en prosa, el pseudocódigo y **vectores de prueba** (al menos: árbol vacío; un archivo en
  la raíz; dos archivos en orden inverso de inserción; un archivo en `a/b/`; un `INDEX`; un directorio vacío;
  nombres con mayúsculas y con caracteres no ASCII que prueben el orden por bytes). Cada vector con entradas y raíz
  esperada; las raíces se obtienen de la implementación y se fijan en las pruebas.

Este código se usará después dentro de la transacción de `PushDelta` cargando del repositorio solo los nodos de los
directorios afectados; diseña `Tree` para que funcione con un subconjunto (los hermanos de cada ancestro tocado), no
solo con el árbol completo, y documenta en `///` qué nodos necesita cargados.

## Pruebas exigidas

- Unitarias: cada regla de `VaultPath` (válidas e inválidas, incluida una ruta no NFC); cada vector de `MERKLE.md`;
  cada conflicto de `apply`; borrar un `DIR` elimina el subárbol; mover registra `Move`.
- `proptest`: el orden de inserción no cambia la raíz; cambiar un byte de cualquier `content_hash` cambia la raíz;
  aplicar un delta y luego el inverso devuelve la raíz original; recalcular solo ancestros da lo mismo que recalcular
  todo el árbol.

## Fuera de alcance

Persistencia y gRPC.

## Hecho cuando

Verificación en verde, test de arquitectura en verde con `blake3` y `unicode-normalization` como permitidas, 2.2
marcada, CHANGELOG.
