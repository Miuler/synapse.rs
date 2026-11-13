---
schema_version: 9
id: syna-jxe9
title: Reorganizar el repo en synapsers-desktop, synapsers-app y synapse-server
priority: P2
status: open
deps: []
tags:
- refactor
- repo
owner: null
created_at: 2026-11-13T06:05:06.130445249Z
acceptance:
- Los cambios pendientes del historial de archivo están commiteados antes de mover nada
- Todo el código actual vive en synapsers-desktop/ y la app compila y arranca (bun tauri dev / build)
- .github/workflows/release.yml y ci/release.py apuntan a las rutas nuevas (caché de src-tauri, copia de binarios, package.json y tauri.conf.json)
- Las pruebas e2e y de unidad pasan desde la nueva ubicación
- CLAUDE.md, AGENTS.md y ARCHITECTURE.md describen la nueva estructura
- Decidido qué es synapsers-app y creada su carpeta (o descartada)
---

Mover todo lo actual (src/, src-tauri/, e2e/, package.json, bunfig.toml, vite.config.ts, etc.) a synapsers-desktop/. Crear synapsers-app/ (pendiente definir qué es: cliente ligero móvil, crate compartido u otra cosa) y dejar sitio para synapse-server/ (lo crea el cambio OpenSpec add-sync-server). openspec/ se queda en la raíz. Es un cambio mecánico sin comportamiento nuevo, por eso va aquí y no en OpenSpec. Antes de mover, commitear los cambios pendientes del historial de archivo. Evaluar un Cargo.toml de workspace en la raíz; si se adopta, actualizar rutas de caché y de binarios del release.