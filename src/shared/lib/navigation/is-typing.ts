/** Si la tecla va a un sitio donde se escribe (campo de texto o editor de código). */
export function isTyping(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return (
    target.isContentEditable ||
    target.closest('input, textarea, select, [contenteditable=""], [contenteditable="true"], .cm-editor') !== null
  );
}
