const MAX_QUERY = 120;

/** Texto de la notificación de fin de turno: qué agente y con qué consulta empezó. */
export function completionNotice(agent: string, handle: string, query: string, failed: boolean): { title: string; body: string } {
  const chars = [...query.replace(/\s+/g, ' ').trim()];
  const short = chars.length > MAX_QUERY ? `${chars.slice(0, MAX_QUERY).join('').trimEnd()}…` : chars.join('');
  return {
    title: `${agent} · ${handle}`,
    body: `${failed ? 'Falló' : 'Terminó'} la consulta: "${short}"`,
  };
}
