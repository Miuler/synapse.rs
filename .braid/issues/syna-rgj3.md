---
schema_version: 9
id: syna-rgj3
title: 'Tableros: traducir los textos fijos en español del editor'
priority: P3
status: done
deps: []
tags:
- tableros
- i18n
owner: null
created_at: 2026-11-27T14:03:30.206148697Z
completed_at: 2026-11-28T04:44:43.243007433Z
acceptance:
- Con la aplicación en inglés el editor de tableros no muestra textos en español
---

Cabeceras, botones y avisos del editor de tableros escritos directamente en español. Pasarlos a t() con base en inglés.

Avance 2026-11-27: los textos del editor, del diálogo de tarea y del marco de los diálogos redimensionables salen en el idioma de la interfaz, comprobado con la app en inglés (e2e board-english). Falta: los detalles de error que escribe Rust siguen en español dentro del mensaje traducido (nombre repetido, vacío, falta la revisión, cambio de tipo no admitido, sin llavero, 1Password), y los bloques de tablero ya pintados en una nota no cambian de idioma hasta volver a abrirla.

Avance 2026-11-27: los motivos fijos que Rust pone en los errores de los tableros son frases en inglés que la interfaz traduce (`shared/lib/i18n/reason.ts`); lo que escriben el sistema, el servidor u `op` sale como está. El motivo de «sin llavero» solo se compila en sistemas no publicados y no está probado.

Avance 2026-11-27: los motivos de rechazo de las operaciones con archivos de la bóveda (archivo de un tablero, nombre en uso, ruta, elemento que no existe) se traducen en un solo punto, `invokeTauri`, y también los avisos de error del explorador. Quien use el MCP recibe esas frases en inglés. Siguen en español: los menús y avisos de éxito del explorador y otros errores de Rust fuera de tableros y archivos (git, plugins, búsqueda), y los bloques de tablero ya pintados en una nota no cambian de idioma hasta volver a abrirla.

Avance 2026-11-27: la lista completa de lo que queda en español fuera de este editor está en syna-8jeu. De los tableros solo falta que los bloques ya pintados en una nota cambien de idioma sin volver a abrirla.
