---
schema_version: 9
id: syna-jqrv
title: 'e2e: correr los archivos en paralelo (3-4 procesos), primero medir si conviene'
priority: P3
status: open
deps: []
tags:
- e2e
- pruebas
owner: null
created_at: 2026-11-17T19:07:47.221464883Z
acceptance:
- Medido el tiempo total y los fallos intermitentes con 1, 2, 3 y 4 procesos en esta máquina
- Si conviene, test:e2e usa el número de procesos que rindió mejor; si no, queda documentado por qué se descartó
---

Hoy la suite e2e corre archivo por archivo con un solo WebDriver y cada archivo arranca la app desde cero (~15-30 s), unos 20+ min en total. Idea: varios procesos a la vez, cada uno con su puerto de WebDriver, su SYNAPSE_CONFIG_DIR y su vault (ya se aíslan por vault).

OJO: antes de adoptarlo hay que probar que de verdad sea más rápido. Varias instancias de Tauri/WebKit a la vez pueden saturar CPU/RAM/GPU y volver todo más lento o más intermitente (las e2e ya tienen tiempos sensibles). Medir con 1, 2, 3 y 4 procesos el tiempo total y la tasa de fallos intermitentes, y quedarse con el que rinda mejor (o descartarlo). Hacerlo al final, después del corte rápido del harness, las pausas fijas y la agrupación de archivos.