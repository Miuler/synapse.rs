## 1. Base

- [x] 1.1 Limpieza del registro de puertos (bóvedas inexistentes, procesos muertos) y pruebas aisladas con `SYNAPSE_CONFIG_DIR`.
- [ ] 1.2 Token por instancia en el registro y ruta `/peer` del servidor MCP que lo exige.

## 2. Consultar otra bóveda (fase 1)

- [x] 2.1 `list_vaults`: bóvedas abiertas y recientes.
- [x] 2.2 Parámetro `vault` en las herramientas de lectura (lectura directa de los archivos y del índice en solo lectura).
- [x] 2.3 ~~Instancias sin ventana~~: no hacen falta para leer (ver design); se reconsidera en la fase 2 si hace falta.
- [x] 2.4 Permiso «puede consultar a» por bóveda (`.synapse/vault-access.json`), preguntando la primera vez.
- [x] 2.5 Marca 📚 en la respuesta cuando lee otra bóveda.
- [ ] 2.6 e2e con dos bóvedas y el diálogo de permiso.

## 3. Preguntarle a otra bóveda (fase 2)

- [ ] 3.1 `send_to_conversation` a otra bóveda por `/peer`, con nombres `bóveda#n`.
- [ ] 3.2 Sesiones entrantes limpias, hilos por conversación de origen y reutilización explícita.
- [ ] 3.3 Levantar agente en el destino (pedido → para entrantes → último usado) con permisos de ambos lados.
- [ ] 3.4 Respuesta de vuelta al origen y límite de saltos entre instancias.
- [ ] 3.5 Pulpo 🐙 en sesiones entrantes, sus mensajes e instancias sin ventana.

## 4. Verificación

- [ ] 4.1 e2e con dos instancias (dos bóvedas) y el agente falso.
- [ ] 4.2 Prueba con Claude y Antigravity reales.
- [ ] 4.3 CHANGELOG.
