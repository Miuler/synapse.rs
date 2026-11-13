## ADDED Requirements

### Requirement: Registro de versiones por archivo
Cada operación de crear o modificar aplicada por un delta SHALL dejar una versión del archivo con: bóveda, ruta, identificador de versión, `content_hash`, tamaño y fecha. Mover y borrar SHALL quedar registrados sin perder las versiones anteriores de la ruta. Registrar una versión MUST NOT copiar contenido: solo guarda la referencia al `content_hash`.

#### Scenario: Dos ediciones del mismo archivo
- **WHEN** se confirman dos deltas que modifican `notas/a.md`
- **THEN** el historial de `notas/a.md` muestra tres versiones (creación y dos cambios) y el almacenamiento de blobs solo crece por los contenidos nuevos

#### Scenario: Archivo borrado
- **WHEN** se confirma un delta que borra `notas/a.md`
- **THEN** la ruta desaparece del árbol vivo pero sus versiones previas siguen en el historial mientras la retención lo permita

### Requirement: Consulta del historial
El servidor SHALL listar las versiones de una ruta, de la más reciente a la más antigua, y SHALL entregar el contenido de cualquier versión retenida (reensamblando bloques si el archivo se guardó en bloques).

#### Scenario: Contenido de una versión antigua
- **WHEN** el cliente pide el contenido de una versión retenida
- **THEN** recibe exactamente los bytes que tenía el archivo en esa versión

### Requirement: Retención por plan
Cada tenant SHALL tener una política de retención configurable: últimas N versiones por archivo, versiones de los últimos N días, o ilimitada. La versión vigente de cada ruta del árbol vivo MUST conservarse siempre, sin importar la política. Las versiones fuera de la política SHALL eliminarse del historial.

#### Scenario: Plan con 5 versiones
- **WHEN** un archivo de un tenant con política «últimas 5 versiones» llega a 7 versiones y corre la retención
- **THEN** quedan las 5 más recientes en el historial

#### Scenario: Versión vigente antigua
- **WHEN** la política es «30 días» y un archivo no cambia desde hace 90 días
- **THEN** su versión vigente se conserva

### Requirement: Recolección de basura de contenidos
Un proceso del servidor SHALL borrar físicamente los blobs y bloques que no estén referenciados por el árbol vivo, por el historial retenido ni por la información de índice retenida. MUST NOT borrar contenido que se subió hace poco y aún no fue confirmado por un delta, durante un periodo de gracia configurable.

#### Scenario: Contenido sin referencias
- **WHEN** la retención elimina la última versión que apuntaba a un blob y corre la recolección pasado el periodo de gracia
- **THEN** el blob se borra del almacenamiento

#### Scenario: Blob compartido
- **WHEN** un blob lo referencian una versión eliminada y otra ruta viva
- **THEN** la recolección no lo borra

#### Scenario: Subida en curso
- **WHEN** un cliente subió un blob y todavía no confirmó el delta
- **THEN** la recolección no lo borra antes de que venza el periodo de gracia
