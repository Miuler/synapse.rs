// Cuota de la cuenta de un agente: qué límite enseñar y cómo.
import { t } from '@shared/lib/i18n';
import type { AgentQuota, QuotaGroup, QuotaWindow } from '../model/types';

export type QuotaLevel = 'ok' | 'warn' | 'low';

const norm = (s: string) => s.toLowerCase().replace(/[^a-z0-9]+/g, '');

/** Grupos que cuentan para el modelo elegido; todos si no se sabe a cuál pertenece. */
export function groupsForModel(quota: AgentQuota, model: string | null): QuotaGroup[] {
  const wanted = norm(model ?? '');
  if (!wanted) return quota.groups;
  const own = quota.groups.filter((g) => g.models.some((m) => norm(m) && (wanted.includes(norm(m)) || norm(m).includes(wanted))));
  return own.length ? own : quota.groups;
}

/** El límite más apretado: el primero que dejaría sin poder seguir. */
export function tightestWindow(groups: QuotaGroup[]): QuotaWindow | null {
  return groups
    .flatMap((g) => g.windows)
    .reduce<QuotaWindow | null>((min, w) => (!min || w.remaining < min.remaining ? w : min), null);
}

export function quotaLevel(remaining: number): QuotaLevel {
  return remaining < 0.1 ? 'low' : remaining < 0.25 ? 'warn' : 'ok';
}

/** Hacia abajo: con 0,4 % no se promete un 1 %. */
export const percent = (remaining: number) => `${Math.floor(remaining * 100 + 1e-9)}%`;

export function windowLabel(window: string): string {
  return window === 'weekly' ? t('quota.weekly') : window === '5h' ? t('quota.fiveHours') : window.replaceAll('_', ' ');
}

/** `5d 22h`, `2h 31min`, `4min`. */
export function duration(ms: number): string {
  const min = Math.max(1, Math.round(ms / 60_000));
  const d = Math.floor(min / 1440);
  const h = Math.floor((min % 1440) / 60);
  if (d) return h ? `${d}d ${h}h` : `${d}d`;
  if (h) return min % 60 ? `${h}h ${min % 60}min` : `${h}h`;
  return `${min}min`;
}

function resetText(w: QuotaWindow, now: number): string {
  return w.resets_at && w.resets_at > now ? t('quota.resets', { time: duration(w.resets_at - now) }) : '';
}

const age = (quota: AgentQuota, now: number) =>
  now - quota.fetched_at < 60_000 ? t('quota.justNow') : t('quota.ago', { time: duration(now - quota.fetched_at) });

/** Todos los límites, para el tooltip del indicador. */
export function quotaTooltip(quota: AgentQuota, agent: string, now = Date.now()): string {
  const lines = quota.groups.flatMap((g) => [
    ...(g.name ? [g.name] : []),
    ...g.windows.map((w) => `  ${windowLabel(w.window)}: ${t('quota.remaining', { percent: percent(w.remaining) })}${resetText(w, now) && ` · ${resetText(w, now)}`}`),
  ]);
  return [t('quota.title', { agent }), ...lines, `${age(quota, now)} · ${t('quota.clickToRefresh')}`].join('\n');
}

/** Todos los límites, en Markdown, para `/usage`. */
export function quotaMarkdown(quota: AgentQuota, agent: string, now = Date.now()): string {
  const cell = (s: string) => s.replaceAll('|', '\\|');
  const rows = quota.groups.flatMap((g) =>
    g.windows.map((w) => `| ${cell([g.name, windowLabel(w.window)].filter(Boolean).join(' · '))} | ${t('quota.remaining', { percent: percent(w.remaining) })} | ${resetText(w, now) || '—'} |`),
  );
  return `**${cell(t('quota.title', { agent }))}** (${age(quota, now)})\n\n| | | |\n|---|---|---|\n${rows.join('\n')}`;
}
