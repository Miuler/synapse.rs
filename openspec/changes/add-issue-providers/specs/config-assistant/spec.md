# Spec Delta

## Purpose

El agente integrado ayuda a configurar Synapse, empezando por el origen de un tablero: propone la consulta, la prueba
y la corrige mientras el usuario revisa y decide.

## ADDED Requirements

### Requirement: Agente disponible al configurar

La configuración del origen de un tablero SHALL permitir usar al mismo tiempo el chat del agente.

#### Scenario: Pedir una consulta

- **GIVEN** la configuración de un tablero abierta
- **WHEN** el usuario pide al agente «mis errores abiertos de esta iteración»
- **THEN** puede conversar con él sin cerrar la configuración

### Requirement: Probar consultas

El agente SHALL poder probar un filtro o consulta con la conexión elegida, en solo lectura, y recibir el recuento y
una muestra limitada, o el error del proveedor.

#### Scenario: Consulta con error

- **WHEN** el agente prueba una consulta mal escrita
- **THEN** recibe el motivo que da el proveedor y puede corregirla y volver a probar

### Requirement: Los cambios los guarda el usuario

El agente SHALL poder modificar el filtro, la consulta y los estados del borrador, que la configuración SHALL mostrar
como cambiados. El agente MUST NOT guardar la configuración, traer ni enviar tareas.

#### Scenario: Proponer un cambio

- **WHEN** el agente cambia la consulta del borrador
- **THEN** la configuración la muestra marcada como cambiada
- **AND** el archivo del tablero no cambia hasta que el usuario guarda

### Requirement: Credenciales fuera del alcance del agente

Las herramientas de configuración MUST NOT devolver ni aceptar credenciales, ni crear conexiones o cambiar su
servidor.

#### Scenario: Pedir el token

- **WHEN** el agente consulta el contexto de la configuración
- **THEN** recibe el nombre, proveedor y servidor de la conexión, y ningún dato de su credencial

### Requirement: Herramientas privadas

Las herramientas de configuración MUST ofrecerse solo a las sesiones de agentes abiertas por Synapse y MUST NOT estar
disponibles para otros clientes del servidor MCP ni para páginas web.

#### Scenario: Cliente externo

- **WHEN** un cliente sin el token de una sesión lista o llama a las herramientas
- **THEN** no aparecen y la llamada se rechaza

#### Scenario: Sin configuración abierta

- **WHEN** el agente llama a una herramienta sin ninguna configuración abierta
- **THEN** se le indica que el usuario debe abrirla
