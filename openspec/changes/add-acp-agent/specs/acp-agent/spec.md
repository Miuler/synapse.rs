# Delta Spec: `acp-agent`

## ADDED Requirements

### Requirement: Catálogo verificado y agentes personalizados ACP

El sistema DEBE ofrecer un catálogo verificado de agentes ACP que incluya **Antigravity** y **Claude**, con descarga
automática de sus binarios cuando no estén presentes, además de permitir registrar agentes personalizados mediante
comando, argumentos y variables de entorno.

#### Scenario: Descarga automática de un agente verificado

- **GIVEN** el usuario selecciona Antigravity o Claude por primera vez y su binario no existe en el directorio
  gestionado de Synapse
- **WHEN** el usuario inicia o selecciona ese agente
- **THEN** Synapse descarga e instala automáticamente el binario correspondiente y establece la conexión ACP por
  `stdio`.

#### Scenario: Registro de agente personalizado

- **GIVEN** el usuario abre la configuración de agentes ACP
- **WHEN** agrega un agente personalizado con nombre, comando y argumentos opcionales
- **THEN** el agente queda disponible en el selector del panel ACP para iniciar sesión.

### Requirement: Comunicación ACP por `stdio`

El backend Rust DEBE ejecutar cada agente ACP como un proceso hijo local comunicándose mediante JSON-RPC sobre `stdin` y
`stdout`, transmitiendo respuestas en streaming, llamadas a herramientas y solicitudes de permisos al frontend.

#### Scenario: Envío de prompt y streaming de respuesta

- **GIVEN** una sesión ACP activa con el directorio del vault como contexto de trabajo
- **WHEN** el usuario envía un mensaje desde el panel ACP
- **THEN** el backend envía la petición JSON-RPC por `stdin` y emite eventos de streaming al frontend conforme llegan
  fragmentos por `stdout`.

#### Scenario: Solicitud de permisos del agente

- **GIVEN** el agente ACP solicita permiso para ejecutar una herramienta o modificar un archivo
- **WHEN** el backend recibe la solicitud por `stdout`
- **THEN** el panel muestra un diálogo rápido de aprobación/rechazo y envía la decisión del usuario de vuelta por
  `stdin`.

### Requirement: Panel independiente a la derecha con modo flotante

La interfaz del agente ACP DEBE mostrarse por defecto como un panel independiente fijo a la derecha del workspace, con
un control para alternar entre modo acoplado (fijo) y modo flotante.

#### Scenario: Alternar entre panel fijo derecho y modo flotante

- **GIVEN** el panel ACP está abierto acoplado a la derecha del editor
- **WHEN** el usuario activa el botón de modo flotante en la cabecera del panel
- **THEN** el panel pasa a mostrarse como ventana flotante movible sobre el workspace, y puede volver a acoplarse a la
  derecha al desactivar la opción.

### Requirement: Selector rápido y configuración dual

El panel ACP DEBE incluir un selector rápido en su cabecera para cambiar de agente en cualquier momento, un estado vacío
con acción directa si no hay agentes configurados, y un acceso directo a la sección ACP en los Ajustes de Synapse.

#### Scenario: Cambio de agente desde la cabecera del panel

- **GIVEN** hay más de un agente ACP disponible (verificado o personalizado)
- **WHEN** el usuario selecciona otro agente en el selector de la cabecera del panel
- **THEN** Synapse cierra la sesión anterior si aplica e inicia la sesión con el nuevo agente seleccionado.
