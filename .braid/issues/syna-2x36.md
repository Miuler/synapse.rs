---
schema_version: 9
id: syna-2x36
title: 'Tableros: comprobar la sincronización contra un Azure DevOps real'
priority: P2
status: open
deps: []
tags:
- tableros
- azure
owner: null
created_at: 2026-11-27T14:03:30.211628409Z
acceptance:
- «Traer» y «Enviar» comprobados con una cuenta real
---

Solo se probó contra un servidor falso; tampoco se probó el token desde 1Password real. Requiere al usuario: primero «Traer» (solo lectura) y después una escritura en un item de prueba.

## Comprobado con una cuenta real, solo lectura (2026-11-27)

- `connectionData` acepta el token y trae `providerDisplayName` (no `customDisplayName`), sin `api-version`.
- Los estados de cada tipo llegan con `category`: Proposed, InProgress, Completed y también Removed.
- Las rutas de área e iteración armadas con los nombres desde la raíz funcionan en WIQL; el campo `path` de la respuesta incluye un segmento `Area` / `Iteration` que no se usa.
- Las etiquetas llegan como `{count, value}`; `CONTAINS` funciona sobre `System.Tags` y la comilla doblada se acepta.
- `System.AssignedTo` llega con un identificador de correo y sirve para filtrar.
- Los filtros por asignado, tipo, área, iteración y etiqueta devuelven resultados. La iteración en curso avisa de que no hay ninguna cuando todas han terminado.

Sigue sin comprobar: «Enviar» (escritura), el token desde la propia aplicación y un token con solo permisos de work items.

