---
schema_version: 9
id: syna-ggiu
title: 'Tableros: respetar el modo lectura'
priority: P2
status: done
deps: []
tags:
- tableros
owner: null
created_at: 2026-11-27T14:03:30.201005459Z
completed_at: 2026-11-28T00:38:33.565590335Z
acceptance:
- En modo lectura no se puede mover, editar, añadir, borrar ni comentar
---

El editor de tableros permite modificar el tablero aunque la pestaña esté en modo lectura.

Hecho 2026-11-27: en lectura o con el archivo bloqueado el tablero solo se mira. Los tableros se abren en edición; la elección de lectura se recuerda por tablero y bóveda (el índice marca «reading» todo archivo sin elección, así que no sirve para ellos). e2e/board-read-only.test.ts.
