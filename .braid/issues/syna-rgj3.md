---
schema_version: 9
id: syna-rgj3
title: 'Tableros: traducir los textos fijos en español del editor'
priority: P3
status: open
deps: []
tags:
- tableros
- i18n
owner: null
created_at: 2026-11-27T14:03:30.206148697Z
acceptance:
- Con la aplicación en inglés el editor de tableros no muestra textos en español
---

Cabeceras, botones y avisos del editor de tableros escritos directamente en español. Pasarlos a t() con base en inglés.

Avance 2026-11-27: los textos del editor, del diálogo de tarea y del marco de los diálogos redimensionables salen en el idioma de la interfaz, comprobado con la app en inglés (e2e board-english). Falta: los detalles de error que escribe Rust siguen en español dentro del mensaje traducido (nombre repetido, vacío, falta la revisión, cambio de tipo no admitido, sin llavero, 1Password), y los bloques de tablero ya pintados en una nota no cambian de idioma hasta volver a abrirla.
