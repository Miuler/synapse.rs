---
schema_version: 9
id: syna-2pmu
title: 'Tableros: capa de proveedores de issues y conexiones (Azure DevOps)'
priority: P2
status: done
deps: []
tags:
- tableros
- azure
owner: null
created_at: 2026-11-27T14:03:30.151376846Z
completed_at: 2026-11-27T17:19:07.769913064Z
acceptance:
- Tareas 1.1 a 2.4 de la propuesta marcadas y probadas
---

Grupos 1 y 2 de openspec/changes/add-issue-providers/tasks.md: puerto IssueProvider, adaptador de Azure DevOps, varias conexiones configurables (también del mismo proveedor) con su credencial, y cada tablero vinculado a la suya. No implementado.

Avance 2026-11-27: grupo 1 hecho (modelo común, puerto, adaptador de Azure DevOps, órdenes `issues_*` y tablero sobre el modelo común). Del grupo 2 están las tareas 2.1 y 2.2 (conexiones y envío de cada credencial solo a su servidor); también la 2.3 (pantalla de conexiones con «Probar»); falta la 2.4 (conexión y filtro del tablero).
