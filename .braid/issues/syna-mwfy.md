---
schema_version: 9
id: syna-mwfy
title: 'Tableros: editar en la interfaz qué estado remoto corresponde a cada columna'
priority: P3
status: open
deps: []
tags:
- tableros
- azure
owner: null
created_at: 2026-11-27T14:03:30.189863862Z
acceptance:
- La configuración del tablero permite asignar los estados de cada columna
---

Hoy states de azure_devops solo se cambia editando el frontmatter.

En un uso real (2026-11-27) los estados «Blocked», «Resolving» y «Escalated» no corresponden a ninguna columna por defecto, y esas tarjetas caen en la primera. La comparación no distingue mayúsculas.

Resuelto en parte (tarea 2.4 de add-issue-providers): sin configurar nada, cada estado va a la columna de la categoría que declara el proveedor. Falta poder cambiarlo desde la interfaz.
