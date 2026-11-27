---
schema_version: 9
id: syna-pwgq
title: 'Tableros: traer comentarios e histórico por tarea, con su tipo'
priority: P2
status: open
deps: []
tags:
- tableros
- azure
owner: null
created_at: 2026-11-27T14:03:30.167789943Z
acceptance:
- «Traer» trae comentarios e histórico distinguidos por tipo
- 'Lo traído no se vuelve a enviar y ## Comentarios actual se convierte sin pérdidas'
---

Pedido por el usuario. Hoy solo se envían comentarios. Cada entrada necesita un campo que diga si es comentario o historia, con autor e id remoto (## Actividad, tareas 3.2 y 3.3). Falta verificar los endpoints de Azure DevOps en su documentación.

Avance 2026-11-27: «Traer» guarda comentarios e histórico en los tableros de carpeta (tarea 3.6), probado contra un servidor falso. Falta comprobarlo contra un Azure DevOps real y que lleguen antes los comentarios editados allí.
