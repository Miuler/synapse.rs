## ADDED Requirements

### Requirement: El agente solo usa el MCP de su instancia

El sistema SHALL encaminar toda consulta o mensaje a otra bóveda a través de la instancia de Synapse del agente; el
agente MUST NOT recibir la dirección del MCP de otra instancia.

#### Scenario: Consultar otra bóveda

- **GIVEN** las bóvedas «diagramas» y «mainframe» abiertas en dos ventanas
- **WHEN** el agente de «diagramas» llama a `search_notes {vault: "mainframe", query: "copybook"}`
- **THEN** la instancia de «diagramas» consulta a la de «mainframe» por el canal interno y devuelve los resultados.

### Requirement: Descubrimiento de instancias y registro limpio

El sistema SHALL listar las bóvedas disponibles (`list_vaults`) a partir del registro de puertos y de los recientes, y
SHALL quitar del registro las bóvedas inexistentes y marcar detenidas las de procesos que ya no viven.

#### Scenario: Entradas fantasma

- **GIVEN** una entrada del registro cuya carpeta ya no existe
- **WHEN** una instancia lee el registro
- **THEN** la entrada se elimina.

### Requirement: Instancias sin ventana con apagado por inactividad

Cuando la bóveda consultada no está abierta, el sistema SHALL lanzar una instancia sin ventana (`--mcp`) que MUST
apagarse sola tras un tiempo sin pedidos.

#### Scenario: Bóveda cerrada

- **GIVEN** la bóveda «mainframe» está en los recientes pero no abierta
- **WHEN** un agente consulta `read_note {vault: "mainframe"}`
- **THEN** Synapse lanza la instancia sin ventana, responde, y la instancia se apaga tras 5 minutos sin pedidos.

### Requirement: Conversaciones entrantes limpias

Al preguntarle al agente de otra bóveda, el sistema SHALL abrir por defecto una sesión nueva en el destino, SHALL
reutilizar la sesión del mismo hilo para las repreguntas de la misma conversación de origen, y SHALL usar otra
conversación existente solo si se pide explícitamente.

#### Scenario: Primera pregunta

- **GIVEN** «mainframe» no tiene conversaciones abiertas
- **WHEN** el agente de «diagramas#2» pregunta a «mainframe»
- **THEN** «mainframe» abre una sesión nueva con su agente, marcada como entrante desde «diagramas#2», y la respuesta
  vuelve a «diagramas#2».

#### Scenario: Repregunta del mismo hilo

- **GIVEN** «diagramas#2» ya preguntó a «mainframe»
- **WHEN** vuelve a preguntar
- **THEN** la pregunta llega a la misma sesión entrante.

### Requirement: Permisos de ambos lados

El sistema SHALL pedir permiso al usuario la primera vez que una bóveda consulta a otra, que una bóveda acepta
conversaciones de otra y que se levanta un agente para una conversación entrante, y SHALL recordar la respuesta.

#### Scenario: Primera consulta

- **GIVEN** «diagramas» nunca consultó a «mainframe»
- **WHEN** su agente lo intenta
- **THEN** Synapse pregunta al usuario y recuerda la respuesta.

### Requirement: Marca del pulpo

Las sesiones entrantes, sus mensajes y las instancias sin ventana levantadas por otra instancia SHALL mostrarse con el
ícono del pulpo.

#### Scenario: Sesión entrante en el panel

- **WHEN** una sesión entrante aparece en el panel de «mainframe»
- **THEN** su fila muestra el pulpo y el origen (`🐙 ⇠ diagramas#2`).
