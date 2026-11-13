## ADDED Requirements

### Requirement: Servidor independiente de la aplicación de escritorio

El servidor de sincronización SHALL compilarse y ejecutarse como un binario propio, sin depender de Tauri, del frontend
ni del instalador de escritorio. SHALL leer su configuración (dirección de escucha, base de datos, backend de blobs) de
un archivo o de variables de entorno, y SHALL exponer una comprobación de salud.

#### Scenario: Arranque sin la app de escritorio

- **WHEN** se ejecuta el binario del servidor en una máquina sin la app de escritorio instalada
- **THEN** el servidor arranca, abre su base de datos y responde a la comprobación de salud

#### Scenario: El instalador de escritorio no incluye el servidor

- **WHEN** se genera un release de la app de escritorio
- **THEN** los artefactos no contienen el binario del servidor

### Requirement: Autenticación con Google

Toda llamada MUST ir autenticada con un ID token de Google en `authorization: Bearer`. El servidor SHALL validar la
firma contra el JWKS de Google, el emisor, la audiencia (el `client_id` configurado) y la vigencia, y SHALL rechazar
llamadas sin token o con token inválido o caducado.

#### Scenario: Llamada sin token

- **WHEN** un cliente invoca cualquier operación sin token
- **THEN** el servidor responde `UNAUTHENTICATED` y no modifica nada

#### Scenario: Token caducado o con audiencia distinta

- **WHEN** el token está caducado o su audiencia no es el `client_id` del servidor
- **THEN** el servidor responde `UNAUTHENTICATED` y no modifica nada

### Requirement: Aislamiento por tenant

El tenant SHALL derivarse del claim `sub` del token validado, creándose en el primer acceso, y nunca de un campo del
mensaje. Si hay una lista de cuentas permitidas configurada, el servidor MUST rechazar a las que no estén en ella. Un
tenant nunca puede leer, referenciar ni sobrescribir árboles o artefactos de otro.

#### Scenario: Primer acceso de una cuenta nueva

- **WHEN** llega un token válido cuyo `sub` no tiene tenant y la cuenta está permitida
- **THEN** el servidor crea el tenant con la retención por defecto y atiende la llamada

#### Scenario: Cuenta no permitida

- **WHEN** hay lista de cuentas permitidas y el token válido pertenece a una cuenta fuera de ella
- **THEN** el servidor responde `PERMISSION_DENIED` y no crea tenant

#### Scenario: Bóveda de otro tenant

- **WHEN** un tenant envía un delta sobre una bóveda que pertenece a otro tenant
- **THEN** el servidor responde `NOT_FOUND` y no modifica nada

### Requirement: Comprobación del estado

El servidor SHALL ofrecer una operación en la que el cliente envía el `root_hash` que conoce de una bóveda y recibe si
coincide con la instantánea vigente y, si no, el `root_hash` vigente.

#### Scenario: Cliente al día

- **WHEN** el `root_hash` del cliente coincide con el del servidor
- **THEN** la respuesta indica que está sincronizado y no hay transferencia de datos

#### Scenario: Cliente desactualizado

- **WHEN** el `root_hash` del cliente difiere
- **THEN** la respuesta incluye el `root_hash` vigente del servidor

### Requirement: Transferencia de artefactos direccionados por contenido

El servidor SHALL recibir y entregar por streaming artefactos (archivos o índices) identificados por su `content_hash`
(BLAKE3 del contenido sin comprimir). MUST verificar al terminar una subida que el hash del contenido recibido coincide
con el declarado y descartarlo si no coincide. Subir un artefacto que ya existe para el tenant SHALL ser idempotente y
no duplicar almacenamiento.

#### Scenario: Hash que no coincide

- **WHEN** el cliente sube un artefacto cuyo contenido no corresponde al `content_hash` declarado
- **THEN** el servidor responde `INVALID_ARGUMENT` y no guarda el artefacto

#### Scenario: Subida repetida

- **WHEN** el cliente sube dos veces el mismo artefacto
- **THEN** ambas respuestas son exitosas y el almacenamiento contiene una sola copia

#### Scenario: Mismo contenido en dos rutas

- **WHEN** dos archivos de la misma bóveda tienen idéntico contenido
- **THEN** el servidor guarda un único artefacto referenciado por ambas rutas

#### Scenario: Descarga

- **WHEN** el cliente pide un `content_hash` que su tenant tiene
- **THEN** recibe por streaming exactamente los bytes originales

### Requirement: Archivos grandes en bloques definidos por contenido

Para archivos por encima de un umbral configurable, el cliente SHALL poder declarar un manifiesto con la lista ordenada
de hashes de bloques (chunking definido por contenido) en lugar de subir el archivo completo. El `content_hash` del
archivo SHALL seguir siendo el del contenido completo. El servidor SHALL guardar cada bloque una sola vez y MUST
rechazar un manifiesto que referencie bloques que no tiene.

#### Scenario: Edición pequeña en un archivo grande

- **WHEN** el cliente modifica unos bytes de un archivo de 50 MB ya sincronizado y lo vuelve a enviar
- **THEN** solo se suben los bloques que cambiaron y el almacenamiento crece en esos bloques, no en 50 MB

#### Scenario: Manifiesto con bloques faltantes

- **WHEN** el cliente declara un manifiesto que referencia un bloque que nunca subió
- **THEN** el servidor rechaza la operación indicando los bloques faltantes

### Requirement: Confirmación atómica de deltas del árbol

El servidor SHALL aceptar un delta con los nodos creados o modificados (ruta, tipo `FILE`/`DIR`/`INDEX`, `content_hash`,
tamaño) y las rutas eliminadas, junto con el nuevo `root_hash` calculado por el cliente. El delta SHALL aplicarse
completo o no aplicarse, actualizando a la vez el árbol y el `root_hash` de la instantánea. Todos los contenidos
referenciados MUST existir antes de confirmar.

#### Scenario: Delta válido

- **WHEN** el cliente envía un delta sobre la instantánea vigente con todos los artefactos ya subidos
- **THEN** el servidor aplica todos los nodos y la instantánea pasa a tener el nuevo `root_hash`

#### Scenario: Artefacto no subido

- **WHEN** el delta referencia un `content_hash` que el servidor no tiene
- **THEN** el servidor rechaza el delta completo indicando los hashes faltantes

### Requirement: Validación de integridad del árbol

El servidor SHALL actuar como notario: recalcula el `root_hash` a partir del árbol resultante y MUST rechazar el delta
si no coincide con el declarado por el cliente.

#### Scenario: Root hash inconsistente

- **WHEN** el `root_hash` declarado no corresponde al árbol que resulta de aplicar el delta
- **THEN** el servidor responde `FAILED_PRECONDITION` y no aplica nada

### Requirement: Concurrencia optimista por instantánea

Cada delta MUST incluir el `root_hash` base sobre el que se calculó. Si no coincide con el vigente, el servidor SHALL
rechazarlo como conflicto sin aplicar nada.

#### Scenario: Delta sobre una instantánea vieja

- **WHEN** el cliente envía un delta cuyo `root_hash` base no es el vigente
- **THEN** el servidor responde `ABORTED` con el `root_hash` vigente y no aplica ningún nodo

### Requirement: Descarga del estado actual

El servidor SHALL devolver por streaming el manifiesto completo de la instantánea vigente de una bóveda (ruta, tipo,
`content_hash`, tamaño de cada nodo), para clientes desactualizados por varias versiones o que se reconcilian tras un
conflicto.

#### Scenario: Reconciliación tras conflicto

- **WHEN** el cliente recibe `ABORTED` y pide el estado
- **THEN** obtiene el `root_hash` vigente y todos los nodos con sus hashes
