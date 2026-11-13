/**
 * Lo que se le pide al agente con `/feedback`: revisar la conversación y registrar, con la
 * herramienta `report_feedback` del MCP de la instancia, qué mejoraría su trabajo en Synapse.
 */
export function feedbackPrompt(extra: string): string {
  return [
    'Petición de feedback del usuario (no continúes con la tarea anterior).',
    '',
    'Revisa esta conversación y tu experiencia trabajando desde el panel de agentes de Synapse con el MCP `synapse_instance`. ' +
      'Piensa qué haría tu trabajo más fácil, rápido o fiable, y registra cada punto con la herramienta `report_feedback` de `synapse_instance` ' +
      '(una llamada por punto, como mucho 5, de lo más importante a lo menos).',
    '',
    'Qué buscar:',
    '- Herramientas o parámetros del MCP que te faltaron, o que tuviste que reemplazar leyendo o escribiendo archivos directamente.',
    '- Respuestas del MCP confusas, incompletas, demasiado largas o con errores.',
    '- Formatos que no pudiste manejar bien (notas, enlaces, metadatos, diagramas, imágenes, PDF, Git).',
    '- Contexto que Synapse no te pasó y te habría servido (nota activa, selección, estructura de la bóveda, instrucciones).',
    '- Lo que mejoraría el panel del agente para el usuario.',
    '',
    'En cada llamada: `title`, un resumen corto; `details`, qué intentabas, qué pasó, qué propones en concreto ' +
      '(nombre de la herramienta, parámetros y un ejemplo de respuesta si aplica) y por qué ayudaría.',
    'No inventes problemas: si algo funcionó bien, no lo registres. Si no hay nada que mejorar, dilo sin llamar a la herramienta.',
    'Al terminar, responde con una lista breve de lo que registraste.',
    ...(extra ? ['', `Indicaciones del usuario para este feedback: ${extra}`] : []),
  ].join('\n');
}
