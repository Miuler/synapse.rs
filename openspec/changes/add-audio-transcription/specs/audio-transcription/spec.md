## ADDED Requirements

### Requirement: Local audio-to-text transcription
The system SHALL transcribe audio files stored in the vault (such as `.wav`, `.mp3`, `.ogg`, `.m4a`) into plain text or Markdown locally without sending audio data to external cloud services.

#### Scenario: Transcribe an audio file in the vault
- **WHEN** the user triggers transcription on a supported audio file and a local model is available
- **THEN** the backend decodes the audio, runs inference locally, and returns the transcribed text

### Requirement: Pluggable neural engine and model
The transcription architecture MUST abstract the inference engine behind a domain/application port so the backend can use `candle` (pure Rust), `whisper-rs`, or a newer/better local speech-to-text model without changing use cases or UI.

#### Scenario: Swap inference backend
- **WHEN** the configured transcription engine is replaced or upgraded (e.g., from Whisper via `whisper-rs` to `candle` or a newer model)
- **THEN** the application use cases and frontend continue working without interface changes

### Requirement: On-demand model download
The system SHALL NOT bundle neural network weights inside the application binary; instead, it SHALL download the selected model on demand into the application data directory with integrity verification.

#### Scenario: First-time transcription without local model
- **WHEN** the user requests transcription and no model is present on disk
- **THEN** the system prompts or initiates an on-demand download of the configured model before running inference
