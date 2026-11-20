# P11 — Índices, prueba de extremo a extremo y documentación

**Tareas OpenSpec:** 5.1, 5.2, 6.1, 3.7, 6.3. **Depende de:** P10.

## Objetivo

1. **Índices** (5.1, 5.2): los nodos `INDEX` ya pasan por todo el flujo; verifica con pruebas que el servidor los trata
   como opacos (nunca los abre) y completa lo que falte. Convención recomendada para el cliente, a documentar (el
   servidor no la impone): segmentos en `.synapse/index/<nombre>`.
2. **Extremo a extremo** (6.1) en `tests/integration/e2e.rs`, con un cliente de prueba en Rust que use solo la API
   gRPC (como el cliente real):
   - Bóveda en un directorio temporal con notas Markdown, un PDF de prueba de > 4 MiB (en bloques) y un «segmento de
     Tantivy» (bytes opacos: no añadas `tantivy` como dependencia).
   - Subida inicial (`FindMissing` → `UploadArtifact`/`DeclareManifest` → `PushDelta`), editar, mover y borrar
     archivos, reemplazar el segmento, un cliente ligero que hace `PullState` solo de `INDEX` y descarga el segmento,
     `ABORTED` y reconciliación desde un segundo cliente, retención `versions:2`, avance del reloj y GC.
   - Verifica: raíz final, árbol vivo, historial de cada ruta, que el almacenamiento físico contiene exactamente los
     objetos esperados (ni más ni menos) y que el segmento viejo se recolectó.
3. **README** (3.7) `synapse-server/README.md`: qué es y qué no; cómo ejecutar (configuración, variables, ejemplo
   TOML); autenticación (ID token de Google, `aud`, lista permitida); flujo de sincronización paso a paso con un
   diagrama de secuencia Mermaid; chunking (umbral recomendado y parámetros de FastCDC del cliente); regla Merkle
   (enlace a `proto/MERKLE.md`); tabla de errores y qué debe hacer el cliente con cada uno; retención y GC; límites;
   despliegue (TLS en el proxy inverso, copias de seguridad de SQLite en WAL).
4. **Cierre** (6.3): entrada en `CHANGELOG.md` que resuma el servidor; sección en `ARCHITECTURE.md` (qué es, capas,
   flujo, dónde está el contrato); menciona `synapse-server/` en `AGENTS.md`/`CLAUDE.md` solo si describen la
   estructura del repo (no reorganices nada).
5. Marca las tareas; **no archives** el cambio de OpenSpec: lo decide el usuario (falta P12).

## Pruebas exigidas

- `indices::cliente_ligero_solo_recibe_index_y_descarga_el_segmento`.
- `indices::segmento_sin_cambios_no_se_vuelve_a_almacenar`.
- `indices::segmento_reemplazado_se_recolecta_al_salir_de_la_retencion`.
- `e2e::boveda_completa` (puede ser una sola prueba larga con pasos comentados).

## Hecho cuando

Verificación en verde, 3.7, 5.1, 5.2, 6.1 y 6.3 marcadas, CHANGELOG.
