---
schema_version: 9
id: syna-yazn
title: 'search_in_note: alternativas (a|b), recuento y varios archivos'
priority: P2
status: done
deps: []
tags:
- busqueda
- mcp
owner: null
created_at: 2026-11-26T16:54:01.653035138Z
completed_at: 2026-11-26T17:08:12.453015094Z
acceptance:
- query "a|b" (o "a | b") busca cualquiera de las alternativas y cada coincidencia indica en "matched" cuáles aparecen en esa línea
- 'Si al partir por | queda alguna parte vacía ("| campo |", "a||b") la consulta entera es literal; "\|" es un | literal; literal: true desactiva el reparto'
- 'Cada archivo devuelve counts por alternativa (lines y occurrences); counts_only: true devuelve solo los recuentos'
- 'relative_paths: [...] busca en varios archivos (tope ~20); un archivo inexistente lleva su propio error con la pista de bóveda activa y no impide los demás'
- Con un solo relative_path la respuesta mantiene la forma actual y solo añade counts
- Descripción MCP con ejemplos; pruebas unitarias e integración; CHANGELOG en los dos idiomas
---

## Por qué

En los logs de los agentes (bóveda `apti-interbank-transfer-diagrams-ds-emisor`, `.synapse/agent-sessions`, revisados el 2026-11-26), unos 5 de los ~25 usos de la terminal dentro de la bóveda eran para mirar dentro de archivos que el agente ya conocía, casi siempre para comprobar lo que acababa de editar:

- `grep -nE "id_ach_operation|Límite BD|no almacena" ach_data_mapping_proposal.md` (varias alternativas en un archivo)
- `grep -c "Límite BD" …` (cuántas veces aparece)
- `rg -n "participant|outbounds-ach" "Sync from Topics.mmd" "Sync from Topics 2.mmd"` (los mismos textos en dos o tres archivos)
- `rg -n "outbound|transfers" "BS Financial Gateway.mmd"`

Hoy `search_in_note` busca una sola subcadena literal (sin distinguir mayúsculas) en un solo archivo y sin recuento: `TIEL|QTI1` se busca tal cual, con la barra.

Diferencia con full_text_search (syna-3e0k): full_text_search es para buscar en toda la bóveda («¿dónde aparece X?»); search_in_note es para mirar dentro de archivos que ya conoces («¿quedó bien lo que edité?»).

## Plan

Archivos: `src-tauri/src/application/use_cases/note/search_in_note_use_case.rs`, `src-tauri/src/application/dto/note/search_in_note_query.rs` y `execute_search_in_note` en `src-tauri/src/infrastructure/entrypoint/mcp/tool_handler.rs`. Sigue leyendo cada archivo con `note_repository.read_note`, como ahora.

1. **Alternativas.** Usar `split_alternatives` (función compartida del dominio definida en syna-3e0k, paso 7; si esta tarea va antes, crearla aquí y que syna-3e0k la reutilice):
   - `a|b` y `a | b` se parten recortando los espacios.
   - Si al partir queda alguna parte vacía (`| campo |`, `a||b`, `|` al principio o al final), toda la consulta es literal. Importante: esta bóveda tiene muchas tablas Markdown y un agente puede buscar `| campo |`.
   - `\|` es siempre un `|` literal; `literal: true` desactiva el reparto.
2. **DTO.** `SearchInNoteQuery`: `relative_paths: Vec<String>` (tope ~20) en lugar de `relative_path`, y `alternatives: Vec<String>` en lugar de `query`. `LineMatch` añade `matched: Vec<String>` (qué alternativas aparecen en esa línea).
3. **Recuento.** Por archivo, `counts`: por cada alternativa, líneas y apariciones (`{"Límite BD": {"lines": 12, "occurrences": 12}}`). Parámetro `counts_only: true` para devolver solo los recuentos, como `grep -c`.
4. **Varios archivos.** Parámetro `relative_paths: [...]` además de `relative_path`. Un archivo que no existe lleva su propio error (con la pista de bóveda activa de `with_location_hint`, adaptada a varias rutas) y los demás se procesan igual.
5. **Compatibilidad.** Con un solo `relative_path`, la respuesta mantiene la forma de hoy (`relative_path`, `total_matches`, `returned_matches`, `matches`) y solo añade `counts`. Con `relative_paths`, `files: [...]` con esa misma forma por archivo. Sigue aceptando `vault` (está en CROSS_VAULT_TOOLS) y `max_matches`.
6. **Descripción MCP.** Explicar `a|b`, la regla de las partes vacías y `\|`, `relative_paths`, `counts` y `counts_only`, con un ejemplo de verificación tras editar.
7. **Pruebas.** Unitarias del caso de uso con un repositorio en memoria: alternativas, `matched`, recuentos, `counts_only`, un archivo inexistente entre varios, `| campo |` literal, `\|`. Integración MCP en `src-tauri/tests/integration`. La caracterización (`tests/integration/characterization/mcp_tools.txt`) de search_in_note con un solo archivo solo debe cambiar por el campo `counts` (`UPDATE_CHARACTERIZATION=1`). CHANGELOG.md y CHANGELOG.es.md.