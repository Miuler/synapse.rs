---
schema_version: 9
id: syna-nvve
title: 'Tableros: definir qué va en el frontmatter y qué en el detalle de cada issue'
priority: P2
status: open
type: design
deps: []
tags:
- tableros
owner: null
created_at: 2026-11-27T14:47:27.485971229Z
acceptance:
- Formato acordado con el usuario y reflejado en openspec/changes/add-issue-providers
---

Pedido por el usuario (2026-11-27): lo propio de cada issue debe ir en el detalle del issue y no en el frontmatter; hay que definirlo juntos antes de implementarlo.

Hoy `azure_devops.links` guarda datos por tarea en el frontmatter. La propuesta add-issue-providers (decisiones 4 y 5 de design.md) habría que ajustarla.

## Propuesta por confirmar

- Frontmatter, solo configuración del tablero: type, default_view, columns, column_widths, campos visibles, conexión, proyecto, consulta y estados por columna.
- Tabla, una fila por tarea: id, título, columna, inicio, duración, dependencia y progreso; el orden de filas es el orden del tablero.
- Detalle de cada issue (`### <id>` bajo `## Detalle`): tipo, asignado, referencia, datos de la última sincronización, descripción (como cita, para que sus encabezados no rompan el formato) y actividad con su clase (comment | history).

Alternativa: un archivo Markdown por issue, con sus datos en su propio frontmatter.

## Decisión parcial (2026-11-27)

- Un solo archivo por ahora; pasar a un archivo por issue queda para más adelante.
- Separador confirmado por el usuario (2026-11-27): el encabezado `### <id>` separa y sirve de ancla; no usar `---` como delimitador (en Markdown es una línea horizontal, convierte en encabezado la línea anterior y puede aparecer en una descripción).
- Para abaratar el cambio posterior: leer y escribir cada sección de issue con una función propia, con los mismos campos que tendría un archivo por issue.
- Tableros sintéticos (formato actual, build de desarrollo): leer 4 MB tarda unos 11 ms; el Kanban de 1000 tareas abre en menos de 1 s; el límite lo pone la vista de tabla.
- Claves del formato en inglés (confirmado 2026-11-27).

## Escala de equipo (por decidir antes de implementar)

El usuario plantea coordinar como PM un equipo de más de 20 personas. Recomendación pendiente de respuesta: seguir con archivos de texto, un archivo por issue y el `.board.md` como definición (origen, consulta, columnas y orden); índice en memoria en Rust derivado de los archivos y actualizado por el observador, como `notes: DashMap` del motor de navegación, sin base de datos; descripción e histórico se leen al abrir la tarea; filtros y listas virtualizadas en la interfaz; traer solo lo cambiado. El equipo sigue trabajando en Azure DevOps y los archivos son la copia local.

## Decisión (2026-11-27)

Un archivo por tarea en `Nombre.board/`, junto a `Nombre.board.md` (origen, filtro, columnas y orden); la carpeta queda protegida con un candado que no se puede quitar y el filtro se pide al crear el tablero. Sustituye a las secciones anteriores (un solo archivo y `### <id>`). Reflejado en openspec/changes/add-issue-providers; falta que el usuario revise la propuesta.

