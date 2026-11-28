import { afterEach, beforeEach, describe, expect, it } from 'bun:test';
import { i18n } from '@shared/lib/i18n';
import { feedbackPrompt } from './feedback-prompt';

const initial = i18n.locale;
beforeEach(() => i18n.setLocale('es'));
afterEach(() => i18n.setLocale(initial));

describe('feedbackPrompt', () => {
  it('pide usar report_feedback del MCP de la instancia', () => {
    const p = feedbackPrompt('');
    expect(p).toContain('report_feedback');
    expect(p).toContain('synapse_instance');
    expect(p).not.toContain('Indicaciones del usuario');

    i18n.setLocale('en');
    const enPrompt = feedbackPrompt('');
    expect(enPrompt).toContain('User feedback request');
    expect(enPrompt).not.toContain('User instructions');
  });

  it('añade las indicaciones del usuario', () => {
    expect(feedbackPrompt('céntrate en los diagramas')).toContain('Indicaciones del usuario para este feedback: céntrate en los diagramas');
    i18n.setLocale('en');
    expect(feedbackPrompt('focus on diagrams')).toContain('User instructions for this feedback: focus on diagrams');
  });
});
