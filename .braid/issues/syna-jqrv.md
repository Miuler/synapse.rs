---
schema_version: 9
id: syna-jqrv
title: 'e2e: correr los archivos en paralelo (3-4 procesos), primero medir si conviene'
priority: P3
status: skip
deps: []
tags:
- e2e
- pruebas
owner: null
created_at: 2026-11-17T19:07:47.221464883Z
completed_at: 2026-11-19T19:57:15.506002535Z
acceptance:
- Medido el tiempo total y los fallos intermitentes con 1, 2, 3 y 4 procesos en esta máquina
- Si conviene, test:e2e usa el número de procesos que rindió mejor; si no, queda documentado por qué se descartó
---

Hoy la suite e2e corre archivo por archivo con un solo WebDriver y cada archivo arranca la app desde cero (~15-30 s), unos 20+ min en total. Idea: varios procesos a la vez, cada uno con su puerto de WebDriver, su SYNAPSE_CONFIG_DIR y su vault (ya se aíslan por vault).

OJO: antes de adoptarlo hay que probar que de verdad sea más rápido. Varias instancias de Tauri/WebKit a la vez pueden saturar CPU/RAM/GPU y volver todo más lento o más intermitente (las e2e ya tienen tiempos sensibles). Medir con 1, 2, 3 y 4 procesos el tiempo total y la tasa de fallos intermitentes, y quedarse con el que rinda mejor (o descartarlo). Hacerlo al final, después del corte rápido del harness, las pausas fijas y la agrupación de archivos.

DESCARTADO (2026-11-19): tras syna-7qa9 la suite completa tarda ~5,5 min (160/160) y cada archivo arranca en ~2 s; el tiempo restante es el de las pruebas mismas. Paralelizar añadiría riesgo de intermitencias (varias WebKit a la vez, tiempos sensibles) para ganar pocos minutos. Reabrir si la suite vuelve a pasar de ~15 min.
