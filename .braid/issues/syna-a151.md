---
schema_version: 9
id: syna-a151
title: 'Chat del agente: modelo y esfuerzo desde configOptions'
priority: P2
status: done
deps: []
owner: null
created_at: 2026-11-24T08:39:39.223576283Z
completed_at: 2026-11-24T08:39:39.228146953Z
---

Claude (claude-agent-acp 0.107 y 0.109) ya no manda `models` ni acepta `session/set_model`: anuncia modelo y esfuerzo en `configOptions` y se cambian con `session/set_config_option`. Se leen de ahí y se añade el selector de esfuerzo.