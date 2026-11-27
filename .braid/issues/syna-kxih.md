---
schema_version: 9
id: syna-kxih
title: 'Tableros: un archivo por tarea en la carpeta protegida del tablero'
priority: P2
status: open
deps:
- syna-nvve
tags:
- tableros
owner: null
created_at: 2026-11-27T15:14:49.684227839Z
acceptance:
- Cada tarea se guarda en su archivo y los tableros anteriores se convierten con confirmación
- Ni el editor, ni el explorador, ni las herramientas MCP modifican la carpeta de un tablero, y no se puede desbloquear
---

Tareas 3.1 a 3.5 de openspec/changes/add-issue-providers: archivo por tarea en Nombre.board/, índice en memoria en Rust, conversión de los tableros con tabla y carpeta protegida (se consulta, el candado no se puede quitar; solo el tablero escribe). No implementado. La edición de la descripción y del resto de campos se hace desde las vistas del tablero.

Avance 2026-11-27: formato (tarea 3.1) e índice con sus órdenes (3.2) hechos y sin conectar a la interfaz. Con 5000 tareas sintéticas, leer el tablero tarda unos 0,1 s. Siguen la protección (3.3), el editor (3.4) y la conversión (3.5).

Avance 2026-11-27: la carpeta del tablero está protegida (tarea 3.3). Siguen el editor sobre el formato nuevo (3.4) y la conversión (3.5). La 3.4 ya está: el editor lee y escribe los tableros «format: 2». Los tableros nuevos se siguen creando como tabla hasta decidir la conversión (3.5).
