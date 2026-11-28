import { t } from '@shared/lib/i18n';

/**
 * Lo que se le pide al agente con `/feedback`: revisar la conversación y registrar, con la
 * herramienta `report_feedback` del MCP de la instancia, qué mejoraría su trabajo en Synapse.
 */
export function feedbackPrompt(extra: string): string {
  return [
    t('acp.prompts.feedbackHeader'),
    '',
    t('acp.prompts.feedbackIntro'),
    '',
    t('acp.prompts.feedbackWhatToLookFor'),
    t('acp.prompts.feedbackItemTools'),
    t('acp.prompts.feedbackItemResponses'),
    t('acp.prompts.feedbackItemFormats'),
    t('acp.prompts.feedbackItemContext'),
    t('acp.prompts.feedbackItemPanel'),
    '',
    t('acp.prompts.feedbackCallFormat'),
    t('acp.prompts.feedbackNoInvent'),
    t('acp.prompts.feedbackSummary'),
    ...(extra ? ['', t('acp.prompts.feedbackUserNotes', { extra })] : []),
  ].join('\n');
}
