import burbujaFmUrl from '../../assets/burbuja_fm.wav';
import sonarOrganicoUrl from '../../assets/sonar_organico.wav';

export type AgentCompletionSound = 'burbuja_fm' | 'sonar_organico' | 'none';

export const AGENT_SOUND_OPTIONS: { id: AgentCompletionSound; label: string }[] = [
  { id: 'burbuja_fm', label: 'Burbuja FM' },
  { id: 'sonar_organico', label: 'Sonar orgánico' },
  { id: 'none', label: 'Sin sonido' },
];

const SOUND_URLS: Record<Exclude<AgentCompletionSound, 'none'>, string> = {
  burbuja_fm: burbujaFmUrl,
  sonar_organico: sonarOrganicoUrl,
};

let currentSound: AgentCompletionSound = 'burbuja_fm';
let audioCtx: AudioContext | null = null;
const bufferCache = new Map<string, AudioBuffer>();

export function setAgentCompletionSound(sound: AgentCompletionSound): void {
  currentSound = sound;
}

export function getAgentCompletionSound(): AgentCompletionSound {
  return currentSound;
}

export async function playAgentCompletionSound(override?: AgentCompletionSound): Promise<void> {
  const chosen = override ?? currentSound;
  if (chosen === 'none') return;
  const url = SOUND_URLS[chosen];
  if (!url || typeof window === 'undefined') return;

  try {
    const AudioCtx = window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
    if (AudioCtx) {
      audioCtx ??= new AudioCtx();
      if (audioCtx.state === 'suspended') {
        await audioCtx.resume();
      }
      let buffer = bufferCache.get(url);
      if (!buffer) {
        const res = await fetch(url);
        const bytes = await res.arrayBuffer();
        buffer = await audioCtx.decodeAudioData(bytes);
        bufferCache.set(url, buffer);
      }
      const source = audioCtx.createBufferSource();
      source.buffer = buffer;
      source.connect(audioCtx.destination);
      source.start(0);
      return;
    }
    const audio = new Audio(url);
    await audio.play();
  } catch {
    // Entornos sin dispositivo de audio (p. ej. pruebas headless) o autoplay bloqueado
  }
}
