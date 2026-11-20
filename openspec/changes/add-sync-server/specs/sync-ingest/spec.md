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

#### Scenario: Claves de Google inaccesibles

- **WHEN** la caché del JWKS venció y el servidor no puede descargarlo de nuevo
- **THEN** el servidor responde `UNAVAILABLE` y no acepta el token sin verificarlo

### Requirement: Aislamiento por tenant

El tenant SHALL derivarse del claim `sub` del token validado, creándose en el primer acceso, y nunca de un campo del
mensaje. El identificador de bóveda SHALL ser único en todo el servidor: una bóveda pertenece a un solo tenant. Si hay una lista de cuentas permitidas configurada, el servidor MUST rechazar a las que no estén en ella. Un
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

#### Scenario: Lectura de una bóveda de otro tenant

- **WHEN** un tenant pide el estado o el manifiesto de una bóveda de otro tenant
- **THEN** el servidor responde `NOT_FOUND`, igual que si la bóveda no existiera

### Requirement: Comprobación del estado

El servidor SHALL ofrecer una operación en la que el cliente envía el `root_hash` que conoce de una bóveda y recibe si
coincide con la instantánea vigente y, si no, el `root_hash` vigente.

#### Scenario: Cliente al día

- **WHEN** el `root_hash` del cliente coincide con el del servidor
- **THEN** la respuesta indica que está sincronizado y no hay transferencia de datos

#### Scenario: Cliente desactualizado

- **WHEN** el `root_hash` del cliente difiere
- **THEN** la respuesta incluye el `root_hash` vigente del servidor

#### Scenario: Bóveda aún no subida

- **WHEN** el cliente pide el estado de una bóveda que el servidor no conoce
- **THEN** el servidor responde `NOT_FOUND` y el cliente puede subirla con un delta sobre la raíz vacía

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
de hashes de bloques ya subidos (chunking definido por contenido, calculado por el cliente). El `content_hash` del
archivo SHALL seguir siendo el del contenido completo. El servidor SHALL guardar cada bloque una sola vez, MUST rechazar
un manifiesto con bloques que no tiene y MUST verificar el `content_hash` y el tamaño del archivo reensamblado.

#### Scenario: Edición pequeña en un archivo grande

- **WHEN** el cliente modifica unos bytes de un archivo de 50 MB ya sincronizado y lo vuelve a enviar
- **THEN** solo se suben los bloques que cambiaron y el almacenamiento crece en esos bloques, no en 50 MB

#### Scenario: Manifiesto con bloques faltantes

- **WHEN** el cliente declara un manifiesto que referencia un bloque que nunca subió
- **THEN** el servidor rechaza la operación indicando los bloques faltantes

#### Scenario: Manifiesto que no corresponde al archivo

- **WHEN** los bloques del manifiesto, unidos en orden, no producen el `content_hash` declarado
- **THEN** el servidor responde `INVALID_ARGUMENT` y no registra el archivo

#### Scenario: Descarga de un archivo en bloques

- **WHEN** el cliente descarga un archivo registrado mediante manifiesto
- **THEN** recibe por streaming los bytes completos del archivo, en orden

### Requirement: Confirmación atómica de deltas del árbol

El servidor SHALL aceptar un delta con los nodos hoja creados o modificados (ruta, tipo `FILE`/`INDEX`, `content_hash`,
tamaño y, si es un movimiento, la ruta de origen), los directorios vacíos que se crean y las rutas eliminadas, junto con
el nuevo `root_hash` calculado por el cliente. El delta SHALL aplicarse completo o no aplicarse, actualizando a la vez
el árbol y el `root_hash` de la instantánea. Todos los contenidos referenciados MUST existir antes de confirmar.

#### Scenario: Delta válido

- **WHEN** el cliente envía un delta sobre la instantánea vigente con todos los artefactos ya subidos
- **THEN** el servidor aplica todos los nodos y la instantánea pasa a tener el nuevo `root_hash`

#### Scenario: Artefacto no subido

- **WHEN** el delta referencia un `content_hash` que el servidor no tiene
- **THEN** el servidor rechaza el delta completo con `FAILED_PRECONDITION` indicando los hashes faltantes

#### Scenario: Primera subida de una bóveda

- **WHEN** el cliente envía el primer delta de una bóveda nueva con base igual a la raíz vacía
- **THEN** el servidor crea la bóveda para ese tenant y aplica el delta

### Requirement: Rutas válidas

Toda ruta de un delta MUST ser relativa a la raíz de la bóveda, usar `/` como separador, estar en UTF-8 NFC y no tener
`/` inicial ni final, segmentos vacíos, `.` ni `..`. El servidor SHALL rechazar el delta completo si alguna no cumple.

#### Scenario: Ruta inválida

- **WHEN** el delta contiene una ruta con `..`, un `/` inicial o un segmento vacío
- **THEN** el servidor responde `INVALID_ARGUMENT` y no aplica nada

### Requirement: Directorios del árbol

Los directorios intermedios de una ruta SHALL crearse implícitamente y su hash SHALL calcularlo el servidor. Un delta
MAY crear directorios vacíos de forma explícita. Eliminar un directorio SHALL eliminar todo su subárbol.

#### Scenario: Directorios implícitos

- **WHEN** el delta crea `a/b/c.md` en una bóveda sin la carpeta `a`
- **THEN** el árbol vivo contiene los directorios `a` y `a/b` con su hash calculado por el servidor

#### Scenario: Borrado de un directorio

- **WHEN** el delta elimina una ruta de tipo `DIR`
- **THEN** desaparecen del árbol vivo el directorio y todo lo que contenía

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
