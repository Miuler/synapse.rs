---
schema_version: 9
id: syna-b8n6
title: 'Índice: código con tree-sitter (símbolos, comentarios y docstrings con su línea)'
priority: P3
status: open
deps: []
tags:
- busqueda
- indice
owner: null
created_at: 2026-11-18T09:21:23.338708648Z
---

Hoy el código se indexa crudo y cortado a max_extracted_chars. Con tree-sitter (Rust, TS, Java, Python, Go, COBOL vía tree-sitter-cobol) indexar funciones/clases/tipos, comentarios y docstrings, cada uno con su línea. Relacionado con indexar por elemento.