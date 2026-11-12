# Temas y fragmentos CSS

Cada bóveda puede tener su propio aspecto con archivos `.css`:

| Carpeta en la bóveda | Qué es | Cómo se activa |
|---|---|---|
| `.synapse/themes/*.css` | Temas. Se usa uno a la vez. | Configuración → Apariencia → **Tema personalizado** |
| `.synapse/snippets/*.css` | Fragmentos: retoques sueltos. | Configuración → Apariencia, un interruptor por fragmento |

- Lo elegido se guarda en `.synapse/appearance.json` de la bóveda.
- El tema se carga primero y los fragmentos después, así que un fragmento puede retocar un tema.
- Los cambios se aplican sin reiniciar. Si editas o agregas archivos, pulsa **Recargar** en
  Configuración: la app no vigila la carpeta `.synapse`.
- La app trae temas integrados, con versión clara y oscura, que aparecen en la lista sin copiar nada
  (están en [`src/entities/appearance/themes/`](../src/entities/appearance/themes/)):
  - **Papel** (`papel.css`): estilo máquina de escribir.
  - **Neón** (`neon.css`): cyberpunk, con fondo de rejilla, paneles translúcidos y títulos con brillo.
  - **Rosa** (`rosa.css`): tonos rosa suaves, frambuesa en claro y ciruela en oscuro.
  - **Islas** (`islas.css`): paneles flotantes con esquinas redondeadas sobre un marco más oscuro.
    No trae colores propios; se combina con los demás en **Papel Islas**, **Neón Islas** y **Rosa Islas**.
    Se ajusta con `--island-gap`, `--island-radius`, `--island-frame` y `--island-border`.

  Sirven de ejemplo para escribir los tuyos. Para modificar uno, copia su archivo a
  `.synapse/themes/` con el mismo nombre: el de la bóveda reemplaza al integrado.

## Modo claro y oscuro

La app marca el modo en `<html data-theme="light|dark">`. Un tema define sus variables para cada modo:

```css
html[data-theme='light'] { --bg-primary: #f7f1e3; }
html[data-theme='dark']  { --bg-primary: #1e1d1a; }
html[data-theme]         { --font-text: 'Courier New', monospace; } /* ambos modos */
```

## 1. Variables (lo más estable: preferirlas siempre)

### Colores

| Variable | Uso |
|---|---|
| `--bg-primary`, `--bg-secondary`, `--bg-tertiary` | Fondos: contenido, paneles, elementos hundidos |
| `--text-primary`, `--text-secondary`, `--text-muted` | Textos: principal, secundario, atenuado |
| `--border-primary`, `--border-secondary`, `--border-subtle` | Bordes |
| `--accent`, `--accent-bg`, `--accent-border` | Color de acento, su fondo suave y su borde |
| `--on-accent` | Texto sobre fondos de color (botones de acento, insignias) |
| `--hover-bg`, `--hover-bg-subtle`, `--active-bg`, `--badge-bg` | Estados al pasar el ratón, activo e insignias |
| `--overlay-bg` | Fondo oscurecido detrás de los diálogos |
| `--card-shadow`, `--card-shadow-hover`, `--popover-shadow`, `--modal-shadow` | Sombras |
| `--success`, `--warning`, `--danger`, `--purple` (+ `-bg`, `-border`) | Estados semánticos |
| `--error-bg`, `--error-text`, `--error-border` | Mensajes de error |
| `--git-modified`, `--git-staged`, `--git-untracked`, `--git-stashed` (+ `-bg`, `-border`) | Estados de Git |
| `--syntax-keyword`, `--syntax-comment`, `--syntax-string`, `--syntax-function`, `--syntax-variable`, `--syntax-type`, `--syntax-number` | Resaltado de código |
| `--code-inline-bg`, `--table-alt-row` | Código en línea y filas alternas de tablas |
| `--icon-folder-color`, `--icon-folder-git-color`, `--icon-file-*-color` | Color de los íconos del explorador |

### Fuentes

| Variable | Uso |
|---|---|
| `--font-text` | Texto de las notas, en lectura y edición |
| `--main-font` | Interfaz: menús, explorador, diálogos |
| `--code-font` | Código y bloques monoespaciados |

Se puede usar cualquier fuente instalada por su nombre, o cargar una con `@font-face` desde un archivo.

### Títulos por nivel

Para `N` de 1 a 6: `--hN-font`, `--hN-size`, `--hN-weight`, `--hN-color`. Se aplican igual en la vista
de lectura y en el editor en vivo. El modo código fuente conserva su estilo propio.

### Íconos de carpeta

`--icon-folder` y `--icon-folder-open` reciben un `url(...)` a una imagen (SVG en línea, PNG…). Reemplazan
el ícono de la app conservando los colores de la imagen. Sin definirlas se usa el ícono normal.

## 2. Zonas públicas

Clases estables para apuntar a cada parte de la app. No cambian entre versiones; las demás clases son
internas y pueden cambiar.

| Clase | Zona |
|---|---|
| `.synapse-app` | Contenedor de toda la ventana |
| `.synapse-ribbon` | Barra vertical de herramientas |
| `.synapse-sidebar` / `.synapse-explorer` | Barra lateral con el explorador de archivos |
| `.synapse-explorer-item` | Cada fila del explorador (`data-path` con la ruta; `.synapse-explorer-folder` o `.synapse-explorer-file`) |
| `.synapse-right-sidebar` | Barra lateral derecha (relaciones) |
| `.synapse-header` | Cabecera con las pestañas |
| `.synapse-tabs` / `.synapse-tab` | Barra de pestañas / cada pestaña (`.active` en la actual) |
| `.synapse-main` | Área principal de trabajo |
| `.synapse-pane` | Cada panel cuando la vista está dividida |
| `.synapse-note-title` | Título de la nota con su ruta de títulos |
| `.synapse-editor` | Editor de Markdown |
| `.synapse-reading` | Vista de lectura (el contenido está en `.markdown-body`) |
| `.synapse-empty-tab` | Pestaña vacía con el buscador |
| `.synapse-statusbar` | Barra de estado |
| `.synapse-dialog` | Cualquier diálogo; además `.synapse-settings`, `.synapse-quick-open`, `.synapse-command-palette` |
| `.synapse-breadcrumb` | Ruta de la nota |
| `.synapse-folder-icon` | Ícono de carpeta (`.synapse-folder-open`, `.synapse-folder-closed`) |

Ejemplo: íconos distintos según la carpeta.

```css
.synapse-explorer-item[data-path='Diario'] .synapse-folder-icon { color: #c2410c; }
```

## 3. Contenido de la nota

Dentro de `.markdown-body` (vista de lectura) los elementos son HTML normal: `h1`–`h6`, `p`, `a`,
`blockquote`, `ul`, `table`, `code`, `pre`… En el editor, CodeMirror usa `.cm-line` para cada línea.

## Consejos

- Si los fondos son translúcidos, pon algo detrás en `.synapse-app` (una imagen o un degradado).
- En Linux los `<select>` usan el control nativo de GTK, que ignora el fondo. Para pintarlos, ponles
  `appearance: none` y dibuja la flecha con `background-image` (ver `neon.css`).
- Cambiar la fuente o el espaciado del título de la nota (`.synapse-note-title input`) descuadra su
  ancho; mejor limitarse a color y sombras.

## Límites

- Los temas cambian el aspecto, no la estructura: no agregan ni quitan paneles ni cambian su
  comportamiento (aunque pueden ocultar o recolocar algo con CSS).
- Lo que se dibuja en `canvas` (grafo de notas, interior de Excalidraw, láser de diagramas, diff de
  píxeles) todavía usa colores propios.
