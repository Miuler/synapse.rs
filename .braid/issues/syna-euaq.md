---
schema_version: 9
id: syna-euaq
title: 'Tableros: pedir el filtro al crear un tablero'
priority: P2
status: done
deps:
- syna-2pmu
tags:
- tableros
- azure
owner: null
created_at: 2026-11-27T15:14:49.690270383Z
completed_at: 2026-11-27T17:19:07.780674142Z
acceptance:
- Al crear un tablero con origen se elige el filtro antes de traer, y se puede cambiar sin borrar tareas
---

Pedido por el usuario para segmentar el trabajo entre varios tableros. Tarea 2.4 de openspec/changes/add-issue-providers.

Hecho (2026-11-27, tareas 2.4 a 2.7 de add-issue-providers): se pide el origen al crear el tablero, filtro por criterios o consulta, marca de lo que queda fuera y confirmación al desvincular. Falta que el usuario lo pruebe en su aplicación.
