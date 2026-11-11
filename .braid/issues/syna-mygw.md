---
schema_version: 9
id: syna-mygw
title: 'Temas: cargar fragmentos CSS y temas por bóveda'
priority: P2
status: done
deps: []
tags:
- temas
owner: null
created_at: 2026-11-11T21:45:09.200913378Z
completed_at: 2026-11-11T21:53:54.854764668Z
acceptance:
- Un fragmento activado cambia el aspecto sin reiniciar
- El tema elegido se guarda por bóveda
- e2e que activa un fragmento y un tema
---

Leer .synapse/snippets/*.css y .synapse/themes/*.css de la bóveda abierta e inyectarlos como <style>. En Configuración: interruptor por fragmento y selector de tema (con 'Ninguno'). Recargar al cambiar los archivos. Un tema puede definir variables para claro y oscuro con [data-theme].