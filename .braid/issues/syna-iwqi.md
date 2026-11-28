---
schema_version: 9
id: syna-iwqi
title: 'Tableros: la vista de tabla se vuelve lenta con cientos de tareas'
priority: P2
status: done
deps: []
tags:
- tableros
owner: null
created_at: 2026-11-27T14:57:22.816540908Z
completed_at: 2026-11-28T00:23:06.802962925Z
acceptance:
- Con 1000 tareas se pasa a Tabla en menos de 1 s
---

Medido con tableros sintéticos en el build de desarrollo, X11 virtual: pasar a Tabla tarda 0,2 s con 100 tareas, 2,8 s con 500 y entre 8 y 10 s con 1000. Causa probable (sin comprobar): cada fila crea el selector «Depende» con todas las demás tareas como opciones, así que los nodos crecen con el cuadrado.

Corregido 2026-11-27: la lista «Depende» se crea al usar el selector y las filas entran por tandas de 100. Mismo equipo, build de desarrollo, 1000 tareas: antes 7,2 s y 1 001 000 opciones; ahora las primeras filas en 0,35-0,4 s, las 1000 en 1,65 s y 2999 opciones.
