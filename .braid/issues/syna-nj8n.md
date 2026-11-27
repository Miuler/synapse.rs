---
schema_version: 9
id: syna-nj8n
title: 'Tableros: editar todos los campos desde las vistas, con aviso en el histórico'
priority: P2
status: open
deps:
- syna-kxih
tags:
- tableros
owner: null
created_at: 2026-11-27T15:23:34.07787452Z
acceptance:
- Título, descripción, tipo, asignado, estado, planificación y comentarios se editan y se envían al proveedor cuando lo admite
- Editar el histórico pide confirmación, solo cambia la copia local y no se pisa al traer
---

Decidido por el usuario: todo se actualiza desde la interfaz (Kanban, Gantt, tabla y detalle); solo al cambiar el histórico se advierte. Tareas 3.7 y 3.8 de openspec/changes/add-issue-providers. Hoy el detalle solo añade comentarios.

Avance 2026-11-27: el detalle de la tarea edita todos los campos, los comentarios y el histórico con aviso (tarea 3.7). Falta enviar al proveedor la descripción, el tipo, el asignado y los comentarios editados (3.8), y probarlo en un tablero real.

Avance 2026-11-27: «Enviar» manda tipo, asignado, descripción y comentarios editados (tarea 3.8), probado solo contra un servidor falso. Falta probarlo contra un work item de prueba real, con permiso del usuario.
