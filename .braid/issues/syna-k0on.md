---
schema_version: 9
id: syna-k0on
title: 'Temas: colores de canvas (grafo, láser, diff de píxeles) desde variables'
priority: P2
status: done
deps: []
tags:
- temas
owner: null
created_at: 2026-11-11T22:26:29.084605441Z
completed_at: 2026-11-19T20:56:50.631670538Z
acceptance:
- El grafo sigue el tema claro/oscuro y los temas de usuario
---

El grafo (canvas/WebGL), el láser de DiagramSplitViewer y el magenta del diff de píxeles tienen colores fijos marcados con /* fijo: */. Leer variables (--graph-*, --laser, --diff-pixel) con getComputedStyle y redibujar al cambiar tema.

HECHO (2026-11-19): src/shared/lib/theme-colors (cssVar, withAlpha, onThemeChange) y variables --graph-* y --laser en app.css (claro y oscuro). El grafo 2D/3D y el trazo del láser las leen y se repintan al cambiar de tema (data-theme o un tema de usuario en <head>). Verificado con capturas del grafo en oscuro y claro. El magenta del diff de píxeles se deja fijo: el backend (Rust) pinta el mismo color en las imágenes de diff de git, y una variable solo en CSS dejaría la leyenda distinta de la imagen.
De paso: el botón Láser tenía el punto estirado (la regla `.zoom-controls span` del «100%» le daba min-width 45px); ahora es redondo y usa --laser (e2e/laser-theme.test.ts).
