// Soltar archivos. En Linux, si lo arrastrado lleva archivos, WebKitGTK oculta su contenido a la página
// (también el de un arrastre propio) y Tauri se queda con el soltado. Por eso se atiende su evento
// nativo, que trae las rutas reales, y se avisa al elemento donde se soltó.
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { getCurrentWindow } from '@tauri-apps/api/window';

export interface NativeDrop {
  /** Rutas relativas a la bóveda si `internal`; absolutas si vienen de otra aplicación. */
  paths: string[];
  internal: boolean;
}

/** Se emite en el elemento bajo el cursor, al soltar. */
export const NATIVE_DROP = 'synapse-native-drop';

let internal: string[] | null = null;
let over: { el: Element; at: number } | null = null;

/** Empieza un arrastre desde el explorador: los datos del evento no llegan a destino en Linux. */
export function startInternalDrag(paths: string[]): void {
  internal = paths;
}

export function endInternalDrag(): void {
  // El soltado nativo puede llegar justo después de `dragend`
  const ended = internal;
  setTimeout(() => {
    if (internal === ended) internal = null;
  }, 500);
}

/** Rutas de un arrastre del explorador en curso, o `null`. */
export function internalDragPaths(): string[] | null {
  return internal;
}

/** El arrastre puede traer archivos de otra aplicación. */
export function carriesFiles(e: DragEvent): boolean {
  const types = e.dataTransfer?.types ?? [];
  return types.includes('Files') || types.includes('text/uri-list');
}

/** Solo rutas absolutas: un enlace web arrastrado también llega como «archivo». */
export function droppedFilePaths(paths: string[]): string[] {
  return paths.filter((p) => /^(\/|[A-Za-z]:[\\/]|\\\\)/.test(p));
}

/** Elemento en la posición del evento nativo: lógica en Linux, física en los demás. */
async function elementAt(x: number, y: number): Promise<Element | null> {
  const win = getCurrentWindow();
  const size = await win.innerSize();
  const width = navigator.userAgent.includes('Linux') ? size.width / (await win.scaleFactor()) : size.width;
  const ratio = width > 0 ? innerWidth / width : 1;
  return document.elementFromPoint(x * ratio, y * ratio);
}

/** Atiende el soltado nativo de esta ventana. Devuelve cómo dejar de hacerlo. */
export function listenNativeDrop(): () => void {
  const onDragOver = (e: DragEvent) => {
    if (e.target instanceof Element) over = { el: e.target, at: Date.now() };
  };
  window.addEventListener('dragover', onDragOver, true);
  const off = getCurrentWebview().onDragDropEvent(async ({ payload }) => {
    if (payload.type !== 'drop') return;
    const paths = internal ?? droppedFilePaths(payload.paths);
    const detail: NativeDrop = { paths, internal: internal !== null };
    internal = null;
    // En Linux la página sí ve por dónde pasa el cursor: es más fiable que convertir coordenadas
    const el = over && Date.now() - over.at < 1500 ? over.el : await elementAt(payload.position.x, payload.position.y);
    over = null;
    if (paths.length) el?.dispatchEvent(new CustomEvent(NATIVE_DROP, { bubbles: true, detail }));
  });
  return () => {
    window.removeEventListener('dragover', onDragOver, true);
    void off.then((f) => f());
  };
}
