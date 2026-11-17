// Conversaciones entre bóvedas de Synapse (otras ventanas, por el canal `/peer`): a quién se le
// escribe, el permiso de cada bóveda, en qué conversación entra un mensaje, cómo vuelve la
// respuesta y qué se permite solo en una conversación que abrió otra bóveda.
import { askConfirm } from '@shared/lib/confirm';
import { agentApi } from '../api/agent.api';
import { Conversation, MAX_AGENT_HOPS, type AgentStore, type Sender } from './agent-store.svelte';
import { load, save } from './storage';
import type { ConversationSummary } from './types';

/** Espera de un permiso no de lectura en una conversación entrante antes de rechazarlo. */
export const INCOMING_PERMISSION_TIMEOUT = 120_000;

/** Bóveda remota y conversación a la que vuelve una respuesta. */
export interface RemoteRef {
  vault: string;
  path: string;
  convId: string;
}

/** Origen de un pedido de otra instancia. */
export interface PeerFrom extends Sender {
  vault: string;
  path: string;
  convId: string;
}

/** Pedido que llega de otra instancia de Synapse por su canal `/peer`. */
export interface PeerRequest {
  action: 'ping' | 'list' | 'send' | 'deliver';
  from: PeerFrom;
  to: number | string | null;
  fresh?: boolean;
  message: string;
  expectsReply?: boolean;
  hops: number;
}

/** Lado de esta ventana en las conversaciones con otras bóvedas. */
export class PeerLink {
  readonly #store: AgentStore;
  #identity: { vault: string; path: string } | null = null;

  constructor(store: AgentStore) {
    this.#store = store;
  }

  /** Bóvedas a las que esta bóveda ya respondió «Permitir» o «No permitir» (Ajustes). */
  decisions(): { path: string; vault: string; allowed: boolean }[] {
    const cwd = this.#store.cwd;
    if (!cwd) return [];
    const saved = JSON.parse(load(`synapse.agent.accept.${cwd}`) ?? '{}') as Record<string, boolean>;
    return Object.entries(saved)
      .map(([path, allowed]) => ({ path, vault: path.split(/[\\/]/).filter(Boolean).pop() ?? path, allowed }))
      .sort((a, b) => a.vault.localeCompare(b.vault));
  }

  /** Olvida la decisión sobre `path`: la próxima vez que escriba se vuelve a preguntar. */
  forget(path: string) {
    const cwd = this.#store.cwd;
    if (!cwd) return;
    const key = `synapse.agent.accept.${cwd}`;
    const saved = JSON.parse(load(key) ?? '{}') as Record<string, boolean>;
    delete saved[path];
    save(key, Object.keys(saved).length ? JSON.stringify(saved) : null);
  }

  async me() {
    this.#identity ??= await agentApi.peerIdentity().catch(() => null);
    return this.#identity;
  }
  /** Bóvedas de Synapse abiertas en otras ventanas, con sus conversaciones (menciones `@`). */
  async openVaults(): Promise<{ vault: string; path: string; conversations: ConversationSummary[] }[]> {
    const me = await this.me();
    const peers = await agentApi.peerList({ action: 'list' }).catch(() => []);
    return peers
      .filter((p) => !p.error && p.path !== me?.path)
      .map((p) => ({ vault: p.vault, path: p.path, conversations: Array.isArray(p.result) ? (p.result as ConversationSummary[]) : [] }));
  }
  /** `to` apunta a otra bóveda abierta: su nombre, con `#n` o `#nueva` opcional. */
  async remoteTarget(to: string): Promise<{ vault: string; number: number | null; fresh: boolean } | null> {
    const m = /^(.*?)#(\d+|nueva)?$/i.exec(to) ?? [to, to, undefined];
    const vault = (m[1] ?? '').trim();
    if (!vault) return null;
    const me = await this.me();
    if (me && vault.toLowerCase() === me.vault.toLowerCase()) return null;
    const peers = await agentApi.peerList({ action: 'ping' }).catch(() => []);
    const peer = peers.find((p) => p.vault.toLowerCase() === vault.toLowerCase() || p.path === vault);
    if (!peer) {
      // Sin «#» puede ser el título de una conversación local; con «#» es una bóveda que no está abierta
      if (to.includes('#')) throw new Error(`La bóveda «${vault}» no está abierta en otra ventana de Synapse.`);
      return null;
    }
    const n = m[2];
    return { vault: peer.path, number: n && /^\d+$/.test(n) ? Number(n) : null, fresh: /^nueva$/i.test(n ?? '') };
  }
  async sendRemote(
    caller: Conversation,
    target: { vault: string; number: number | null; fresh: boolean },
    message: string,
    expectsReply: boolean,
    hops: number,
  ) {
    const me = await this.me();
    if (!me) throw new Error('Esta ventana no tiene una bóveda abierta.');
    const result = (await agentApi.peerSend(target.vault, {
      action: 'send',
      from: { ...this.#store.senderOf(caller), vault: me.vault, path: me.path, convId: caller.id },
      to: target.number,
      fresh: target.fresh,
      message,
      expectsReply,
      hops,
    })) as { status: string; to: string };
    if (expectsReply && !caller.waitingFor.includes(result.to)) caller.waitingFor = [...caller.waitingFor, result.to];
    return {
      ...result,
      note:
        `Enviado a la bóveda de otra ventana (${result.to}). ` +
        (expectsReply ? 'Su respuesta te llegará como un mensaje nuevo en esta conversación; no la esperes.' : ''),
    };
  }
  /** Devuelve la respuesta a la conversación de otra bóveda que preguntó. */
  async replyRemote(remote: RemoteRef, from: Conversation, text: string, hops: number) {
    const me = await this.me();
    try {
      await agentApi.peerSend(remote.path, {
        action: 'deliver',
        to: remote.convId,
        from: { ...this.#store.senderOf(from), vault: me?.vault ?? '', path: me?.path ?? '' },
        message: text,
        hops,
      });
    } catch (e) {
      from.error(`No se pudo devolver la respuesta a «${remote.vault}»: ${e instanceof Error ? e.message : String(e)}`);
    }
  }
  /** Pedidos de otra instancia: listar, recibir un mensaje (en una conversación entrante) o una respuesta. */
  async handlePeer(req: PeerRequest): Promise<unknown> {
    if (!req || req.action === 'ping') return [];
    if (req.action === 'list') return this.#store.localSummaries(null);
    const cwd = this.#store.cwd;
    if (!cwd) throw new Error('Esa ventana de Synapse no tiene una bóveda abierta.');
    const from = req.from;
    if (req.hops > MAX_AGENT_HOPS) throw new Error(`Límite de ${MAX_AGENT_HOPS} mensajes seguidos entre agentes.`);
    const sender: Sender = { handle: `${from.vault}${from.handle}`, agent: from.agent, title: from.title, vault: from.vault };
    if (req.action === 'deliver') {
      const c = this.#store.byId(req.to as string);
      if (!c) throw new Error('La conversación que preguntó ya no está abierta.');
      c.waitingFor = c.waitingFor.filter((h) => !h.startsWith(from.vault));
      c.receive({ from: sender, text: req.message, hops: req.hops, replyTo: null });
      return { status: 'delivered' };
    }
    if (req.action !== 'send') throw new Error(`Pedido desconocido: ${req.action}`);
    if (!(await this.#acceptsFrom(cwd, from))) throw new Error(`La bóveda «${(await this.me())?.vault}» no aceptó la conversación.`);
    const c = await this.#incomingConversation(cwd, req);
    const me = await this.me();
    const status = c.receive({
      from: sender,
      text: req.message,
      hops: req.hops,
      replyTo: req.expectsReply ? { remote: { vault: from.vault, path: from.path, convId: from.convId } } : null,
    });
    return { status, to: `${me?.vault ?? ''}${c.handle}` };
  }
  /**
   * Permisos en una conversación abierta por otra bóveda, donde quizá nadie está mirando: las
   * lecturas se permiten solas; lo demás espera al usuario y, sin respuesta, se rechaza.
   */
  incomingPermission(c: Conversation, itemId: number, e: { toolCallId?: string | null; options: { id: string; kind: string }[] }) {
    const tool = c.items.find((i) => i.type === 'tool' && i.toolCallId === e.toolCallId);
    const kind = tool && tool.type === 'tool' ? (tool.kind ?? '') : '';
    const allow = e.options.find((o) => o.kind === 'allow_once') ?? e.options.find((o) => o.kind.startsWith('allow'));
    const reject = e.options.find((o) => o.kind === 'reject_once') ?? e.options.find((o) => o.kind.startsWith('reject'));
    if (['read', 'search', 'fetch', 'think'].includes(kind) && allow) {
      void c.answer(itemId, allow.id);
      return;
    }
    setTimeout(() => {
      const item = c.items.find((i) => i.id === itemId);
      if (item && item.type === 'permission' && item.answered === null) {
        c.push({ type: 'error', text: 'Nadie respondió el permiso en 2 minutos: se rechazó.' });
        void c.answer(itemId, reject?.id ?? null);
      }
    }, INCOMING_PERMISSION_TIMEOUT);
  }
  /** Permiso de esta bóveda para conversar con `from` (se pregunta la primera vez y se recuerda). */
  async #acceptsFrom(cwd: string, from: PeerFrom): Promise<boolean> {
    const key = `synapse.agent.accept.${cwd}`;
    const saved = JSON.parse(load(key) ?? '{}') as Record<string, boolean>;
    if (from.path in saved) return saved[from.path];
    if (!this.#store.agents.length) await this.#store.refresh();
    const agentId = this.#store.incomingAgentId ?? this.#store.fallbackAgentId;
    const agent = agentId ? (this.#store.agents.find((a) => a.id === agentId)?.name ?? '') : '';
    const allowed = await askConfirm(
      '🐙 Conversación desde otra bóveda',
      `Un agente de «${from.vault}» quiere conversar con un agente de esta bóveda` +
        (agent ? ` (se usará ${agent})` : '') +
        '. Si no hay una conversación abierta para él, se abrirá una nueva. ¿Permitir?',
    );
    saved[from.path] = allowed;
    save(key, JSON.stringify(saved));
    return allowed;
  }
  /** Conversación que atiende a `from`: la pedida, la de su hilo, o una nueva y limpia. */
  async #incomingConversation(cwd: string, req: PeerRequest): Promise<Conversation> {
    const from = req.from;
    if (typeof req.to === 'number') {
      const c = this.#store.conversations.find((x) => x.number === req.to);
      if (!c) throw new Error(`No hay una conversación #${req.to} en esa bóveda.`);
      return c;
    }
    const threadsKey = `synapse.agent.threads.${cwd}`;
    const threads = JSON.parse(load(threadsKey) ?? '{}') as Record<string, string>;
    const thread = `${from.path}|${from.convId}`;
    const known = req.fresh ? null : threads[thread];
    if (known) {
      const open = this.#store.byId(known);
      if (open) return open;
      if (this.#store.history.some((h) => h.id === known)) {
        const c = await this.#store.openFromHistory(cwd, known);
        c.incoming = { vault: from.vault, handle: from.handle, title: from.title };
        return c;
      }
    }
    if (!this.#store.agents.length) await this.#store.refresh();
    const configured = this.#store.incomingAgentId && this.#store.agents.some((a) => a.id === this.#store.incomingAgentId) ? this.#store.incomingAgentId : null;
    const agentId = configured ?? this.#store.fallbackAgentId;
    if (!agentId) throw new Error('Esa ventana de Synapse no tiene ningún agente disponible para atender conversaciones.');
    const c = new Conversation(this.#store, agentId, cwd);
    c.incoming = { vault: from.vault, handle: from.handle, title: from.title };
    this.#store.addConversation(c);
    this.#store.panelOpen = true;
    await c.connect();
    threads[thread] = c.id;
    save(threadsKey, JSON.stringify(threads));
    this.#store.loadHistory(cwd);
    return c;
  }
}
