export interface YouTubeToken {
  id: string;
  url: string;
  from: number;
  to: number;
}

const YOUTUBE_URL_REGEX = /https?:\/\/(?:www\.|m\.)?(?:youtube\.com|youtube-nocookie\.com|youtu\.be)\/[^\s)<>\]"']+/gi;
const VIDEO_ID_REGEX = /^[A-Za-z0-9_-]{11}$/;

export function extractYouTubeId(url: string): string | null {
  let parsed: URL;
  try {
    parsed = new URL(url.trim());
  } catch {
    return null;
  }

  const host = parsed.hostname.replace(/^(?:www\.|m\.)/, '').toLowerCase();
  let id: string | null = null;

  if (host === 'youtu.be') {
    id = parsed.pathname.split('/')[1] ?? null;
  } else if (host === 'youtube.com' || host === 'youtube-nocookie.com') {
    const [, kind, value] = parsed.pathname.split('/');
    if (kind === 'watch') {
      id = parsed.searchParams.get('v');
    } else if (kind === 'shorts' || kind === 'embed' || kind === 'live') {
      id = value ?? null;
    }
  }

  return id && VIDEO_ID_REGEX.test(id) ? id : null;
}

export function getYouTubeThumbnailUrl(id: string): string {
  return `https://img.youtube.com/vi/${id}/hqdefault.jpg`;
}

/** Devuelve los links de YouTube de una línea, con posiciones absolutas en el documento. */
export function extractYouTubeTokens(line: string, lineFrom = 0): YouTubeToken[] {
  const tokens: YouTubeToken[] = [];
  for (const match of line.matchAll(YOUTUBE_URL_REGEX)) {
    const url = match[0].replace(/[.,;:!?]+$/, '');
    const id = extractYouTubeId(url);
    if (!id) continue;
    const from = lineFrom + (match.index ?? 0);
    tokens.push({ id, url, from, to: from + url.length });
  }
  return tokens;
}
