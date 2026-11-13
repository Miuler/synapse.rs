import type { ChatItem } from './types';

/** Conversación anterior en texto, para un agente que no puede retomarla por sí mismo. */
export function replayText(items: ChatItem[]): string {
  const lines: string[] = [];
  // Tras un `/compact`, el agente solo conservaba su resumen: se manda lo posterior a la última marca
  // (empezando por la respuesta del agente al compactar, que es ese resumen)
  const lastCompact = items.findLastIndex((i) => i.type === 'compact');
  for (const i of items.slice(lastCompact + 1)) {
    if (i.type === 'user' && /^\/compact\b/.test(i.text.trim())) continue;
    if (i.type === 'user') lines.push(`Usuario: ${i.text}`);
    else if (i.type === 'agent') lines.push(`Agente: ${i.text}`);
    else if (i.type === 'tool') lines.push(`(herramienta: ${i.title})`);
  }
  const body = lines.join('\n\n').slice(-20000);
  return `Contexto: esta conversación continúa una anterior. Resumen de lo dicho hasta ahora:\n\n${body}\n\n— Fin del contexto anterior —`;
}
