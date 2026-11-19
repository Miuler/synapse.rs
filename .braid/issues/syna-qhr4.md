---
schema_version: 9
id: syna-qhr4
title: 'Índice: perfil BPMN'
priority: P3
status: done
deps: []
tags:
- busqueda
- indice
owner: null
created_at: 2026-11-18T09:21:23.347080556Z
completed_at: 2026-11-19T21:11:00.443585485Z
---

Sobre el extractor XML genérico (xml_extractor.rs): reglas por formato (bpmn:task name, xs:element name, operaciones WSDL, groupId:artifactId…), como ya hay para ArchiMate y draw.io.

ALCANCE (2026-11-19): solo BPMN. XSD/WSDL, pom.xml y Spring se descartan: el índice se centra en documentación y diagramas; el código y la configuración se siguen buscando como texto plano, sin entenderlos.
HECHO: bpmn_extractor.rs (nombre de cada paso con su tipo —userTask, exclusiveGateway…—, documentación y anotaciones; descarta bpmndi:* e ids), tipo de documento `bpmn` (filtro kind:bpmn) y SCHEMA_VERSION 6 para reindexar.
