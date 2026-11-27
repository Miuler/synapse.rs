---
schema_version: 9
id: syna-iwqi
title: 'Tableros: la vista de tabla se vuelve lenta con cientos de tareas'
priority: P2
status: open
deps: []
tags:
- tableros
owner: null
created_at: 2026-11-27T14:57:22.816540908Z
acceptance:
- Con 1000 tareas se pasa a Tabla en menos de 1 s
---

Medido con tableros sintéticos en el build de desarrollo, X11 virtual: pasar a Tabla tarda 0,2 s con 100 tareas, 2,8 s con 500 y entre 8 y 10 s con 1000. Causa probable (sin comprobar): cada fila crea el selector «Depende» con todas las demás tareas como opciones, así que los nodos crecen con el cuadrado.