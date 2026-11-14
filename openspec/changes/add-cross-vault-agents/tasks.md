## 1. Base

- [x] 1.1 Limpieza del registro de puertos (bóvedas inexistentes, procesos muertos) y pruebas aisladas con `SYNAPSE_CONFIG_DIR`.
- [x] 1.2 Token por instancia en el registro y ruta `/peer` del servidor MCP que lo exige.

## 2. Consultar otra bóveda (fase 1)

- [x] 2.1 `list_vaults`: bóvedas abiertas y recientes.
- [x] 2.2 Parámetro `vault` en las herramientas de lectura (lectura directa de los archivos y del índice en solo lectura).
- [x] 2.3 ~~Instancias sin ventana~~: no hacen falta para leer (ver design); se reconsidera en la fase 2 si hace falta.
- [x] 2.4 Permiso «puede consultar a» por bóveda (`.synapse/vault-access.json`), preguntando la primera vez.
- [x] 2.5 Marca 📚 en la respuesta cuando lee otra bóveda.
- [x] 2.6 e2e con dos bóvedas y el diálogo de permiso.

## 3. Preguntarle a otra bóveda (fase 2)

- [x] 3.1 `send_to_conversation` a otra bóveda por `/peer`, con nombres `bóveda`, `bóveda#n` y `bóveda#nueva`; `list_conversations` incluye las de otras ventanas.
- [x] 3.2 Sesiones entrantes limpias, hilos por conversación de origen y reutilización explícita.
- [x] 3.3 Levantar agente en el destino (agente para entrantes de la bóveda → último usado → primer instalado); permiso del destino preguntado la primera vez; si la bóveda está en los recientes pero no abierta, se abre su ventana con el MCP encendido.
- [x] 3.6 En la conversación entrante, las lecturas se permiten solas y lo demás se rechaza si nadie responde en 2 minutos.
- [x] 3.7 `render_diagram` sobre otra bóveda (lo dibuja la ventana propia con el archivo leído con permiso).
- [x] 3.4 Respuesta de vuelta al origen y límite de saltos entre instancias.
- [x] 3.5 Pulpo 🐙 en sesiones entrantes y sus mensajes (las instancias sin ventana no existen: ver design).

## 4. Verificación

- [ ] 4.1 e2e con dos instancias (dos bóvedas) y el agente falso.
- [ ] 4.2 Prueba con Claude y Antigravity reales.
- [x] 4.3 CHANGELOG.
