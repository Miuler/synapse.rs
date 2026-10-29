import { EditorView } from '@codemirror/view';
import { vaultApi } from '@entities/vault-item';
import { isTauriEnvironment } from '@shared/api';
import { activeFilePathFacet } from './editor-facets';

/**
 * Formatea una fecha en formato YYYYMMDD_HHmmss.
 */
export function formatAttachmentTimestamp(date: Date = new Date()): string {
  const pad = (n: number) => String(n).padStart(2, '0');
  const year = date.getFullYear();
  const month = pad(date.getMonth() + 1);
  const day = pad(date.getDate());
  const hours = pad(date.getHours());
  const minutes = pad(date.getMinutes());
  const seconds = pad(date.getSeconds());
  return `${year}${month}${day}_${hours}${minutes}${seconds}`;
}

/**
 * Deduce la extensión de archivo a partir del MIME type o nombre de archivo.
 */
export function getExtensionFromMimeOrName(type: string, name?: string): string {
  if (name) {
    const extMatch = name.match(/\.([a-zA-Z0-9]+)$/);
    if (extMatch) {
      const ext = extMatch[1].toLowerCase();
      if (['webp', 'png', 'jpg', 'jpeg', 'gif', 'bmp', 'svg', 'avif'].includes(ext)) {
        return ext === 'jpeg' ? 'jpg' : ext;
      }
    }
  }

  const mime = type.toLowerCase();
  if (mime.includes('webp')) return 'webp';
  if (mime.includes('png')) return 'png';
  if (mime.includes('jpeg') || mime.includes('jpg')) return 'jpg';
  if (mime.includes('gif')) return 'gif';
  if (mime.includes('svg')) return 'svg';
  if (mime.includes('avif')) return 'avif';
  if (mime.includes('bmp')) return 'bmp';

  return 'webp';
}

/**
 * Convierte un Blob / File a una Data URL Base64.
 */
export async function fileToBase64DataUrl(blob: Blob): Promise<string> {
  if (typeof FileReader !== 'undefined') {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onloadend = () => {
        if (typeof reader.result === 'string') {
          resolve(reader.result);
        } else {
          reject(new Error('No se pudo convertir el blob a Data URL'));
        }
      };
      reader.onerror = () => reject(reader.error);
      reader.readAsDataURL(blob);
    });
  }

  // Fallback para entornos sin FileReader nativo
  const buffer = await blob.arrayBuffer();
  const bytes = new Uint8Array(buffer);
  let binary = '';
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  const base64 = typeof btoa === 'function' ? btoa(binary) : '';
  const mime = blob.type || 'application/octet-stream';
  return `data:${mime};base64,${base64}`;
}

/**
 * Obtiene el directorio contenedor de una ruta relativa de archivo.
 */
export function getDirectoryFromPath(filePath: string): string {
  const clean = filePath.trim().replace(/^\.?\/+/, '');
  const lastSlash = clean.lastIndexOf('/');
  return lastSlash >= 0 ? clean.slice(0, lastSlash) : '';
}

/**
 * Guarda una imagen pegada en la subcarpeta `attachments/` de la nota actual
 * y devuelve el enlace transcluido `![[image_...]]`.
 */
export async function savePastedImage(
  imageBlob: Blob | File,
  currentNotePath?: string | null
): Promise<{ wikilink: string; relativePath: string; fileName: string }> {
  const ext = getExtensionFromMimeOrName(
    imageBlob.type,
    imageBlob instanceof File ? imageBlob.name : undefined
  );
  const timestamp = formatAttachmentTimestamp();
  const baseName = `image_${timestamp}.${ext}`;

  // Carpeta base de la nota activa
  const noteDir = currentNotePath ? getDirectoryFromPath(currentNotePath) : '';
  const attachmentsDir = noteDir ? `${noteDir}/attachments` : 'attachments';

  // Asegurar unicidad si se pegan varias en el mismo segundo
  let fileName = baseName;
  let relativePath = `${attachmentsDir}/${fileName}`;
  let counter = 1;

  if (isTauriEnvironment()) {
    while (await vaultApi.checkVaultPathExists(relativePath)) {
      fileName = `image_${timestamp}_${counter}.${ext}`;
      relativePath = `${attachmentsDir}/${fileName}`;
      counter++;
    }
  }

  // Convertir a Data URL y guardar mediante el backend Rust
  const base64Data = await fileToBase64DataUrl(imageBlob);

  if (isTauriEnvironment()) {
    await vaultApi.saveBinaryAsset(relativePath, base64Data);
  }

  return {
    wikilink: `![[${fileName}]]`,
    relativePath,
    fileName,
  };
}

/**
 * Extensión de CodeMirror que intercepta el pegado de imágenes en el editor Markdown.
 */
export const imagePasteExtension = EditorView.domEventHandlers({
  paste(event, view) {
    const clipboardData = event.clipboardData;
    if (!clipboardData) return false;

    // Buscar si hay items de imagen en el portapapeles
    const items = Array.from(clipboardData.items || []);
    const imageItem = items.find((item) => item.type.startsWith('image/'));

    if (!imageItem) {
      // También comprobar en files si items no expone directamente
      const files = Array.from(clipboardData.files || []);
      const imageFile = files.find((f) => f.type.startsWith('image/'));
      if (!imageFile) return false;
      
      event.preventDefault();
      handleImagePasteAsync(imageFile, view);
      return true;
    }

    const file = imageItem.getAsFile();
    if (!file) return false;

    event.preventDefault();
    handleImagePasteAsync(file, view);
    return true;
  },
});

async function handleImagePasteAsync(file: File, view: EditorView) {
  try {
    const currentPath = view.state.facet(activeFilePathFacet);
    const { wikilink } = await savePastedImage(file, currentPath);

    // Insertar el wikilink en la posición actual del cursor o reemplazar la selección
    const selection = view.state.selection.main;
    view.dispatch({
      changes: {
        from: selection.from,
        to: selection.to,
        insert: wikilink,
      },
      selection: { anchor: selection.from + wikilink.length },
      scrollIntoView: true,
    });
  } catch (err) {
    console.error('Error al guardar y enlazar la imagen pegada:', err);
  }
}
