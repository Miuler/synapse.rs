---
schema_version: 9
id: syna-af1y
title: 'Temas: pasar los colores fijos a variables'
priority: P2
status: open
deps: []
tags:
- temas
owner: null
created_at: 2026-11-11T21:45:09.247816664Z
acceptance:
- Ningún color fijo fuera de app.css salvo justificados
- Sin cambios visibles en los temas claro y oscuro actuales
---

Hay ~265 colores escritos a mano en .svelte/.css fuera de var(). Pasarlos a variables del sistema (existentes o nuevas) sin cambiar el aspecto actual en claro ni oscuro. Los componentes en canvas (grafo, diagramas) leen las variables del tema.