# Design: Conversaciones y consultas entre bóvedas

## Principio: el agente solo habla con su ventana

El agente solo conoce el MCP de su instancia (`synapse_instance`). La instancia hace de intermediaria con las demás por
un canal interno que el agente no ve, así que no puede llamarse a sí mismo creyendo que es otra ni hablar con otra
instancia sin pasar por la suya.

```
Agente (diagramas) ──MCP──▶ Synapse diagramas ──/peer + token──▶ Synapse mainframe ──ACP──▶ Agente (mainframe)
```

## Descubrimiento

El registro de puertos (`~/.config/synapsers/mcp-ports.json`) es la lista compartida. Cada instancia anota bóveda,
puerto, PID, si tiene ventana y un token aleatorio. Una instancia es «otra viva» si está `running`, su PID vive y no es
el propio, y responde `GET /mcp`. Al leer el registro se limpian las entradas fantasma (bóveda inexistente, proceso
muerto).

## Dos formas de usar otra bóveda

1. **Consultar su conocimiento** (fase 1): `search_notes {vault}`, `read_note {vault}`… Solo lectura, sin agente
   en el destino. **Decisión de implementación:** la instancia del agente lee directamente los archivos de la otra
   bóveda y su índice de búsqueda en solo lectura (`FullTextIndexImpl::open_read_only`, sin tomar el bloqueo de
   escritura que tiene la instancia de esa bóveda). Así no hace falta levantar instancias sin ventana ni el canal
   entre instancias para leer: funciona esté o no abierta la otra bóveda. `render_diagram` sobre otra bóveda queda
   para la fase 2 (necesita la ventana que la tenga abierta). Los permisos se guardan en
   `<bóveda>/.synapse/vault-access.json`; sin ventana (modo `--mcp`) solo se consultan las bóvedas ya autorizadas.
2. **Preguntarle a su agente** (fase 2): `send_to_conversation {to: "mainframe"}`.
   - Por defecto, **sesión nueva y limpia** en el destino.
   - **Hilo**: las repreguntas de la misma conversación de origen van a la misma sesión entrante.
   - Reutiliza otra conversación solo si se pide explícitamente (`mainframe#1`).
   - Si no hay agente, lo levanta (agente pedido → agente para entrantes de la bóveda → último usado), siempre que los
     permisos lo autoricen. Necesita la ventana del destino (los agentes los lanza la app con ventana): si no está
     abierta, se abre.
   - Las lecturas del agente entrante se permiten solas; las escrituras piden permiso en la ventana del destino y, sin
     respuesta, se rechazan avisando al origen.

## Nombres y recursión

Las conversaciones se nombran `bóveda#n`; `#n` a secas es «esta ventana». `list_conversations` marca `is_you` (por la
URL `?conv=`) y la ventana de cada una. El canal `/peer` solo devuelve conversaciones locales y entrega mensajes
directos (sin reenviar), así que no hay recursión; el límite de 3 idas y vueltas viaja con el mensaje.

## Conexiones

El canal entre instancias es HTTP de pedido y respuesta: no hay conexiones persistentes. Lo que vive en el tiempo son
las instancias sin ventana (con apagado por inactividad) y las sesiones entrantes (se cierran tras 10 minutos sin
actividad y quedan en el historial del destino, marcadas como entrantes).

## Permisos

| Lado | Ajuste | Por defecto |
|---|---|---|
| Origen | Puede consultar a: … | Pregunta la primera vez |
| Destino | Acepta conversaciones de: … | Pregunta la primera vez |
| Destino | Puede levantar un agente para ellas | Pregunta la primera vez |

## Marca visual

Lo que viene de otra instancia lleva un pulpo (🐙): la fila de una sesión entrante (`🐙 ⇠ diagramas#2 · …`), sus
mensajes y las instancias sin ventana. Más adelante, un ícono SVG propio.
