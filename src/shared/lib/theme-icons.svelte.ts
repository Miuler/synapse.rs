// Íconos que un tema puede reemplazar con variables CSS que apuntan a una imagen:
//   --icon-folder: url("data:image/svg+xml,...");  --icon-folder-open: url(...);
// Se leen del estilo calculado de <html>; hay que volver a leerlos cuando cambia el CSS del tema
// (`refreshThemeIcons`) o el modo claro/oscuro (se observa `data-theme`).

export const themeIcons = $state({ folder: '', folderOpen: '' });

export function refreshThemeIcons() {
  if (typeof document === 'undefined') return;
  const css = getComputedStyle(document.documentElement);
  const read = (name: string) => {
    const v = css.getPropertyValue(name).trim();
    return v && v !== 'none' ? v : '';
  };
  themeIcons.folder = read('--icon-folder');
  themeIcons.folderOpen = read('--icon-folder-open') || themeIcons.folder;
}

if (typeof document !== 'undefined') {
  new MutationObserver(refreshThemeIcons).observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-theme'],
  });
}
