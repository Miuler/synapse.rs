## 1. Base

- [x] 1.1 Limpieza del registro de puertos (bóvedas inexistentes, procesos muertos) y pruebas aisladas con `SYNAPSE_CONFIG_DIR`.
- [ ] 1.2 Token por instancia en el registro y ruta `/peer` del servidor MCP que lo exige.

## 2. Consultar otra bóveda (fase 1)

- [ ] 2.1 `list_vaults`: bóvedas abiertas (con o sin ventana) y recientes.
- [ ] 2.2 Parámetro `vault` en `search_notes`, `full_text_search`, `read_note`, `list_folder_contents` y `render_diagram`, reenviado por `/peer`.
- [ ] 2.3 Instancias sin ventana para bóvedas cerradas, con apagado tras 5 minutos sin pedidos.
- [ ] 2.4 Permiso «puede consultar a» por bóveda, preguntando la primera vez.
- [ ] 2.5 Marca 📚 en la herramienta cuando lee otra bóveda.

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
