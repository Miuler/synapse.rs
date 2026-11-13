import { describe, expect, it } from 'bun:test';
import { feedbackPrompt } from './feedback-prompt';

describe('feedbackPrompt', () => {
  it('pide usar report_feedback del MCP de la instancia', () => {
    const p = feedbackPrompt('');
    expect(p).toContain('report_feedback');
    expect(p).toContain('synapse_instance');
    expect(p).not.toContain('Indicaciones del usuario');
  });

  it('añade las indicaciones del usuario', () => {
    expect(feedbackPrompt('céntrate en los diagramas')).toContain('Indicaciones del usuario para este feedback: céntrate en los diagramas');
  });
});
