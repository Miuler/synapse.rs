---
schema_version: 9
id: syna-gen0
title: 'Búsqueda: abrir el resultado con el buscador del editor cargado con el término'
priority: P3
status: done
deps: []
tags:
- busqueda
- indice
owner: null
created_at: 2026-11-18T09:21:23.343072738Z
completed_at: 2026-11-24T07:51:42.209499684Z
---

Al abrir un resultado de la búsqueda global (`full_text_search`, Quick Open), el archivo se abre con el buscador nativo del editor ya cargado con el término buscado y posicionado en la primera coincidencia; Enter / flechas recorren el resto.

Reemplaza el enfoque anterior (indexar subdocumentos de Tantivy con su línea): no cambia el esquema del índice, no necesita extractores por tipo de archivo y la posición siempre es la actual del archivo.

- Markdown en modo lectura: barra de búsqueda con el término y coincidencias resaltadas.
- Markdown en modo edición y CodeEditor (YAML/JSON/texto): panel de búsqueda de CodeMirror con el término.
- Se aplica una sola vez por búsqueda: si el usuario cierra el buscador no se reabre al re-renderizar.
- Pendiente: visores de diagramas (draw.io, ArchiMate, BPMN) no tienen buscador; ver si aplica algo equivalente (resaltar el elemento).
