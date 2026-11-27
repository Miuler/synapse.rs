---
schema_version: 9
id: syna-stcs
title: 'Tableros: traer la descripción desde Azure DevOps'
priority: P2
status: open
deps:
- syna-nvve
tags:
- tableros
- azure
owner: null
created_at: 2026-11-27T14:47:27.491754163Z
acceptance:
- «Traer» trae la descripción y se ve en el detalle de la tarea
---

El usuario preguntó si se traen. No: solo título, estado y tipo (que se descarta). La descripción llega como HTML y hay que sanearla o convertirla. No estaba en la propuesta add-issue-providers. Comentarios e histórico: syna-pwgq.

Avance 2026-11-27: «Traer» guarda la descripción como Markdown saneado en los tableros de carpeta (tarea 3.6), probado contra un servidor falso. Falta comprobarlo contra un Azure DevOps real.
