---
schema_version: 9
id: syna-sh6i
title: Actualizar svelte-check cuando soporte TypeScript 7 sin TypeScript 6
priority: P3
status: open
deps: []
tags:
- tooling
owner: null
created_at: 2026-11-24T06:42:04.427468491Z
acceptance:
- svelte-check actualizado y sin TypeScript 6 en el proyecto
- bun run check arranca y revisa los .svelte
- Errores acumulados corregidos o registrados
---

`bun run check` no arranca: svelte-check 4.7.8 exige TypeScript <= 6 (o 6 y 7 a la vez con --tsgo) y el repo usa solo TypeScript 7. Mientras tanto nadie revisa los tipos dentro de los .svelte (tsc no los entiende). Decidido el 2026-11-24: esperar en lugar de instalar TS 6.

Seguimiento:
- https://github.com/sveltejs/language-tools/pull/3193 (borrador: soporte propio de TS 7, sin TS 6)
- https://github.com/sveltejs/language-tools/issues/3063 (TS 7 rompe svelte-check)
- https://github.com/sveltejs/language-tools/issues/2733

Alternativa si tarda: svelte-check con TS 6 en tools/svelte-check, dejando TS 7 como default.