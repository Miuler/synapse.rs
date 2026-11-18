/**
 * Excalidraw dibuja el cuadro de edición de texto a tamaño de escena y lo amplía con
 * `transform: translate(…) scale(zoom) rotate(…)`. WebKitGTK con DMABUF rasteriza esa capa a escala 1
 * y la estira, así que el texto se ve borroso (más cuanto más pequeña es la letra y mayor el zoom).
 * Aquí se quita la escala y se aplica a los tamaños: mismo sitio y tamaño en pantalla, texto nítido.
 */

interface BoxStyle {
  transform: string;
  left: string;
  top: string;
  width: string;
  height: string;
  fontSize: string;
  maxWidth: string;
  maxHeight: string;
}

/** Estilos sin escala equivalentes, o `null` si no hay escala que quitar. */
export function unscaledTextEditorStyle(st: BoxStyle): Partial<BoxStyle> | null {
  const scale = /scale\(\s*([-\d.e]+)\s*\)/.exec(st.transform);
  const z = scale ? Number(scale[1]) : 1;
  if (!Number.isFinite(z) || z <= 0 || Math.abs(z - 1) < 1e-6) return null;
  const translate = /translate\(([^)]*)\)/.exec(st.transform)?.[1].split(',') ?? [];
  const a = parseFloat(translate[0] ?? '0') || 0;
  const b = parseFloat(translate[1] ?? '0') || 0;
  const rotate = /rotate\(([^)]*)\)/.exec(st.transform)?.[1] ?? '0deg';
  const px = (v: string) => parseFloat(v) || 0;
  const w = px(st.width);
  const h = px(st.height);
  // La escala es desde el centro (transform-origin por defecto): el centro no se mueve
  const cx = px(st.left) + a + w / 2;
  const cy = px(st.top) + b + h / 2;
  const out: Partial<BoxStyle> = {
    transform: `rotate(${rotate})`,
    width: `${w * z}px`,
    height: `${h * z}px`,
    left: `${cx - (w * z) / 2}px`,
    top: `${cy - (h * z) / 2}px`,
    fontSize: `${px(st.fontSize) * z}px`,
  };
  if (st.maxWidth.endsWith('px')) out.maxWidth = `${px(st.maxWidth) * z}px`;
  if (st.maxHeight.endsWith('px')) out.maxHeight = `${px(st.maxHeight) * z}px`;
  return out;
}

/** Vigila `root` y mantiene sin escala el cuadro de edición de texto de Excalidraw. Devuelve cómo dejar de vigilar. */
export function keepTextEditorSharp(root: HTMLElement): () => void {
  const fix = (el: HTMLElement) => {
    const next = unscaledTextEditorStyle(el.style);
    if (next) Object.assign(el.style, next);
  };
  const styleObserver = new MutationObserver((records) => {
    for (const r of records) fix(r.target as HTMLElement);
  });
  const watched = new WeakSet<Element>();
  const scan = () => {
    for (const el of root.querySelectorAll<HTMLElement>('textarea.excalidraw-wysiwyg')) {
      if (watched.has(el)) continue;
      watched.add(el);
      fix(el);
      styleObserver.observe(el, { attributes: true, attributeFilter: ['style'] });
    }
  };
  const treeObserver = new MutationObserver(scan);
  treeObserver.observe(root, { childList: true, subtree: true });
  scan();
  return () => {
    treeObserver.disconnect();
    styleObserver.disconnect();
  };
}
