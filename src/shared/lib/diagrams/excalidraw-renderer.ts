import { vaultRepository } from '@shared/repositories';
import { parseExcalidrawContent } from '@shared/obsidian-api';

interface CacheEntry {
  svg: string;
  timestamp: number;
}

// Caché en memoria RAM de nivel 1 (LRU simple)
const memoryCache = new Map<string, CacheEntry>();
const MAX_RAM_CACHE_ENTRIES = 100;

// Mutex simple para evitar renders concurrentes del mismo archivo
const pendingRenders = new Map<string, Promise<string | null>>();

/**
 * Función de hashing rápido FNV-1a para generar una clave de caché determinista y corta
 */
function fastHash(str: string): string {
  let hash = 2166136261;
  for (let i = 0; i < str.length; i++) {
    hash ^= str.charCodeAt(i);
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0).toString(16).padStart(8, '0');
}

/**
 * Renderiza un documento Excalidraw a SVG utilizando exportToSvg con caché multinivel:
 * 1. Nivel 1: Memoria RAM (LRU) - 0ms
 * 2. Nivel 2: Disco (<vault>/.synapse/cache/excalidraw/<hash>_<theme>.svg) - <2ms
 * 3. Nivel 3: Motor @excalidraw/excalidraw exportToSvg
 */
export async function renderExcalidrawToSvg(
  rawContent: string,
  filePath?: string,
  isDark = false
): Promise<string | null> {
  if (!rawContent || !rawContent.trim()) {
    return null;
  }

  const themeSuffix = isDark ? 'dark' : 'light';
  const contentHash = fastHash(rawContent);
  const cacheKey = `${contentHash}_${themeSuffix}`;
  const diskRelativePath = `.synapse/cache/excalidraw/${cacheKey}.svg`;

  // 1. Verificar Nivel 1: Memoria RAM
  const ramHit = memoryCache.get(cacheKey);
  if (ramHit) {
    ramHit.timestamp = Date.now();
    return ramHit.svg;
  }

  // 2. Si ya hay un render en progreso para esta clave, reutilizar la promesa
  if (pendingRenders.has(cacheKey)) {
    return pendingRenders.get(cacheKey)!;
  }

  const renderPromise = (async () => {
    try {
      // 3. Verificar Nivel 2: Disco (<vault>/.synapse/cache/excalidraw/<cacheKey>.svg)
      try {
        const diskFile = await vaultRepository.readNote(diskRelativePath);
        if (diskFile && diskFile.content && diskFile.content.includes('<svg')) {
          setMemoryCache(cacheKey, diskFile.content);
          return diskFile.content;
        }
      } catch {
        // Archivo no existe en disco aún, continuar al renderizado
      }

      // 4. Nivel 3: Renderizar con @excalidraw/excalidraw
      const parsedDoc = parseExcalidrawContent(rawContent);
      const elements = parsedDoc.scene.elements || [];
      const appState = parsedDoc.scene.appState || {};
      const files = parsedDoc.scene.files || {};

      if (!elements || elements.length === 0) {
        return null;
      }

      // Importar dinámicamente exportToSvg
      const { exportToSvg } = await import('@excalidraw/excalidraw');

      const svgEl = await exportToSvg({
        elements,
        appState: {
          ...appState,
          theme: isDark ? 'dark' : 'light',
          exportWithDarkMode: isDark,
          exportBackground: true,
          viewBackgroundColor: isDark ? '#121212' : '#ffffff',
        },
        files,
        exportPadding: 20,
      });

      if (!svgEl) {
        return null;
      }

      // Ajustes responsivos en el SVG generado
      svgEl.setAttribute('width', '100%');
      svgEl.setAttribute('height', '100%');
      svgEl.style.maxWidth = '100%';
      svgEl.style.maxHeight = '100%';
      svgEl.style.display = 'block';

      const serializer = new XMLSerializer();
      const svgString = serializer.serializeToString(svgEl);

      // Guardar en Nivel 1 (RAM)
      setMemoryCache(cacheKey, svgString);

      // Guardar en Nivel 2 (Disco) de forma asíncrona sin bloquear
      vaultRepository
        .saveNote({
          relativePath: diskRelativePath,
          title: `${cacheKey}.svg`,
          content: svgString,
          encoding: 'UTF-8',
        })
        .catch((err) => {
          console.warn('[ExcalidrawCache] No se pudo guardar caché en disco:', err);
        });

      return svgString;
    } catch (err) {
      console.error('[ExcalidrawCache] Error renderizando Excalidraw a SVG:', err);
      return null;
    } finally {
      pendingRenders.delete(cacheKey);
    }
  })();

  pendingRenders.set(cacheKey, renderPromise);
  return renderPromise;
}

function setMemoryCache(key: string, svg: string) {
  if (memoryCache.size >= MAX_RAM_CACHE_ENTRIES) {
    // Eliminar el más antiguo
    const oldestKey = memoryCache.keys().next().value;
    if (oldestKey) memoryCache.delete(oldestKey);
  }
  memoryCache.set(key, { svg, timestamp: Date.now() });
}

/**
 * Limpia la caché en memoria RAM.
 */
export function clearExcalidrawMemoryCache(): void {
  memoryCache.clear();
}
