## 1. Dominio y Casos de Uso (Rust)

- [ ] 1.1 Definir el modelo de resultado (`TranscriptionResult`, segmentos con timestamps) y el puerto `AudioTranscriber` en `src-tauri/src/domain/`.
- [ ] 1.2 Crear el caso de uso `TranscribeAudioUseCase` en `src-tauri/src/application/use_cases/`.

## 2. Infraestructura y Motor Neuronal

- [ ] 2.1 Evaluar y elegir adaptador inicial (`candle` en Rust puro vs `whisper-rs` u otro modelo mejor en español).
- [ ] 2.2 Implementar decodificación y remuestreo a 16 kHz mono con `symphonia` + `rubato` en `infrastructure/`.
- [ ] 2.3 Implementar descarga bajo demanda del modelo con progreso y verificación de hash en el directorio de datos de la app.
- [ ] 2.4 Exponer comando Tauri `#[tauri::command]` para transcribir un archivo de audio y gestionar el modelo.

## 3. Frontend e Integración

- [ ] 3.1 Añadir acción en la UI para transcribir un audio del vault e insertar/guardar la transcripción como nota Markdown.
- [ ] 3.2 Añadir configuración de modelo/idioma en *Ajustes*.
- [ ] 3.3 (Opcional) Indexar transcripciones en Tantivy para que los audios sean buscables.

## 4. Verificación

- [ ] 4.1 Pruebas unitarias del caso de uso (con mock de `AudioTranscriber`) y de decodificación de audio.
- [ ] 4.2 Verificar `bun run check`, `bun run build` y `cargo test --manifest-path src-tauri/Cargo.toml`.
