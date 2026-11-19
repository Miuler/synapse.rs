// Agente ACP guionado SOLO para generar las capturas de la documentación (e2e/docs-shots.test.ts).
// Responde siempre con la misma conversación: plan, lectura vía MCP, edición con diff y resumen.
import { createInterface } from 'node:readline';

const send = (m: unknown) => process.stdout.write(JSON.stringify(m) + '\n');
let current = 'ses-docs';
let n = 0;
const update = (u: unknown) => send({ jsonrpc: '2.0', method: 'session/update', params: { sessionId: current, update: u } });

for await (const line of createInterface({ input: process.stdin })) {
  if (!line.trim()) continue;
  const msg = JSON.parse(line);
  if (msg.method === 'initialize') {
    send({ jsonrpc: '2.0', id: msg.id, result: { protocolVersion: 1, agentCapabilities: { promptCapabilities: { embeddedContext: true } } } });
  } else if (msg.method === 'session/new') {
    current = `ses-docs-${++n}`;
    send({ jsonrpc: '2.0', id: msg.id, result: { sessionId: current, modes: { currentModeId: 'ask', availableModes: [{ id: 'ask', name: 'Preguntar' }, { id: 'code', name: 'Programar' }] } } });
    update({ sessionUpdate: 'available_commands_update', availableCommands: [{ name: 'adr', description: 'Redacta un ADR' }, { name: 'compact', description: 'Compacta el contexto' }] });
  } else if (msg.method === 'session/prompt') {
    current = msg.params.sessionId;
    update({ sessionUpdate: 'usage_update', used: 46_000, size: 200_000 });
    update({ sessionUpdate: 'plan', entries: [
      { content: 'Leer el diagrama de contenedores (sistema.d2)', priority: 'high', status: 'completed' },
      { content: 'Detectar acoplamientos entre contenedores', priority: 'high', status: 'completed' },
      { content: 'Registrar la decisión en un ADR', priority: 'medium', status: 'in_progress' },
    ] });
    update({ sessionUpdate: 'tool_call', toolCallId: 't1', title: 'read_note sistema.d2', kind: 'read', status: 'completed', rawInput: { relative_path: 'sistema.d2', outline_only: false } });
    update({ sessionUpdate: 'tool_call', toolCallId: 't2', title: 'render_diagram sistema.d2', kind: 'read', status: 'completed', rawInput: { relative_path: 'sistema.d2' } });
    update({ sessionUpdate: 'tool_call', toolCallId: 't3', title: 'patch_note sistema.d2', kind: 'edit', status: 'completed',
      locations: [{ path: '/tmp/sistema.d2' }],
      content: [{ type: 'diff', path: '/tmp/sistema.d2', oldText: 'api -> broker: publica eventos', newText: 'api -> broker: publica eventos\nbroker -> notificaciones: consume\nnotificaciones -> correo: envía' }],
      rawInput: { relative_path: 'sistema.d2', expected_hash: 'sha256:9f2c…' } });
    update({ sessionUpdate: 'agent_message_chunk', content: { type: 'text', text:
      'Listo. Conecté **Notificaciones** al broker en el diagrama:\n\n' +
      '- `broker -> notificaciones`: consume los eventos de pedidos\n- `notificaciones -> correo`: envía los avisos\n\n' +
      'Usé `patch_note` con `expected_hash`: si alguien editaba el archivo a la vez, la escritura se rechazaba. ' +
      'Comprobé el resultado dibujado con `render_diagram` y dejé la decisión en el ADR-0002.' } });
    send({ jsonrpc: '2.0', id: msg.id, result: { stopReason: 'end_turn' } });
  }
}
