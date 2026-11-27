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

Por verificar de «Probar» (`GET <servidor>/_apis/connectionData`, usado por el cliente oficial de Microsoft pero fuera de la referencia REST): que acepte un token con solo permisos de work items, que el nombre llegue en `customDisplayName` o `providerDisplayName`, y que no exija `api-version`.

Por verificar de los estados: que `workitemtypes/{tipo}/states` responda con `name` y `category` con un token solo de work items, y qué categoría llega para los estados retirados.

Por verificar del filtro: las rutas de área e iteración armadas con los nombres de los nodos desde la raíz; la comilla simple doblada en WIQL; `CONTAINS` sobre `System.Tags`; la forma de la respuesta de etiquetas; y la forma de `System.AssignedTo`.
