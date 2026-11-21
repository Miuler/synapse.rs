## Why

Los usuarios de Synapse (desarrolladores, PMs, investigadores, analistas) graban o reciben notas de voz, reuniones y entrevistas en audio, pero actualmente esos archivos no se pueden convertir a texto ni buscar dentro de la bóveda sin salir a herramientas externas o servicios en la nube. Contar con transcripción local mediante una red neuronal en Rust mantiene la filosofía *local-first* y cero telemetría.

## What Changes

- Añadir un puerto de transcripción de audio en `domain` / `application` desacoplado del motor neuronal concreto (abierto a `whisper-rs`, `candle` en Rust puro u otro motor/modelo mejor como `distil-whisper` o `moonshine`).
- Descargar el modelo bajo demanda a la carpeta de datos de la aplicación (sin inflar el binario de Synapse) con progreso y verificación de integridad.
- Decodificar formatos de audio comunes (`.wav`, `.mp3`, `.ogg`, `.m4a`) en Rust y transcribirlos a texto/Markdown, con opción de indexar la transcripción en Tantivy e insertarla en notas.

## Capabilities

### New Capabilities
- `audio-transcription`: Transcripción local de archivos de audio a texto mediante un motor neuronal en Rust (Whisper, `candle` u otro equivalente), con descarga bajo demanda de modelos e integración opcional con notas y búsqueda.

### Modified Capabilities
- (Ninguna por ahora)

## Impact

- **Backend Rust (`src-tauri/src/`):** Nuevo puerto `AudioTranscriber` en `domain`/`application`, caso de uso en `application/use_cases/`, adaptador en `infrastructure/` (decodificación con `symphonia` + motor `candle` o `whisper-rs`) y comando Tauri.
- **Frontend (`src/`):** Botón/acción para transcribir un audio adjunto o grabado, selector/descarga de modelo en Ajustes.
- **CI / Binario:** El binario base no debe incluir pesos de modelos; la compilación en Linux/Windows/macOS debe funcionar sin requerir GPU obligatoria.
