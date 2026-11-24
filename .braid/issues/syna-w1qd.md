---
schema_version: 9
id: syna-w1qd
title: 'Seguridad: terminar la insignia OpenSSF (proyecto 17082) y las alertas de Scorecard'
priority: P3
status: open
deps: []
tags:
- seguridad
owner: null
created_at: 2026-11-24T18:28:20.591632601Z
acceptance:
- Nivel passing en bestpractices.dev y alertas 4 y 2 cerradas
- Decidido qué hacer con las alertas 1, 3, 5 y 6 (corregir o descartar con motivo)
---

Estado al 2026-11-24: 84 % guardado en https://www.bestpractices.dev/en/projects/17082. Licencia GPL-3.0, reporte privado, docs/reference.md y web ya hechos.

Insignia, obligatorios pendientes:
- documentation_interface: Met (docs/reference.md). vulnerability_report_private: Met (SECURITY.md).
- crypto_random y dynamic_analysis_fixed: N/A.
- know_secure_design y know_common_errors: los afirma el autor.
- vulnerabilities_fixed_60_days: decidir si cuentan glib (GHSA-wrw7-89jp-8q8g) y los avisos npm transitivos.
Recomendaciones en Unmet: english (README y web en inglés), test_most (cobertura), dynamic_analysis (fuzzing).

Scorecard: 1 protección de main (ligera: sin force push ni borrado), 3 revisión de código (proyecto de una persona: descartar), 5 fuzzing, 6 dependencias (28 avisos Rust sin vulnerabilidades reales + 10 de bun audit).
Dependencias que parecen quitables: heapless 0.7 y atomic-polyfill (postcard sin opciones por defecto), adler (pixelmatch), serde_yaml; overrides de nanoid, katex y minimatch probando Excalidraw y Mermaid. No dependen de nosotros: glib, heapless 0.8 (gix), zvariant/zbus_names.

Riesgos conocidos sin tarea propia: csp null; MCP sin autenticación y con CORS abierto; un script inyectado podría manejar una sesión de agente o escribir .git/config; save_theme_dirs acepta cualquier carpeta.