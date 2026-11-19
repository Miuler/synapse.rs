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
   Después: compare-files sin pausas (espera la selección real del árbol y el texto del diff); 2/2 pasan.
   Luego: explorer-pin sin pausas (espera que el explorador flote o se fije) y reading-search con 1 (Ctrl+F se reintenta hasta que abre la búsqueda); 3/3 pasan.
   Luego: diagram-pan-zoom (espera que el SVG se quede quieto y que el zoom/arrastre se apliquen), compare-diagrams y compare-files-git (esperan la selección real y el texto del diff) sin pausas; 8/8 pasan.
   Luego: compare-images, explorer-keyboard-focus, editor-tab, document-words y drawio (2 de 4: esperan el visor montado, el zoom aplicado, el PNG reescrito en disco y la recarga) sin pausas fijas; code-edit-mode deja la suya comentada (comprobación negativa). Todas pasan.
3. Punto 4 (parte): `bun run test:e2e:quick [nombre…]` (e2e/quick.ts): smoke + e2e cambiadas sin commitear o en el último commit + las que contengan los nombres dados; un archivo por proceso, en serie.

4. Suite completa (2026-11-19), un archivo por proceso y sin real-vault, perf-open, docs-shots ni startup-profile: 53/53 archivos pasan, sin caídas en cadena, 2036 s (~34 min). No hay medida «antes» comparable: la corrida anterior se caía en cadena.

5. Punto 3 replanteado: medido, cada archivo tardaba ~34 s, de los que 30 s eran una espera agotada: waitForPort comprobaba Vite en 127.0.0.1 y Vite solo escucha en ::1. Arreglado (espera en localhost) y Vite se arranca una vez por proceso y se reutiliza entre archivos; un vigilante (sh) cierra su grupo al terminar el proceso. Arranque por archivo: ~2 s. Suite completa en un proceso: 160/160 en 323 s (antes 2036 s, ×6). test:e2e:quick también corre en un proceso. Compartir la app entre archivos ya no compensa: ahorraría ~2 s por archivo a cambio de estado compartido entre pruebas.

PENDIENTE:
- Pausas que quedan, todas justificadas: explorer-system-clipboard (el menú lee el portapapeles al abrirse), drawio (2, comprobaciones negativas), code-edit-mode, 1 en acp-agent (comprobación negativa del panel cerrado), editor-tab, vim-redo (negativa), document-words, tab-cursor (justificada), reading-search (1 justificada: mide los 300 ms tras el resultado). perf-open y real-vault se dejan.
- Punto 4: mover a unitarias la lógica simple.
- Verificar el corte rápido con una sesión que se cae de verdad (no probado).
- Reglas: CHANGELOG en el mismo commit, commit solo con aprobación, sin push.