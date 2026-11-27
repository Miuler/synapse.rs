---
schema_version: 9
id: syna-j2ef
title: 'Tableros: resolver en la interfaz los conflictos de sincronización'
priority: P3
status: open
deps: []
tags:
- tableros
- azure
owner: null
created_at: 2026-11-27T14:03:30.195690468Z
acceptance:
- Para cada conflicto se ven ambos valores y se elige cuál queda
---

Al cambiar en los dos lados se conserva lo local y solo se avisa; para sobrescribir hay que marcarlo al enviar. No hay forma gráfica de comparar y quedarse con lo remoto.