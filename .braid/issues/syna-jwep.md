---
schema_version: 9
id: syna-jwep
title: 'Temas: variables para títulos por nivel, fuentes e íconos de carpeta'
priority: P2
status: done
deps: []
tags:
- temas
owner: null
created_at: 2026-11-11T21:45:09.224248672Z
completed_at: 2026-11-11T21:53:54.858850476Z
acceptance:
- Un mismo CSS cambia los títulos en lectura y edición
- Las fuentes de texto e interfaz se pueden cambiar
- El ícono de carpeta se puede reemplazar por un SVG
---

Variables --h1..--h6 (fuente, tamaño, peso, color) aplicadas igual en lectura (h1–h6) y edición (CodeMirror). Variables --font-text y --font-ui. Variables --icon-folder y --icon-folder-open (URL de SVG) que reemplazan el ícono de Lucide cuando están definidas.