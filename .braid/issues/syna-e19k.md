---
schema_version: 9
id: syna-e19k
title: 'Tableros: el agente ayuda a configurar el origen con herramientas MCP privadas'
priority: P2
status: open
deps:
- syna-2pmu
tags:
- tableros
- mcp
- agente
owner: null
created_at: 2026-11-27T15:17:32.645292141Z
acceptance:
- El agente integrado prueba y cambia el borrador de la consulta, y solo el usuario guarda
- Sin el token de una sesión las herramientas no se listan ni se ejecutan, y ninguna devuelve credenciales
---

Pedido por el usuario: con la configuración del tablero abierta, pedir al agente que genere la consulta, la pruebe y la modifique. Tareas del grupo 7 y especificación config-assistant de openspec/changes/add-issue-providers. El servidor MCP actual no pide autenticación (límite conocido): las herramientas privadas necesitan un token por sesión.

Avance 2026-11-27: las herramientas `config_` ya se sirven solo a los agentes que abre Synapse, con un token por sesión (tarea 7.1); todavía responden que no hay ninguna configuración abierta. Faltan las tareas 7.2 y 7.3. Límite anotado en tasks.md sobre cómo claude-acp pasa el token a su CLI.
