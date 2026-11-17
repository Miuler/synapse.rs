---
schema_version: 9
id: syna-1c25
title: 'Arreglar e2e que fallan: rename-occurrences, document-words, content-zoom y compare-files-git (intermitente)'
priority: P2
status: open
deps: []
tags:
- e2e
- pruebas
owner: null
created_at: 2026-11-17T16:57:10.584955138Z
acceptance:
- Las cuatro pasan de forma estable al correr la suite completa
---

Fallan también sobre main (comprobado el 2026-11-16).
- rename-occurrences (0/2): Shift+F6 no abre el renombrado (waitUntil 5000 ms). Sospecha: CodeMirror duplicado otra vez (@codemirror/state en overrides) o WebKitWebDriver no envía bien Shift+F6.
- document-words (1/2): queda «base_datosserv» en vez de «base_datosservidor_web»; la sugerencia se acepta antes de que aparezca la lista de autocompletado (tiempos).
- content-zoom (1/2), «funciona en el editor de código»: el tamaño se queda en 13; Ctrl+= no llega al editor (CodeMirror/Vim lo capturan, o el ajuste GDK_DPI_SCALE del harness).
- compare-files-git (intermitente, 1/4 en main): al abrir el menú, selectedPaths tiene 2 pero filesToCompare queda null (el árbol se recarga tras abrir AGENTS.md); con una espera previa pasa.