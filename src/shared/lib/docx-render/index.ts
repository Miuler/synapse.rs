/**
 * Dibuja un documento de Word con docx-preview (se carga al abrir el primero). `className` debe ser
 * distinto en cada documento visible a la vez: con él se nombran sus estilos.
 */
export async function renderDocx(data: ArrayBuffer, body: HTMLElement, styles: HTMLElement, className = 'docx') {
  const { renderAsync } = await import('docx-preview');
  body.replaceChildren();
  styles.replaceChildren();
  await renderAsync(data, body, styles, {
    className,
    inWrapper: true,
    breakPages: true,
    ignoreLastRenderedPageBreak: true,
    renderHeaders: true,
    renderFooters: true,
    renderFootnotes: true,
    renderEndnotes: true,
    renderComments: false,
    renderChanges: false,
    experimental: false,
    useBase64URL: true,
  });
}
