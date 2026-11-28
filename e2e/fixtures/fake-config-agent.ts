// Agente ACP falso para el asistente de configuración. Ante cualquier pedido usa las herramientas
// `config_` del MCP de Synapse por el puente stdio (con el token que Synapse le deja en el entorno):
// mira la configuración abierta, prueba una consulta mala, la corrige y cambia el borrador.
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';

const send = (msg: unknown) => process.stdout.write(JSON.stringify(msg) + '\n');
type Server = { command?: string; args: string[]; env: { name: string; value: string }[] };
let server: Server | undefined;

const GOOD = "SELECT [System.Id] FROM WorkItems WHERE [System.WorkItemType] = 'Bug'";

/** Abre el puente, deja usar sus herramientas y lo cierra. */
async function withTools<T>(use: (tool: (name: string, args?: unknown) => Promise<{ error: boolean; text: string }>) => Promise<T>) {
  const env = { ...process.env, ...Object.fromEntries(server!.env.map((e) => [e.name, e.value])) };
  const child = spawn(server!.command!, server!.args, { stdio: ['pipe', 'pipe', 'ignore'], env });
  const replies = new Map<number, (message: any) => void>();
  createInterface({ input: child.stdout! }).on('line', (line) => {
    const message = JSON.parse(line);
    replies.get(message.id)?.(message);
  });
  let next = 0;
  const call = (method: string, params: unknown) =>
    new Promise<any>((resolve) => {
      replies.set(++next, resolve);
      child.stdin!.write(JSON.stringify({ jsonrpc: '2.0', id: next, method, params }) + '\n');
    });
  try {
    await call('initialize', { protocolVersion: '2024-11-05', capabilities: {}, clientInfo: { name: 'falso', version: '1' } });
    return await use(async (name, args = {}) => {
      const { result } = await call('tools/call', { name, arguments: args });
      return { error: !!result.isError, text: result.content.map((c: any) => c.text).join(' ') };
    });
  } finally {
    child.kill();
  }
}

/** Lo que cuenta de su turno. */
const assist = () =>
  withTools(async (tool) => {
    const context = await tool('config_get_context');
    if (context.error) return `sin configuración: ${context.text}`;
    const { connection, scope } = JSON.parse(context.text);
    const bad = await tool('config_test_query', { query: 'MAL' });
    const good = await tool('config_test_query', { query: GOOD });
    const changed = await tool('config_update_draft', { query: GOOD, states: { done: ['Closed', 'Done'] } });
    return [
      `conexión:${connection?.name} proyecto:${scope.split('/').pop()}`,
      `mala:${bad.error && bad.text.includes('TF51005') ? 'rechazada' : bad.text}`,
      `buena:${good.error ? good.text : JSON.parse(good.text).count}`,
      `cambio:${changed.error ? changed.text : 'hecho'}`,
    ].join(' | ');
  });

createInterface({ input: process.stdin }).on('line', async (line) => {
  const msg = JSON.parse(line);
  if (msg.method === 'initialize') {
    send({ jsonrpc: '2.0', id: msg.id, result: { protocolVersion: 1, agentCapabilities: {} } });
  } else if (msg.method === 'session/new') {
    server = (msg.params.mcpServers as Server[]).find((s) => s.command);
    send({ jsonrpc: '2.0', id: msg.id, result: { sessionId: 'asistente-e2e' } });
  } else if (msg.method === 'session/prompt') {
    const text = await assist().catch((error) => `fallo: ${error}`);
    const update = { sessionUpdate: 'agent_message_chunk', content: { type: 'text', text } };
    send({ jsonrpc: '2.0', method: 'session/update', params: { sessionId: 'asistente-e2e', update } });
    send({ jsonrpc: '2.0', id: msg.id, result: { stopReason: 'end_turn' } });
  }
});
