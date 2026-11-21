## Context

Synapse sigue Arquitectura Cebolla en el backend Rust (`domain` → `application` → `infrastructure`). La transcripción de audio debe encajar sin acoplar el dominio ni los casos de uso a una librería de ML específica, dejando abierta la elección entre `candle` (Rust puro, sin C++), `whisper-rs` (`whisper.cpp`) o modelos más recientes/eficientes.

## Goals / Non-Goals

**Goals:**
- Transcripción 100 % local (CPU por defecto para portabilidad en Linux/Windows/macOS, con aceleración GPU opcional).
- Motor intercambiable mediante trait en `domain` / `application`.
- Descarga bajo demanda de modelos cuantizados a la carpeta de datos de Synapse.

**Non-Goals:**
- No entrenar modelos ni hacer *fine-tuning* desde la app.
- No enviar audio a APIs externas de pago en esta capacidad base.

## Decisions

- **Puerto `AudioTranscriber`:** Definido en `domain` o `application` recibiendo ruta/bytes de audio e idioma opcional, devolviendo segmentos con marcas de tiempo y texto completo.
- **Decodificación de audio:** Usar `symphonia` (+ `rubato` para remuestreo a 16 kHz mono) en `infrastructure` para soportar `.wav`, `.mp3`, `.ogg` y `.m4a` sin depender de `ffmpeg` externo.
- **Evaluación de motor (`candle` vs `whisper-rs`):** Priorizar `candle` (Rust puro, fácil de compilar en CI cruzado) o `whisper-rs` si ofrece mejor rendimiento/cuantización en CPU y GPU; la decisión final queda encapsulada en el adaptador de `infrastructure`.
- **Almacenamiento de modelos:** Guardar pesos en el directorio de datos de la app (no en el binario ni en el vault).

## Risks / Trade-offs

- **Tiempo de inferencia en CPU:** Modelos grandes (`medium`/`large`) pueden ser lentos sin GPU → Mitigación: ofrecer modelos ligeros/cuantizados por defecto (`base`, `small` o `distil-whisper` / `large-v3-turbo` cuantizado).
- **Compilación cruzada en CI:** `whisper-rs` requiere toolchain C/C++ → Mitigación: evaluar primero `candle` en Rust puro y mantener el adaptador detrás de un feature flag si hace falta.
