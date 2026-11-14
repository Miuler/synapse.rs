# Proposal: Conversaciones y consultas entre bóvedas de Synapse

## Why

El usuario trabaja con varias bóvedas a la vez, cada una en su ventana de Synapse: diagramas de secuencia en una,
documentación de mainframe en otra y microservicios en una tercera. Hoy cada agente solo ve su propia bóveda, así que
para validar un diagrama contra la documentación de mainframe hay que copiar a mano de una ventana a otra. Además, las
conversaciones con agentes de una misma ventana ya pueden hablarse (`list_conversations`, `send_to_conversation`), pero
no las de ventanas distintas.

## What Changes

- **Consultar otra bóveda**: herramienta MCP `list_vaults` y parámetro `vault` en las herramientas de lectura
  (`search_notes`, `full_text_search`, `read_note`, `list_folder_contents`, `render_diagram`). La instancia de origen
  hace de intermediaria; el agente solo habla con el MCP de su ventana (`synapse_instance`).
- **Instancias sin ventana**: si la bóveda consultada no está abierta, Synapse lanza `synapsers --mcp --vault <ruta>`
  en segundo plano, que se apaga sola tras unos minutos sin pedidos.
- **Canal entre instancias**: una ruta interna del servidor MCP (`/peer`) protegida con un token anotado en el registro
  de puertos; solo la usan las instancias, nunca los agentes.
- **Conversaciones entrantes**: «pregúntale a mainframe…» abre por defecto una sesión nueva y limpia en la bóveda de
  destino (o retoma el hilo de la misma conversación de origen, o una conversación pedida explícitamente), levantando su
  agente si hace falta.
- **Permisos de ambos lados**: qué bóvedas puede consultar cada una, cuáles acepta y si puede levantar agentes para
  ellas; la primera vez se pregunta al usuario y se recuerda.
- **Ícono del pulpo**: lo que viene de otra instancia (sesiones entrantes, sus mensajes, instancias sin ventana) se marca
  con un pulpo (🐙 por ahora; luego un ícono propio).
- **Registro de puertos limpio**: se quitan las bóvedas que ya no existen y se marcan detenidas las de procesos muertos.

## Capabilities

- `cross-vault-agents`: descubrimiento de instancias, consulta de otras bóvedas, conversaciones entrantes entre
  instancias, permisos y marca visual.

## Impact

- Backend (Rust): registro de puertos (token, instancias sin ventana, limpieza), servidor MCP (ruta `/peer`, parámetro
  `vault`), herramientas MCP nuevas, `ConversationHub` extendido a otras instancias.
- Frontend (Svelte): sesiones entrantes en el panel con su origen y el pulpo, ajustes de permisos por bóveda.
- Riesgo: más estados posibles (ventanas abiertas o no, instancias sin ventana, hilos). Se construye por fases y se
  puede retirar si resulta demasiado.
