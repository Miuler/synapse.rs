---
schema_version: 9
id: syna-5moc
title: 'Web: insignia OpenSSF en el pie al llegar a passing'
priority: P3
status: open
deps:
- syna-w1qd
tags:
- seguridad
- web
owner: null
created_at: 2026-11-24T18:38:30.525439473Z
acceptance:
- Insignia OpenSSF Best Practices (solo esa, en verde) en el pie de ambos idiomas, o descartada
---

El 2026-11-24 se quitaron de la web las insignias OpenSSF Best Practices (in progress 84%) y Scorecard (6.6): a un visitante no le dicen nada y «in progress» suena a incompleto. Siguen en el README.

Al alcanzar passing en https://www.bestpractices.dev/en/projects/17082, valorar ponerla en el pie (web/template.html, .foot-end), en fila y con altura de 20 px. La página dibuja las imágenes como bloque: cuidar que no se apilen.