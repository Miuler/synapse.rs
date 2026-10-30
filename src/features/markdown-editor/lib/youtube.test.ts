import { describe, expect, it } from 'bun:test';
import { extractYouTubeId, extractYouTubeTokens, getYouTubeThumbnailUrl } from './youtube';

describe('youtube lib', () => {
  it('extracts the id from the supported url formats', () => {
    const id = 'dQw4w9WgXcQ';
    expect(extractYouTubeId(`https://www.youtube.com/watch?v=${id}&t=10s`)).toBe(id);
    expect(extractYouTubeId(`https://youtu.be/${id}?si=abc`)).toBe(id);
    expect(extractYouTubeId(`https://www.youtube.com/shorts/${id}`)).toBe(id);
    expect(extractYouTubeId(`https://www.youtube.com/embed/${id}`)).toBe(id);
    expect(extractYouTubeId(`https://m.youtube.com/watch?v=${id}`)).toBe(id);
  });

  it('rejects non-video urls', () => {
    expect(extractYouTubeId('https://www.youtube.com/@canal')).toBeNull();
    expect(extractYouTubeId('https://example.com/watch?v=dQw4w9WgXcQ')).toBeNull();
    expect(extractYouTubeId('no es url')).toBeNull();
  });

  it('finds tokens in markdown links and bare urls with absolute positions', () => {
    const line = 'ver [video](https://youtu.be/dQw4w9WgXcQ) y https://youtu.be/abcdefghijk.';
    const tokens = extractYouTubeTokens(line, 100);
    expect(tokens.map((t) => t.id)).toEqual(['dQw4w9WgXcQ', 'abcdefghijk']);
    expect(tokens[0].from).toBe(100 + line.indexOf('https'));
    expect(tokens[1].url.endsWith('.')).toBe(false);
  });

  it('builds the thumbnail url', () => {
    expect(getYouTubeThumbnailUrl('dQw4w9WgXcQ')).toBe('https://img.youtube.com/vi/dQw4w9WgXcQ/hqdefault.jpg');
  });
});
