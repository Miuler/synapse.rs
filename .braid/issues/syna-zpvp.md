---
schema_version: 9
id: syna-zpvp
title: 'Tableros: referencia, tipo y asignado en tarjetas y tabla'
priority: P2
status: open
deps: []
tags:
- tableros
- azure
owner: null
created_at: 2026-11-27T14:03:30.156776277Z
acceptance:
- Tras sincronizar, toda tarjeta enlazada muestra su referencia (AB#123) y permite abrirla en el proveedor
- El asignado y el tipo de item se ven en el Kanban y la tabla
- El formato actual del tablero se sigue leyendo y las columnas desconocidas se conservan
---

Pedido por el usuario. Hoy las tarjetas traídas muestran ado-123 como id, las creadas en Synapse conservan t1 aunque estén enlazadas, y el tipo de item se lee y se descarta. Asignado y tipo, de solo lectura. Tareas 3.1 y 3.4 de la propuesta add-issue-providers.

En los tableros «format: 2» ya se ven la referencia, el tipo y el asignado según `fields`. Falta abrir la tarea en el proveedor (Azure DevOps todavía no da su enlace) y verlo en un tablero real tras la conversión.
