/** Agentes locales cuyo id no es el del registro ACP. */
const ALIASES: Record<string, string> = {
  antigravity: 'antigravity-acp',
  'github-copilot': 'github-copilot-cli',
};

/** Iconos a color; los demás son monocromos y se pintan con el color del texto. */
const COLORED = new Set(['claude-acp', 'antigravity-acp']);

/** Nombre del SVG (sin extensión) que le toca a un agente. */
export function agentIconId(agentId: string): string {
  return Object.hasOwn(ALIASES, agentId) ? ALIASES[agentId] : agentId;
}

export function isColoredIcon(iconId: string): boolean {
  return COLORED.has(iconId);
}
