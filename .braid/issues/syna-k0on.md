---
schema_version: 9
id: syna-k0on
title: 'Temas: colores de canvas (grafo, láser, diff de píxeles) desde variables'
priority: P2
status: open
deps: []
tags:
- temas
owner: null
created_at: 2026-11-11T22:26:29.084605441Z
acceptance:
- El grafo sigue el tema claro/oscuro y los temas de usuario
---

El grafo (canvas/WebGL), el láser de DiagramSplitViewer y el magenta del diff de píxeles tienen colores fijos marcados con /* fijo: */. Leer variables (--graph-*, --laser, --diff-pixel) con getComputedStyle y redibujar al cambiar tema.