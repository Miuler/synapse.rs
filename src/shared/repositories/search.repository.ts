import { invokeTauri, isTauriEnvironment } from './tauri';

export type DocumentKind = 'markdown' | 'mermaid' | 'code' | 'excalidraw';

export interface HighlightSegment {
  text: string;
  highlighted: boolean;
}

export interface FullTextHit {
  note_path: string;
  title: HighlightSegment[];
  snippet: HighlightSegment[];
  matched_terms: string[];
  kind: DocumentKind;
  score: number;
}

export interface FullTextIndexStatus {
  indexed_docs: number;
  pending: number;
  is_indexing: boolean;
  in_memory_fallback: boolean;
  last_error?: string | null;
}

export interface FullTextSearchResponse {
  hits: FullTextHit[];
  total_hits: number;
  elapsed_ms: number;
  status: FullTextIndexStatus;
}

/**
 * Contrato del repositorio para búsqueda full-text sobre el contenido de notas y diagramas.
 */
export interface SearchRepository {
  isConnected(): boolean;
  search(query: string, limit?: number): Promise<FullTextSearchResponse>;
  getStatus(): Promise<FullTextIndexStatus | null>;
  rebuildIndex(): Promise<void>;
}

export class TauriSearchRepository implements SearchRepository {
  isConnected(): boolean {
    return isTauriEnvironment();
  }

  async search(query: string, limit?: number): Promise<FullTextSearchResponse> {
    if (!this.isConnected()) {
      return {
        hits: [],
        total_hits: 0,
        elapsed_ms: 0,
        status: {
          indexed_docs: 0,
          pending: 0,
          is_indexing: false,
          in_memory_fallback: false,
        },
      };
    }

    try {
      return await invokeTauri<FullTextSearchResponse>('full_text_search', {
        query,
        limit: limit ?? null,
      });
    } catch (error) {
      console.warn('Error en TauriSearchRepository al buscar full-text:', error);
      return {
        hits: [],
        total_hits: 0,
        elapsed_ms: 0,
        status: {
          indexed_docs: 0,
          pending: 0,
          is_indexing: false,
          in_memory_fallback: false,
          last_error: String(error),
        },
      };
    }
  }

  async getStatus(): Promise<FullTextIndexStatus | null> {
    if (!this.isConnected()) return null;
    try {
      return await invokeTauri<FullTextIndexStatus>('get_full_text_index_status');
    } catch (error) {
      console.warn('Error en TauriSearchRepository al obtener status:', error);
      return null;
    }
  }

  async rebuildIndex(): Promise<void> {
    if (!this.isConnected()) return;
    try {
      await invokeTauri('rebuild_full_text_index');
    } catch (error) {
      console.error('Error en TauriSearchRepository al reconstruir índice:', error);
    }
  }
}

export const searchRepository: SearchRepository = new TauriSearchRepository();
