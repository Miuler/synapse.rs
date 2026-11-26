---
schema_version: 9
id: syna-3e0k
title: 'full_text_search: varias alternativas en una consulta (a|b, OR) y líneas coincidentes como rg -n'
priority: P2
status: open
deps: []
tags:
- busqueda
- mcp
- indice
owner: null
created_at: 2026-11-25T23:36:40.039728174Z
acceptance:
- Por MCP (prefix_last_term = true), `TIEL OR QTI1 OR QTI3` devuelve los documentos con cualquiera de las tres palabras (hoy no devuelve nada)
- '`TIEL|QTI1|QTI3`, `TIEL | QTI1` y `(TIEL|QTI1) emisor` equivalen a OR; `or` en minúsculas también; dentro de comillas el `|` es texto'
- El prefijo de la última palabra no se aplica si la consulta lleva operadores (OR, AND, |, paréntesis, comillas, -, +, campo:); con una consulta simple sigue igual (buscar mientras se escribe en la interfaz)
- 'Fase 1: con `lines: true`, cada resultado incluye sus coincidencias, cada una con una `location` propia del formato (`{line}` en Markdown, código, Mermaid, D2 y PlantUML; `{diagram}` en draw.io y Excalidraw; `{element, type}` en ArchiMate y BPMN; `{sheet}` en Excel), el texto (recortado si es largo) y `context_lines` antes y después, con un tope por documento y en total, y `total_matches_in_doc` aunque se recorte'
- 'Fase 1, modo exacto: con `exact: true`, `query` son uno o varios textos literales separados por `|` (regla de `split_alternatives`) y solo se devuelven los documentos cuyo texto guardado los contiene literalmente (sin distinguir mayúsculas): `outbounds-ach` no devuelve un documento que solo dice «outbound for ach»; `total_matches` cuenta solo los comprobados; el fragmento y `lines` resaltan el texto exacto'
- 'Fase 1, modo exacto: si el texto no tiene ninguna palabra con letras o números (p. ej. `":"`) se devuelve un error claro; si el texto guardado del documento estaba truncado al indexar, la respuesta lo indica'
- 'Fase 1: en PDF y Word, `lines: true` no inventa números de línea: devuelve el fragmento actual y `location: null` hasta la fase 2; en Excel, solo la hoja'
- 'Fase 2: PDF → `{page}` (utilizable con read_pdf_document start_page/end_page); Word → `{section, paragraph}`; Excel → `{sheet, row, column}` con el número de fila y la letra de columna reales'
- 'Fase 2: al cambiar los extractores se sube la versión del índice y las bóvedas existentes se reindexan solas (no basta con el mtime)'
- La descripción MCP de full_text_search explica la sintaxis con ejemplos (`TIEL|QTI1|QTI3`, `"frase exacta"`, `-excluir`, `BCTITP*`, `hs:titulo`, `kind:archimate`) y cuándo usar `lines`
- CHANGELOG.md y CHANGELOG.es.md actualizados
---

## Por qué

En los logs de los agentes (bóveda `apti-interbank-transfer-diagrams-ds-emisor`, `.synapse/agent-sessions`) el patrón es: primero buscan con el MCP y después repiten la búsqueda con `rg -n "a|b|c"` o `grep -iE "a|b|c"`. Usan `|` para hacer una sola búsqueda en vez de varias, y `rg -n` porque devuelve todas las líneas que coinciden, con su número. No se trata de reemplazar grep/rg/fd (el usuario no quiere eso; find_files y grep_vault se quitaron en 3fd79b2): se trata de que la búsqueda del MCP cubra ese mismo uso sobre el índice de Tantivy.

## Qué pasa hoy (probado con un índice en memoria, 2026-11-26)

`SearchNotesFullTextUseCase` llama siempre con `prefix_last_term: true` (`src-tauri/src/application/use_cases/search/search_notes_full_text_use_case.rs`).

| Consulta | prefix=false (interfaz) | prefix=true (MCP) |
|---|---|---|
| `TIEL\|QTI1\|QTI3` | nada | nada |
| `TIEL OR QTI1 OR QTI3` | a, b, c | **nada (bug)** |
| `TIEL or QTI1` | nada | nada |
| `(TIEL OR QTI1) emisor` | a | a |
| `TIEL QTI1` | nada (AND) | nada |
| `BCTITP*` | c | c |
| `"interfaz QTI1"`, `-TIEL QTI1` | bien | bien |

- `|` no es operador de la gramática de Tantivy: se descarta y las palabras quedan juntas (como AND o frase).
- Bug: en `build_query` (`src-tauri/src/infrastructure/repository/tantivy/full_text_index_impl.rs`) el prefijo de la última palabra parte `TIEL OR QTI1 OR QTI3` en «`TIEL OR QTI1 OR`» (Must) + «`QTI3`» como prefijo (Must), y así exige las dos partes.
- `OR` solo funciona en mayúsculas.
- La descripción de full_text_search no dice nada de la sintaxis, así que los agentes no la usan.
- full_text_search devuelve un fragmento por documento, no las líneas. `search_notes` y `search_in_note` buscan una subcadena literal (`TIEL|QTI1` tal cual).

## Plan

1. **Bug del prefijo.** En `build_query`, no aplicar el prefijo de la última palabra si la consulta (ya traducida) contiene `(`, `)`, `"`, ` OR `, ` AND `, un término que empieza por `-` o `+`, o `campo:`. Prueba: con `prefix_last_term: true`, `TIEL OR QTI1 OR QTI3` encuentra los tres documentos; `arquitec` sigue encontrando «arquitectura».
2. **`|` y `or`.** Función pura (p. ej. `normalize_operators` junto a `expand_field_aliases`) que, fuera de comillas y de `/regex/`, cambie `a|b`, `a | b` y `(a|b)` por `a OR b`, y `or`/`and` sueltos en minúsculas por `OR`/`AND`. Se aplica antes del parseo y antes del paso 1, para que el prefijo vea los operadores. Pruebas unitarias de la función y de búsqueda.
3. **Coincidencias con ubicación (`lines`).** Parámetro nuevo de full_text_search: `lines: bool` (por defecto false), `context_lines` (por defecto 0, máx 5), `max_lines_per_doc` (por defecto 20) y un tope total (~200).
   - El cuerpo ya está guardado en el índice (`body` es STORED): no se lee el disco.
   - Para saber qué líneas coinciden, se usan los términos de la consulta (`query_terms`, que ya se recogen en `search`) pasando cada línea por el mismo analizador del campo (`analyze`, tokenizer `es_fold`: identificadores, minúsculas, raíz española, sin tildes). Una línea coincide si alguno de sus tokens está entre los términos. Así `QTI1`, `qti1` y «sesiones/sesión» coinciden igual que en la búsqueda.
   - Para los prefijos (regex de la última palabra, `BCTITP*`) no hay términos: hay que comparar el token con el prefijo.
   - Dónde: en `FullTextHit` (dominio, `src-tauri/src/domain/search/full_text.rs`) un campo opcional `matches: Vec<MatchedLine { location: Option<MatchLocation>, text, before, after }>`, y en `FullTextQuery` el flag. `MatchLocation` es un enum del dominio (`Line`, `Page`, `Section { section, paragraph }`, `Cell { sheet, row, column }`, `Diagram`, `Element { name, type }`) que se serializa como un objeto con solo sus campos. Se rellena en `FullTextIndexImpl::search` solo si se pide (cuesta analizar el cuerpo de cada resultado). `SearchNotesFullTextUseCase` y `execute_full_text_search` (tool_handler.rs) lo pasan y lo serializan.
   - Recortar líneas largas (XML de ArchiMate o draw.io en una línea) a ~300 caracteres alrededor de la coincidencia.
4. **Descripción de la herramienta.** En tool_handler.rs: sintaxis (AND por defecto, `a|b` o `OR`, `-excluir`, `"frase"`, `prefijo*`, `title:`/`hs:`/`path:`/`kind:`) con ejemplos, y «usa `lines: true` para ver dónde está cada coincidencia (línea, página, hoja y fila…), como `rg -n`», con lo que significa `location` en cada formato. Actualizar docs/reference.md.
5. **Pruebas.** Unitarias en full_text_index_impl (índice en memoria: OR con prefijo, `|`, `or`, coincidencias con contexto, topes y `location` por formato) e integración MCP (`tests/integration`). Revisar la caracterización (`UPDATE_CHARACTERIZATION=1`) si cambia alguna respuesta. CHANGELOG en los dos idiomas.
6. **Comprobar la interfaz.** El buscador de la app usa el mismo `build_query`: buscar mientras se escribe debe seguir igual con consultas simples.

## Ubicación de cada coincidencia, por formato

Qué guarda hoy el índice (revisado en `src-tauri/src/infrastructure/search/`, 2026-11-26) y qué devolver:

| Formato | Texto indexado hoy | `location` |
|---|---|---|
| Markdown, código, Mermaid, D2, PlantUML | El archivo, línea a línea | `{line}` real (fase 1) |
| draw.io (`drawio_extractor::mxfile_text`) | Una línea por texto de figura y por nombre de página | `{diagram}`: la última línea de nombre de página antes de la coincidencia (fase 1; si hace falta, marcar las líneas de página al extraer) |
| Excalidraw | Textos de los elementos | `{diagram: null}` o solo el texto (fase 1) |
| ArchiMate (`archimate_extractor`) | Una línea por elemento `Nombre (Tipo)`, `clave: valor`, documentación | `{element, type}`: la última línea `Nombre (Tipo)` antes de la coincidencia (fase 1). Valorar añadir el id al extraer para seguir con inspect_archimate_element |
| BPMN (`bpmn_extractor`) | `Nombre (tag)` por paso | `{element, type}` (fase 1) |
| PDF (`pdf_extractor::extract_pdf_text`) | Todas las páginas en **una sola línea** de palabras: se pierden las páginas | Fase 1: `null` y el fragmento. Fase 2: `{page}` |
| Word (`docx_extractor::extract_docx_text`) | Todos los párrafos unidos con espacios en **una sola línea**; los títulos no se separan (headings vacío) | Fase 1: `null`. Fase 2: `{section, paragraph}`. La página no: Word la calcula al dibujar |
| Excel (`spreadsheet_extractor::spreadsheet_text`) | `Hoja: X` y una línea por fila no vacía, celdas con ` \| `. Se saltan las filas vacías, así que el número de fila se pierde | Fase 1: `{sheet}` (la última línea `Hoja:`) sin fila. Fase 2: `{sheet, row, column}` |

## Fase 1

Los pasos 1 a 6 con las ubicaciones que ya se pueden sacar del texto indexado (tabla de arriba). En PDF y Word, `location: null` y el fragmento de siempre; en Excel, solo la hoja. Además, el paso 7:

7. **Modo exacto (`exact: true`).** Para textos con puntuación que el analizador deforma (`outbounds-ach` → `outbound` + `ach`, `APIM:` → `apim`, `/ach-operations/`, `fees/evaluate`, `024684`). Hoy el agente no tiene garantía de coincidencia literal y se va a `rg`. Va después de `lines` porque reutiliza la selección de líneas.
   - **Dominio** (`domain/search/full_text.rs`): `FullTextQuery.exact: Vec<String>` (vacío = búsqueda normal). Función compartida `split_alternatives(query) -> Vec<String>` (la usa también la tarea de search_in_note): `a|b` y `a | b` se parten recortando espacios; si queda alguna parte vacía (`| campo |`, `a||b`, `|` al principio o al final) toda la consulta es literal; `\|` es siempre un `|` literal.
   - **Índice** (`FullTextIndexImpl::search`), en dos pasos y **sin leer el disco**:
     1. Candidatos: cada texto como consulta de frase (`"outbounds-ach"`, que el analizador convierte en `outbound` seguido de `ach`), combinadas con OR, con un tope alto (~2000).
     2. Comprobación: por cada candidato se lee el cuerpo guardado (`body` es STORED) y el título, y se mira si contienen literalmente alguno de los textos (en minúsculas). Lo que no los contiene se descarta. `total_matches` cuenta solo los comprobados; si se llegó al tope de candidatos, indicar que el total es un mínimo.
     - Fragmento: no usar el `SnippetGenerator` (resalta raíces de palabras); armarlo a mano con ~80 caracteres a cada lado de la primera aparición y el texto exacto resaltado. `matched_terms` = el texto tal como aparece en el documento.
     - `lines`: la función que elige las líneas recibe cómo coincide una línea (por términos analizados o por literales) para reutilizarla aquí.
   - **Consecuencias de usar el texto guardado** (documentarlas en la descripción): funciona igual en PDF, Word, Excel, draw.io y ArchiMate (es el texto extraído); refleja el estado del índice, no del disco (un archivo recién editado se ve como estaba hasta que el indexador lo procesa, normalmente en segundos); si el extractor cortó el texto por `max_extracted_chars` (en ArchiMate el tope es más alto), una aparición posterior al corte no se encuentra: marcar en la respuesta los documentos truncados.
   - **Casos límite**: solo puntuación (`":"`) → error claro, porque no hay palabras para buscar candidatos; palabras de más de 40 caracteres no se indexan (`RemoveLongFilter`), así que no habría candidatos: documentarlo o buscar candidatos solo por las partes cortas del texto.
   - **MCP** (`tool_handler.rs`): parámetro `exact: bool`. Con `exact: true` no se interpretan comillas, `-` ni `OR`; se combina con `folder_path`, `extensions` y `lines`. Ejemplo para la descripción: `{"query": "outbounds-ach|/ach-operations/", "exact": true, "lines": true, "folder_path": "src/sequences"}`, el equivalente de `rg -n "outbounds-ach|/ach-operations/" src/sequences`.
   - **Pruebas** (índice en memoria): un documento con `POST /ach-operations/{id}/outbounds-ach/initiate` y otro con «outbound for ach» → solo el primero; alternativas con `|`; mayúsculas; `| campo |` literal; error con solo puntuación; `lines` con exacto; documento truncado marcado.

## Fase 2: extractores con marcas de posición

1. **PDF**: extraer página a página (lopdf ya lo permite; `read_pdf_document` lo hace en su modo search) y guardar un marcador por página (p. ej. una línea `\f` o `[[page:N]]` que el tokenizer ignore, o mejor un salto de línea por página y una tabla aparte de desplazamientos). Sin el marcador en el texto, decidir dónde guardar los desplazamientos (campo STORED aparte, p. ej. `page_offsets`).
2. **Word**: un párrafo por línea (hoy se unen con espacios) y detectar los títulos (`w:pStyle` Heading1…/Título N, o `w:outlineLvl`) para guardarlos en `headings` y como líneas de sección. Así además se puede filtrar con `hs:` en Word.
3. **Excel**: guardar el número de fila real (calamine da `range.start()` + índice) y la columna de cada celda, p. ej. `F37: C=achOperationId | D=…`, o un formato que conserve fila y columna sin ensuciar la búsqueda.
4. **Versión del índice**: subir la versión del esquema o del extractor para que las bóvedas existentes se reindexen solas (hoy `indexed_versions` compara solo el mtime y no volvería a procesar los archivos sin cambios). Ver cómo `open_or_create` recrea el índice cuando cambia la versión.
5. **Comprobar** que la búsqueda de siempre sigue encontrando lo mismo (los saltos de línea nuevos no deben partir frases en las pruebas existentes) y que el visor de la app (Ctrl+F en PDF, Word y Excel, la búsqueda global) no depende del formato del cuerpo indexado.

## Fuera de esta tarea (más adelante, si hace falta)

- `/regex/` sin campo (Tantivy admite `campo:/re/` con `query_parser.allow_regexes()`, pero se aplica a términos indexados, no a líneas) y el resaltado de los resultados de regex (RegexQuery no informa términos al SnippetGenerator).
- Alternativas, recuento y varios archivos en `search_in_note`: tarea aparte **syna-yazn** (comparte `split_alternatives` con el paso 7).
- Alternativas en `search_notes`.

## Estado

Las pistas de la fase 1 (bóveda activa cuando una ruta no existe; aviso de search_notes cuando no hay Markdown) están en `main` (4371555 y 3fd79b2), con pruebas en `src-tauri/tests/integration/mcp_search_hints_tests.rs`.
