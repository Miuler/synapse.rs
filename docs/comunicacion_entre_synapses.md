# Comunicación entre Synapses

Cada ventana de Synapse abre una bóveda y puede tener varias conversaciones con agentes
(`#1`, `#2`…). Un agente puede escribirle a otra conversación de su misma ventana o a una
conversación de **otra bóveda abierta en otra ventana**. Esta página resume cómo funciona,
quién decide qué y cuáles son los límites.

## Quién le escribe a quién

| Origen | Destino | Cómo |
|---|---|---|
| Agente de esta ventana | Otra conversación de esta ventana | MCP `send_to_conversation` con `to: "#2"` |
| Agente de esta ventana | Otra bóveda | `send_to_conversation` con `to: "Bóveda"`, `"Bóveda#2"` o `"Bóveda#nueva"` |
| Cliente MCP externo (p. ej. Claude Code) | Conversación de esta ventana | `send_to_conversation`; las respuestas se recogen con `read_replies` |
| Usuario | Cualquiera de las anteriores | En el chat: `#2`, `@Bóveda` o `@Bóveda#2` (se le explica al agente a quién apunta) |

`list_conversations` muestra las conversaciones de esta ventana y las de las otras bóvedas
abiertas.

## El canal entre ventanas (`/peer`)

- Cada instancia anota su puerto MCP, su bóveda y un **token** aleatorio en
  `~/.config/synapsers/mcp-ports.json`. Solo los procesos del usuario pueden leerlo.
- Una instancia le habla a otra por HTTP local (`POST http://127.0.0.1:<puerto>/peer`) con la
  cabecera `X-Synapse-Token`. Sin el token correcto, la respuesta es `403`.
- Pedidos:
  - `ping`: saber qué instancias están abiertas;
  - `list`: sus conversaciones, con estado, título y extracto;
  - `send`: un mensaje nuevo;
  - `deliver`: la respuesta a una pregunta.
- Si la bóveda destino está entre las recientes pero cerrada, `send` la abre en otra ventana y
  espera hasta 30 s a que responda. Elegir una bóveda ⚪ en `@` también la enciende.
- En el frontend, esta lógica vive en `src/entities/acp-agent/model/peer-link.ts`.

## Permisos

- **Aceptar una bóveda:** la primera vez que un agente de otra bóveda le escribe a esta, Synapse
  pregunta «¿Permitir?». La respuesta se recuerda por bóveda (`synapse.agent.accept.<bóveda>`) y
  se puede revisar o quitar en **Ajustes → Agentes de IA → Bóvedas que pueden conversar con
  esta**. Al quitarla, se vuelve a preguntar.
- **En qué conversación entra:**
  - la pedida (`#2`);
  - la del mismo hilo (misma conversación de origen), aunque esté cerrada en el historial;
  - o una nueva con el agente para conversaciones de otras bóvedas, o si no hay, con el
    agente por defecto (el último usado, el elegido en Ajustes o el primero disponible).
- **Herramientas en una conversación entrante**, donde quizá nadie está mirando:
  - las de lectura (`read`, `search`, `fetch`, `think`) se permiten solas;
  - las demás esperan al usuario: Synapse pide atención (ventana urgente y notificación) y,
    si nadie responde en **2 minutos** (`INCOMING_PERMISSION_TIMEOUT`), se rechazan.

## Límites

- **`MAX_AGENT_HOPS` = 3:** idas y vueltas seguidas entre agentes sin que intervenga el
  usuario. Al pasarlo, `send_to_conversation` falla y se le pide al agente que resuma y deje
  decidir al usuario. Cuando el usuario escribe, el contador vuelve a cero.
- **Una conversación ocupada no se interrumpe:** el mensaje queda en su buzón y entra al
  terminar su turno (`queued`).
- **Respuestas:** si la pregunta pedía respuesta (`expects_reply`), el texto de la respuesta
  vuelve solo como un mensaje nuevo. Se ve cerrado («De Agente · #2 · 2,3 KB ▸») y renderizado
  al abrirlo.

## Depurar

Con **Ajustes → Agentes de IA → Registro de depuración de ACP** activo, Synapse anota en
`~/.config/synapsers/acp-debug.jsonl`:

- cada mensaje ACP con los agentes (`in`/`out`);
- los pedidos `/peer`:
  - `peer-out` / `peer-reply`: lo que esta ventana pide a otra y lo que le contestan;
  - `peer-in` / `peer-answer`: lo que le piden a esta y lo que contesta.

Los `ping` no se anotan.
