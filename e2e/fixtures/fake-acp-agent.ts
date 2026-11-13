// Agente ACP falso para las e2e. Al abrir la sesión anuncia comandos; ante un prompt publica un plan,
// una herramienta con diff, responde con lo que recibió (bloques y MCP) y pide permiso; termina el turno
// al recibir la respuesta.
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';

const send = (msg: unknown) => process.stdout.write(JSON.stringify(msg) + '\n');
/** Un mismo proceso atiende varias conversaciones: cada `session/new` abre una sesión distinta. */
let opened = 0;
let current = 'ses-e2e';
const update = (u: unknown) => send({ jsonrpc: '2.0', method: 'session/update', params: { sessionId: current, update: u } });
let promptId: unknown = null;
let mcp: string[] = [];
let mcpTools = 'sin-probar';
/** Resultado de pedir un diagrama como imagen al MCP (marca de read_note + render_diagram). */
let diagramInfo = 'sin-probar';

/** Lanza el servidor MCP por stdio que pasó Synapse y cuenta sus herramientas (tools/list). */
async function countMcpTools(server: { command: string; args: string[] }): Promise<string> {
  const child = spawn(server.command, server.args, { stdio: ['pipe', 'pipe', 'ignore'] });
  const lines = createInterface({ input: child.stdout! });
  const replies = new Map<number, (v: unknown) => void>();
  lines.on('line', (l) => {
    try {
      const m = JSON.parse(l);
      replies.get(m.id)?.(m);
    } catch {
      /* no es JSON */
    }
  });
  const call = (id: number, method: string, params = {}) =>
    new Promise<any>((resolve) => {
      replies.set(id, resolve);
      child.stdin!.write(JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n');
    });
  try {
    await call(1, 'initialize', { protocolVersion: '2024-11-05', capabilities: {}, clientInfo: { name: 'falso', version: '1' } });
    const list = await Promise.race([call(2, 'tools/list'), new Promise((r) => setTimeout(() => r(null), 10_000))]);
    if (!list) return 'timeout';
    // Un diagrama: read_note lo marca y render_diagram lo devuelve como imagen dibujada por la ventana
    const read = await call(3, 'tools/call', { name: 'read_note', arguments: { relative_path: 'flujo.mmd' } });
    const marker = JSON.parse(read.result?.content?.[0]?.text ?? '{}').diagram;
    const img = await Promise.race([
      call(4, 'tools/call', { name: 'render_diagram', arguments: { relative_path: 'flujo.mmd' } }),
      new Promise((r) => setTimeout(() => r(null), 30_000)),
    ]);
    const image = (img as any)?.result?.content?.find((c: any) => c.type === 'image');
    const err = (img as any)?.result?.isError ? (img as any).result.content?.[0]?.text : '';
    diagramInfo = `diagram:${marker?.kind ?? 'no'} img:${image ? `${image.mimeType}:${image.data.length > 100 ? 'ok' : 'vacia'}` : `no(${err || 'timeout'})`}`;
    return String((list as any).result?.tools?.length ?? 0);
  } finally {
    child.kill();
  }
}

for await (const line of createInterface({ input: process.stdin })) {
  if (!line.trim()) continue;
  const msg = JSON.parse(line);
  if (msg.method === 'initialize') {
    send({
      jsonrpc: '2.0',
      id: msg.id,
      result: { protocolVersion: 1, agentCapabilities: { promptCapabilities: { embeddedContext: true } } },
    });
  } else if (msg.method === 'session/new') {
    current = `ses-e2e-${++opened}`;
    const servers = msg.params.mcpServers ?? [];
    mcp = servers.map((s: { name: string; command?: string }) => `${s.name}:${s.command ? 'stdio' : 'http'}`);
    const stdio = servers.find((s: { command?: string }) => s.command);
    if (stdio) mcpTools = await countMcpTools(stdio);
    send({
      jsonrpc: '2.0',
      id: msg.id,
      result: {
        sessionId: current,
        modes: { currentModeId: 'ask', availableModes: [{ id: 'ask', name: 'Preguntar' }, { id: 'code', name: 'Programar' }] },
      },
    });
    update({ sessionUpdate: 'available_commands_update', availableCommands: [{ name: 'resumir', description: 'Resume la nota' }] });
  } else if (msg.method === 'session/set_mode') {
    send({ jsonrpc: '2.0', id: msg.id, result: {} });
  } else if (msg.method === 'session/prompt') {
    promptId = msg.id;
    current = msg.params.sessionId;
    const instr = msg.params.prompt[0]?.text?.includes('synapse_instance') ? 'instr:si' : 'instr:no';
    const blocks = msg.params.prompt.slice(instr === 'instr:si' ? 1 : 0).map((b: { type: string; uri?: string; resource?: { uri: string } }) =>
      b.type === 'resource_link' ? `link:${b.uri!.split('/').pop()}` : b.type === 'resource' ? `sel:${b.resource!.uri.split('/').pop()}` : b.type,
    );
    update({ sessionUpdate: 'plan', entries: [{ content: 'Leer la nota', priority: 'high', status: 'completed' }, { content: 'Editar la nota', priority: 'high', status: 'in_progress' }] });
    update({
      sessionUpdate: 'tool_call',
      toolCallId: 't1',
      title: 'Editar nota.md',
      kind: 'edit',
      status: 'completed',
      locations: [{ path: '/tmp/nota.md' }],
      content: [{ type: 'diff', path: '/tmp/nota.md', oldText: 'viejo', newText: 'nuevo' }],
      rawInput: { relative_path: 'nota.md', mode: 'full' },
    });
    update({ sessionUpdate: 'agent_message_chunk', content: { type: 'text', text: `Hola desde el agente [${blocks.join(',')}] [mcp ${mcp.join(',')} tools:${mcpTools}] [${instr}] [${diagramInfo}]\n\n**negrita**` } });
    send({
      jsonrpc: '2.0',
      id: 'perm-1',
      method: 'session/request_permission',
      params: {
        sessionId: current,
        toolCall: { toolCallId: 't1', title: 'Editar nota.md' },
        options: [
          { optionId: 'allow', name: 'Permitir', kind: 'allow_once' },
          { optionId: 'reject', name: 'Rechazar', kind: 'reject_once' },
        ],
      },
    });
  } else if (msg.id === 'perm-1') {
    const chosen = msg.result?.outcome?.optionId ?? 'cancelado';
    update({ sessionUpdate: 'agent_message_chunk', content: { type: 'text', text: ` (permiso: ${chosen})` } });
    send({ jsonrpc: '2.0', id: promptId, result: { stopReason: 'end_turn' } });
  }
}
