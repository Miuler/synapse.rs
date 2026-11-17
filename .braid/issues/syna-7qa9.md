---
schema_version: 9
id: syna-7qa9
title: 'E2E: más rápidas y robustas (sin paralelizar)'
priority: P2
status: open
deps: []
tags:
- e2e
- pruebas
owner: null
created_at: 2026-11-17T19:31:53.256391789Z
acceptance:
- Ninguna e2e usa pause() fijo salvo esperas justificadas y comentadas (ventanas de la app, comprobaciones de que algo NO ocurre, perf-open, real-vault)
- Archivos pequeños con bóvedas parecidas comparten una instancia de la app y reinician el estado entre pruebas
- Existen bun run test:e2e:quick (smoke + áreas tocadas) y test:e2e (completa)
- La lógica simple que solo se probaba por e2e tiene pruebas unitarias en bun test src
- La suite completa corre sin caídas en cadena (ECONNREFUSED) y se mide el tiempo antes/después
- CHANGELOG actualizado
---

Pedido del usuario (desde un fork): mejorar velocidad y robustez de las e2e SIN paralelizar (paralelizar es syna-jqrv, al final y midiendo antes).

Contexto: una corrida completa de bun run test:e2e iba en el archivo 6 de 46 tras 17 min, todo fallando por ECONNREFUSED: el WebDriver se caía y cada prueba esperaba su timeout.

HECHO (commit «test(e2e): corte rápido…»):
1. e2e/harness.ts:
   - guardBrowser(): si la sesión muere (proceso del WebDriver terminó, o error ECONNREFUSED/ECONNRESET/sesión inválida) las pruebas restantes fallan al instante con el motivo.
   - connectionRetryCount: 0 y connectionRetryTimeout 30 s (antes 3 reintentos de hasta 2 min).
   - Causa probable de las caídas en cadena: el archivo siguiente arrancaba con el WebDriver anterior aún en el puerto 4444. Ahora close() y launch() esperan el puerto libre (waitForPortFree). Dos archivos seguidos pasaron 3/3.
   - paneReady(s): espera a que la pestaña visible monte su visor (.diagram-viewer-container, .cm-content o .markdown-body).
2. Pausas fijas → esperas reales, sin pause: content-zoom, smoke, editor-line-keys, editor-cursor, code-folding, diagram-render, search-shortcuts, rename-occurrences. tab-cursor: de 8 a 1 (2,6 s justificada: la app ignora 2,5 s los cambios de disco tras su propio guardado, recentSaveTimestamps en WorkspacePage). acp-agent: el pause tras el permiso se cambió por esperar que no queden botones de permiso.
   Todos los archivos tocados pasan por separado.

PENDIENTE:
- Pausas en: compare-files (3), diagram-pan-zoom (3), compare-files-git (2), compare-diagrams (2), explorer-pin (2), reading-search (2), compare-images, drawio, explorer-system-clipboard, explorer-keyboard-focus, y 1 en acp-agent (comprobación negativa del panel cerrado), editor-tab, vim-redo (negativa), document-words, tab-cursor (justificada). perf-open y real-vault se dejan.
- Punto 3: agrupar archivos pequeños con bóvedas parecidas (pestañas, explorador) reutilizando la app y reiniciando estado entre pruebas.
- Punto 4: scripts test:e2e:quick (smoke + áreas tocadas) y test:e2e (completa); mover a unitarias la lógica simple.
- Correr la suite completa y medir tiempo; verificar el corte rápido con una sesión que se cae de verdad (no probado).
- Reglas: CHANGELOG en el mismo commit, commit solo con aprobación, sin push.