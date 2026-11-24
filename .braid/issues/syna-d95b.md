---
schema_version: 9
id: syna-d95b
title: Corregir 3 fallos en suite E2E (screenshots de Office y flake en document-words)
priority: P2
status: done
deps: []
tags:
- e2e
- test
owner: null
created_at: 2026-11-24T02:58:00.291240404Z
completed_at: 2026-11-24T07:35:06.51129148Z
---

Al ejecutar la suite completa de E2E (bun run test:e2e:embedded:run) fallaron 3 pruebas no relacionadas con los cambios recientes:

1. e2e/spreadsheet-viewer.test.ts (zoom: Ctrl++ agranda toda la rejilla...) y e2e/docx-viewer.test.ts (zoom: Ctrl++ y el control agrandan la pagina entera...): fallan en saveScreenshot porque apuntan a una ruta temporal hardcoded inexistente (/tmp/claude-1000/-home-miuler-src-github-com-Miuler-synapse-rs/47a613e4-73f4-47e2-9153-fa7e86d7bbd0/scratchpad). Eliminar esas capturas de depuracion o usar un directorio temporal creado por el propio test.
2. e2e/document-words.test.ts (sugiere un nombre definido en el documento y lo inserta): fallo intermitente al aceptar la sugerencia de autocompletado en D2 (Expected 'servidor_web -> base_datosservidor_web', Received 'servidor_web -> base_datosserv'). Asegurar espera explicita a que el popup de autocompletado muestre la opcion antes de pulsar Enter.