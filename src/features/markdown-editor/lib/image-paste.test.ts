import { describe, expect, it } from 'bun:test';
import {
  formatAttachmentTimestamp,
  getExtensionFromMimeOrName,
  getDirectoryFromPath,
  savePastedImage,
} from './image-paste';

describe('image-paste lib', () => {
  it('formats attachment timestamp consistently with YYYYMMDD_HHmmss', () => {
    const fixedDate = new Date(2026, 9, 29, 13, 45, 30); // 29 de Octubre 2026
    const formatted = formatAttachmentTimestamp(fixedDate);
    expect(formatted).toBe('20261029_134530');
  });

  it('detects correct extensions from MIME or file name', () => {
    expect(getExtensionFromMimeOrName('image/webp')).toBe('webp');
    expect(getExtensionFromMimeOrName('image/png')).toBe('png');
    expect(getExtensionFromMimeOrName('image/jpeg')).toBe('jpg');
    expect(getExtensionFromMimeOrName('image/svg+xml')).toBe('svg');
    expect(getExtensionFromMimeOrName('image/gif')).toBe('gif');
    expect(getExtensionFromMimeOrName('application/octet-stream', 'screenshot.png')).toBe('png');
    expect(getExtensionFromMimeOrName('image/jpeg', 'photo.jpeg')).toBe('jpg');
    expect(getExtensionFromMimeOrName('unknown')).toBe('webp');
  });

  it('extracts parent directory from note path', () => {
    expect(getDirectoryFromPath('notas/diarias/hoy.md')).toBe('notas/diarias');
    expect(getDirectoryFromPath('hoy.md')).toBe('');
    expect(getDirectoryFromPath('/raiz/proyecto/nota.md')).toBe('raiz/proyecto');
  });

  it('builds correct attachments path and transclusion wikilink format ![[...]]', async () => {
    const fakeBlob = new Blob(['fake image content'], { type: 'image/webp' });
    const result = await savePastedImage(fakeBlob, 'trabajo/proyectos/doc.md');

    expect(result.relativePath).toMatch(/^trabajo\/proyectos\/attachments\/image_\d{8}_\d{6}\.webp$/);
    expect(result.wikilink).toBe(`![[${result.fileName}]]`);
    expect(result.wikilink).toMatch(/^!\[\[image_\d{8}_\d{6}\.webp\]\]$/);
  });

  it('builds attachments in root attachments/ when note is in root', async () => {
    const fakeBlob = new Blob(['fake image content'], { type: 'image/png' });
    const result = await savePastedImage(fakeBlob, 'root-note.md');

    expect(result.relativePath).toMatch(/^attachments\/image_\d{8}_\d{6}\.png$/);
    expect(result.wikilink).toBe(`![[${result.fileName}]]`);
  });
});
