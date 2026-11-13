## ADDED Requirements

### Requirement: Índices como artefactos del árbol

El servidor SHALL aceptar índices precomputados por el cliente (segmentos de Tantivy u otras estructuras serializadas)
como nodos de tipo `INDEX` del árbol, con su `content_hash`, y SHALL guardarlos y deduplicarlos igual que cualquier otro
artefacto. El servidor MUST tratarlos como binarios opacos: no los abre, no parsea notas ni construye índices por su
cuenta.

#### Scenario: Segmento de índice en un delta

- **WHEN** el cliente sube un segmento de Tantivy y lo incluye en un delta como nodo `INDEX`
- **THEN** el segmento queda en la instantánea vigente junto a los archivos

#### Scenario: Segmento repetido

- **WHEN** un segmento no cambió entre dos instantáneas
- **THEN** el servidor no lo vuelve a almacenar

### Requirement: Descarga selectiva para clientes ligeros

El servidor SHALL permitir pedir el estado filtrado por tipo de nodo, de modo que un cliente ligero obtenga solo los
nodos `INDEX` de la instantánea vigente y descargue esos artefactos sin descargar los archivos fuente.

#### Scenario: Cliente móvil

- **WHEN** un cliente pide el estado filtrado por `INDEX`
- **THEN** recibe solo los nodos de índice de la última instantánea y puede descargar sus artefactos

### Requirement: Ciclo de vida del índice

Los artefactos de índice SHALL seguir la misma retención y recolección de basura que los demás nodos: al dejar de estar
referenciados por la instantánea vigente y por el historial retenido, la recolección puede borrarlos.

#### Scenario: Índice reemplazado

- **WHEN** un delta reemplaza un segmento por otro y la retención ya no conserva la versión anterior
- **THEN** la recolección de basura borra el segmento viejo
