---
schema_version: 9
id: syna-mxu9
title: 'Tableros: referenciar desde cualquier Markdown un tablero o una tarea'
priority: P2
status: open
deps:
- syna-zpvp
tags:
- tableros
owner: null
created_at: 2026-11-27T14:03:30.178753795Z
acceptance:
- Una nota muestra el estado de una tarea y de un tablero filtrado, en lectura y en vista en vivo
---

Grupo 4 de add-issue-providers: [[Equipo.board#ado-123]] y bloques board con filtros y vistas, pintados desde el archivo, sin red salvo al pulsar «Actualizar».

Avance 2026-11-27: leer el bloque `board`, reconocer el enlace a una tarea, filtrar, ordenar, agrupar y resumir están hechos como funciones puras (tarea 4.1), sin conectar a ninguna vista. Solo resuelven tableros de carpeta.

Avance 2026-11-27: en modo lectura se pintan el bloque `board` y el enlace a una tarea (tarea 4.2). Faltan la vista en vivo (4.3), «Actualizar», abrir el tablero en la tarea y documentar la sintaxis (4.4).

Avance 2026-11-27: la vista en vivo pinta también los bloques y los enlaces (tarea 4.3). Falta «Actualizar», abrir el tablero en la tarea y documentar la sintaxis (4.4).
