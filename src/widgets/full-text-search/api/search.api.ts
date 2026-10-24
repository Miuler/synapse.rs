import { invokeTauri } from '@shared/api';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface FullTextHitDto {
  note_path: string;
  title: string;
  score: number;
  snippet?: string;
  matched_field?: string;
}

export interface FullTextIndexStatusDto {
  state: 'idle' | 'indexing' | 'ready' | 'error';
  indexed_count: number;
  total_count: number;
  current_file?: string;
  error?: string;
}

export interface FullTextSearchResponseDto {
  hits: FullTextHitDto[];
  total_hits: number;
  elapsed_ms: number;
  status: FullTextIndexStatusDto;
}

export const fullTextSearchApi = {
  async search(query: string, limit?: number): Promise<FullTextSearchResponseDto> {
    return await invokeTauri<FullTextSearchResponseDto>('full_text_search', { query, limit });
  },

  async getIndexStatus(): Promise<FullTextIndexStatusDto> {
    return await invokeTauri<FullTextIndexStatusDto>('get_full_text_index_status');
  },

  async rebuildIndex(): Promise<void> {
    await invokeTauri('rebuild_full_text_index');
  },

  async onIndexingStatusChange(callback: (status: FullTextIndexStatusDto) => void): Promise<UnlistenFn> {
    return await listen<FullTextIndexStatusDto>('vault:indexing-status', (event) => {
      callback(event.payload);
    });
  },
};
