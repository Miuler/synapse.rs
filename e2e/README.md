# Pruebas de interfaz (WebDriver)

Manejan la app de escritorio real con `tauri-driver` y WebKitWebDriver. Cada prueba arranca la app
con un `HOME` temporal y una bóveda de ejemplo: nunca toca tu bóveda ni tu configuración.

Requisitos (una vez):

```bash
cargo install tauri-driver --locked
```

`WebKitWebDriver` viene con el paquete `webkitgtk` del sistema.

Ejecutar:

```bash
bun run build:e2e   # binario propio en src-tauri/target/e2e; carga la interfaz desde el puerto 5301
bun run test:e2e
```

Con `E2E_SHOTS=/ruta/carpeta` las pruebas guardan capturas de pantalla en esa carpeta.

- `smoke.test.ts`: la app arranca con la bóveda de prueba.
- `vault-items.test.ts`: explorador, Quick Open, pestañas, título editable y notas nuevas.
- `perf-ipc.test.ts`: tiempo real por IPC de las dos listas de la bóveda con 6 000 notas.
