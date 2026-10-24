import type { SynapseApp } from './app';
import { TFile } from './types';
import {
  extractWikilinksFromElements,
  serializeExcalidrawMarkdown,
  type ExcalidrawScene,
} from './excalidraw-parser';

export interface ElementOptions {
  id?: string;
  strokeColor?: string;
  backgroundColor?: string;
  fillStyle?: 'hachure' | 'cross-hatch' | 'solid';
  strokeWidth?: number;
  roughness?: number;
  opacity?: number;
  groupIds?: string[];
  roundness?: { type: number } | null;
  locked?: boolean;
  link?: string | null;
  fontSize?: number;
  fontFamily?: number;
  textAlign?: 'left' | 'center' | 'right';
  verticalAlign?: 'top' | 'middle' | 'bottom';
}

function randomId(): string {
  return Math.random().toString(36).substring(2, 10);
}

/**
 * Motor de automatización y scripting ExcalidrawAutomate (`ea`).
 * Es la API universal utilizada por scripts comunitarios, macros y plugins de Obsidian
 * para interactuar y generar dibujos de Excalidraw programáticamente.
 */
export class ExcalidrawAutomate {
  private elements: any[] = [];
  private files: Record<string, any> = {};
  private appState: Record<string, any> = {
    viewBackgroundColor: '#ffffff',
    currentItemStrokeColor: '#1e1e1e',
    currentItemBackgroundColor: 'transparent',
    currentItemFillStyle: 'solid',
    currentItemStrokeWidth: 2,
    currentItemRoughness: 1,
  };

  constructor(public readonly app?: SynapseApp) {}

  /**
   * Limpia el lienzo de trabajo interno del automatizador.
   */
  reset(): void {
    this.elements = [];
    this.files = {};
    this.appState = {
      viewBackgroundColor: '#ffffff',
      currentItemStrokeColor: '#1e1e1e',
      currentItemBackgroundColor: 'transparent',
      currentItemFillStyle: 'solid',
      currentItemStrokeWidth: 2,
      currentItemRoughness: 1,
    };
  }

  /**
   * Obtiene la copia de todos los elementos actualmente en el buffer de automatización.
   */
  getElements(): any[] {
    return [...this.elements];
  }

  /**
   * Obtiene un elemento por su ID.
   */
  getElement(id: string): any | undefined {
    return this.elements.find((el) => el.id === id);
  }

  /**
   * Añade un rectángulo al dibujo.
   */
  addRect(
    topX: number,
    topY: number,
    width: number,
    height: number,
    options?: ElementOptions
  ): string {
    const id = options?.id || randomId();
    const el = {
      id,
      type: 'rectangle',
      x: topX,
      y: topY,
      width,
      height,
      angle: 0,
      strokeColor: options?.strokeColor || this.appState.currentItemStrokeColor || '#1e1e1e',
      backgroundColor: options?.backgroundColor || this.appState.currentItemBackgroundColor || 'transparent',
      fillStyle: options?.fillStyle || this.appState.currentItemFillStyle || 'solid',
      strokeWidth: options?.strokeWidth ?? this.appState.currentItemStrokeWidth ?? 2,
      roughness: options?.roughness ?? this.appState.currentItemRoughness ?? 1,
      opacity: options?.opacity ?? 100,
      groupIds: options?.groupIds || [],
      roundness: options?.roundness ?? { type: 3 },
      seed: Math.floor(Math.random() * 100000),
      version: 1,
      versionNonce: 1,
      isDeleted: false,
      link: options?.link || null,
      updated: Date.now(),
    };
    this.elements.push(el);
    return id;
  }

  /**
   * Añade un rombo (diamond) al dibujo.
   */
  addDiamond(
    topX: number,
    topY: number,
    width: number,
    height: number,
    options?: ElementOptions
  ): string {
    const id = options?.id || randomId();
    const el = {
      id,
      type: 'diamond',
      x: topX,
      y: topY,
      width,
      height,
      angle: 0,
      strokeColor: options?.strokeColor || this.appState.currentItemStrokeColor || '#1e1e1e',
      backgroundColor: options?.backgroundColor || this.appState.currentItemBackgroundColor || 'transparent',
      fillStyle: options?.fillStyle || this.appState.currentItemFillStyle || 'solid',
      strokeWidth: options?.strokeWidth ?? 2,
      roughness: options?.roughness ?? 1,
      opacity: options?.opacity ?? 100,
      groupIds: options?.groupIds || [],
      roundness: null,
      seed: Math.floor(Math.random() * 100000),
      version: 1,
      versionNonce: 1,
      isDeleted: false,
      link: options?.link || null,
      updated: Date.now(),
    };
    this.elements.push(el);
    return id;
  }

  /**
   * Añade una elipse al dibujo.
   */
  addEllipse(
    topX: number,
    topY: number,
    width: number,
    height: number,
    options?: ElementOptions
  ): string {
    const id = options?.id || randomId();
    const el = {
      id,
      type: 'ellipse',
      x: topX,
      y: topY,
      width,
      height,
      angle: 0,
      strokeColor: options?.strokeColor || this.appState.currentItemStrokeColor || '#1e1e1e',
      backgroundColor: options?.backgroundColor || this.appState.currentItemBackgroundColor || 'transparent',
      fillStyle: options?.fillStyle || this.appState.currentItemFillStyle || 'solid',
      strokeWidth: options?.strokeWidth ?? 2,
      roughness: options?.roughness ?? 1,
      opacity: options?.opacity ?? 100,
      groupIds: options?.groupIds || [],
      roundness: null,
      seed: Math.floor(Math.random() * 100000),
      version: 1,
      versionNonce: 1,
      isDeleted: false,
      link: options?.link || null,
      updated: Date.now(),
    };
    this.elements.push(el);
    return id;
  }

  /**
   * Añade un elemento de texto al lienzo. Soporta wikilinks `[[Destino]]` en el texto.
   */
  addText(
    topX: number,
    topY: number,
    text: string,
    options?: ElementOptions
  ): string {
    const id = options?.id || randomId();
    const fontSize = options?.fontSize || 20;
    const lines = (text || '').split('\n');
    const width = Math.max(...lines.map((l) => l.length * (fontSize * 0.6)), 20);
    const height = lines.length * (fontSize * 1.2);

    const el = {
      id,
      type: 'text',
      x: topX,
      y: topY,
      width,
      height,
      angle: 0,
      strokeColor: options?.strokeColor || this.appState.currentItemStrokeColor || '#1e1e1e',
      backgroundColor: options?.backgroundColor || 'transparent',
      fillStyle: options?.fillStyle || 'solid',
      strokeWidth: options?.strokeWidth ?? 1,
      roughness: options?.roughness ?? 1,
      opacity: options?.opacity ?? 100,
      groupIds: options?.groupIds || [],
      roundness: null,
      seed: Math.floor(Math.random() * 100000),
      version: 1,
      versionNonce: 1,
      isDeleted: false,
      text: text || '',
      fontSize,
      fontFamily: options?.fontFamily || 1,
      textAlign: options?.textAlign || 'left',
      verticalAlign: options?.verticalAlign || 'top',
      baseline: fontSize * 0.8,
      containerId: null,
      originalText: text || '',
      link: options?.link || null,
      updated: Date.now(),
    };
    this.elements.push(el);
    return id;
  }

  /**
   * Conecta dos elementos mediante una flecha direccional.
   */
  connectElements(
    id1: string,
    id2: string,
    options?: { strokeColor?: string; strokeWidth?: number; link?: string }
  ): string {
    const el1 = this.getElement(id1);
    const el2 = this.getElement(id2);
    if (!el1 || !el2) {
      throw new Error(`[ExcalidrawAutomate] Uno o ambos elementos no existen: ${id1}, ${id2}`);
    }

    const startX = el1.x + el1.width;
    const startY = el1.y + el1.height / 2;
    const endX = el2.x;
    const endY = el2.y + el2.height / 2;

    const arrowId = randomId();
    const arrow = {
      id: arrowId,
      type: 'arrow',
      x: startX,
      y: startY,
      width: Math.abs(endX - startX),
      height: Math.abs(endY - startY),
      angle: 0,
      strokeColor: options?.strokeColor || this.appState.currentItemStrokeColor || '#1e1e1e',
      backgroundColor: 'transparent',
      fillStyle: 'solid',
      strokeWidth: options?.strokeWidth ?? 2,
      roughness: 1,
      opacity: 100,
      groupIds: [],
      roundness: { type: 2 },
      seed: Math.floor(Math.random() * 100000),
      version: 1,
      versionNonce: 1,
      isDeleted: false,
      points: [
        [0, 0],
        [endX - startX, endY - startY],
      ],
      startBinding: { elementId: id1, focus: 0, gap: 1 },
      endBinding: { elementId: id2, focus: 0, gap: 1 },
      startArrowhead: null,
      endArrowhead: 'arrow',
      link: options?.link || null,
      updated: Date.now(),
    };

    this.elements.push(arrow);
    return arrowId;
  }

  /**
   * Extrae todos los wikilinks `[[Nota]]` presentes en los textos del dibujo.
   */
  getLinks(): string[] {
    return extractWikilinksFromElements(this.elements);
  }

  /**
   * Genera el contenido serializado del dibujo (en formato Markdown de Obsidian o JSON).
   */
  async create(params?: {
    elements?: any[];
    appState?: any;
    files?: any;
    format?: 'markdown' | 'json';
  }): Promise<string> {
    const scene: ExcalidrawScene = {
      type: 'excalidraw',
      version: 2,
      source: 'https://github.com/zsviczian/obsidian-excalidraw-plugin',
      elements: params?.elements || this.elements,
      appState: { ...this.appState, ...(params?.appState || {}) },
      files: params?.files || this.files,
    };

    if (params?.format === 'json') {
      return JSON.stringify(scene, null, 2);
    }

    return serializeExcalidrawMarkdown(scene);
  }

  /**
   * Crea un nuevo archivo de dibujo en la bóveda de Synapse.
   */
  async createDrawing(
    path: string,
    options?: { onMarkdown?: boolean; elements?: any[] }
  ): Promise<TFile> {
    const useMarkdown = options?.onMarkdown ?? path.endsWith('.md');
    const content = await this.create({
      elements: options?.elements,
      format: useMarkdown ? 'markdown' : 'json',
    });

    if (!this.app) {
      throw new Error('[ExcalidrawAutomate] SynapseApp requerida para createDrawing');
    }
    return await this.app.vault.create(path, content);
  }

  /**
   * Exporta los elementos a una imagen vectorial SVG transparente.
   * Soporta mapas de archivos resueltos (dataUrl o urls) para elementos de tipo 'image'.
   */
  toSVG(elements?: any[], files?: Record<string, any>): string {
    const targetElements = (elements || this.elements).filter((el) => !el.isDeleted);
    if (targetElements.length === 0) {
      return `<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"></svg>`;
    }

    // Calcular bounding box
    let minX = Infinity;
    let minY = Infinity;
    let maxX = -Infinity;
    let maxY = -Infinity;

    for (const el of targetElements) {
      minX = Math.min(minX, el.x);
      minY = Math.min(minY, el.y);
      maxX = Math.max(maxX, el.x + (el.width || 0));
      maxY = Math.max(maxY, el.y + (el.height || 0));
    }

    const padding = 20;
    const x0 = minX - padding;
    const y0 = minY - padding;
    const w = Math.max(maxX - minX + padding * 2, 50);
    const h = Math.max(maxY - minY + padding * 2, 50);

    const svgParts: string[] = [
      `<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="${x0} ${y0} ${w} ${h}" width="100%" height="100%">`,
      `<style>text { font-family: sans-serif; user-select: none; }</style>`,
    ];

    for (const el of targetElements) {
      if (el.type === 'rectangle') {
        svgParts.push(
          `<rect x="${el.x}" y="${el.y}" width="${el.width}" height="${el.height}" rx="8" fill="${el.backgroundColor}" stroke="${el.strokeColor}" stroke-width="${el.strokeWidth}" />`
        );
      } else if (el.type === 'diamond') {
        const cx = el.x + el.width / 2;
        const cy = el.y + el.height / 2;
        const points = `${cx},${el.y} ${el.x + el.width},${cy} ${cx},${el.y + el.height} ${el.x},${cy}`;
        svgParts.push(
          `<polygon points="${points}" fill="${el.backgroundColor}" stroke="${el.strokeColor}" stroke-width="${el.strokeWidth}" />`
        );
      } else if (el.type === 'ellipse') {
        const cx = el.x + el.width / 2;
        const cy = el.y + el.height / 2;
        const rx = el.width / 2;
        const ry = el.height / 2;
        svgParts.push(
          `<ellipse cx="${cx}" cy="${cy}" rx="${rx}" ry="${ry}" fill="${el.backgroundColor}" stroke="${el.strokeColor}" stroke-width="${el.strokeWidth}" />`
        );
      } else if (el.type === 'arrow') {
        const pts = (el.points || []).map(([px, py]: [number, number]) => `${el.x + px},${el.y + py}`).join(' ');
        svgParts.push(
          `<polyline points="${pts}" fill="none" stroke="${el.strokeColor}" stroke-width="${el.strokeWidth}" marker-end="url(#arrow)" />`
        );
      } else if (el.type === 'image') {
        const fileRecord = files?.[el.fileId];
        const dataUrl = fileRecord?.dataURL || fileRecord?.url || '';
        if (dataUrl) {
          svgParts.push(
            `<image href="${dataUrl}" xlink:href="${dataUrl}" x="${el.x}" y="${el.y}" width="${el.width}" height="${el.height}" preserveAspectRatio="none" />`
          );
        } else {
          // Placeholder para imagen mientras carga o si no se encuentra
          svgParts.push(
            `<rect x="${el.x}" y="${el.y}" width="${el.width}" height="${el.height}" fill="#f1f5f9" stroke="#94a3b8" stroke-dasharray="4" rx="4" />`
          );
          svgParts.push(
            `<text x="${el.x + 8}" y="${el.y + 20}" font-size="12" fill="#64748b">🖼 [Imagen ${el.fileId ? el.fileId.slice(0, 8) : ''}]</text>`
          );
        }
      } else if (el.type === 'text') {
        const escaped = (el.text || '')
          .replace(/&/g, '&amp;')
          .replace(/</g, '&lt;')
          .replace(/>/g, '&gt;');
        svgParts.push(
          `<text x="${el.x}" y="${el.y + el.fontSize}" font-size="${el.fontSize}" fill="${el.strokeColor}">${escaped}</text>`
        );
      }
    }

    svgParts.push(`</svg>`);
    return svgParts.join('\n');
  }

  /**
   * Exporta los elementos a un ArrayBuffer con datos binarios simulados o renderizados en PNG.
   */
  async toPNG(elements?: any[]): Promise<ArrayBuffer> {
    const svgStr = this.toSVG(elements);
    // Si estamos en entorno navegador y canvas está disponible, podemos rasterizar
    if (typeof document !== 'undefined' && typeof OffscreenCanvas !== 'undefined') {
      try {
        const svgBlob = new Blob([svgStr], { type: 'image/svg+xml;charset=utf-8' });
        const url = URL.createObjectURL(svgBlob);
        const img = new Image();
        img.src = url;
        await new Promise((res, rej) => {
          img.onload = res;
          img.onerror = rej;
        });

        const canvas = new OffscreenCanvas(img.width || 200, img.height || 200);
        const ctx = canvas.getContext('2d');
        if (ctx) {
          ctx.drawImage(img, 0, 0);
          const blob = await canvas.convertToBlob({ type: 'image/png' });
          URL.revokeObjectURL(url);
          return await blob.arrayBuffer();
        }
      } catch {
        // Fallback en CLI o entorno sin canvas
      }
    }

    // Fallback: Retornar los bytes del SVG codificados
    return new TextEncoder().encode(svgStr).buffer as ArrayBuffer;
  }
}
