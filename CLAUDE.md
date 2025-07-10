# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a **Voice-to-Text MCP Server** that provides speech-to-text transcription capabilities via the Model Context Protocol (MCP). The project has two operational modes:

1. **MCP Server Mode**: JSON-RPC 2.0 compliant server for integration with MCP clients
2. **Interactive CLI Mode**: Command-line interface for testing and development

## Core Architecture

### Key Components

- **`VoiceToTextService`** (`src/lib.rs`): Core service handling audio capture, processing, and Whisper transcription
- **`VoiceToTextMcpServer`** (`src/mcp_server.rs`): MCP protocol implementation with JSON-RPC message handling
- **`main.rs`**: CLI argument parsing and application entry point with dual-mode support

### Critical Design Patterns

- **Thread-Safe State Management**: Uses `Arc<AtomicBool>` for recording state and `Arc<Mutex<Vec<f32>>>` for audio data
- **Async/Await Throughout**: All audio operations and transcription are async
- **CUDA with CPU Fallback**: whisper-rs compiled with CUDA support but gracefully falls back to CPU
- **Debug Mode**: Configurable audio file saving for troubleshooting with timestamp-based naming

### Audio Pipeline

1. **Capture**: `cpal` captures audio from default input device
2. **Processing**: Convert stereo→mono, resample 44.1kHz→16kHz, normalize amplitude
3. **Transcription**: Whisper processes 16kHz mono float32 audio
4. **Debug**: Optionally save raw and processed audio as WAV files

## Essential Commands

### Building
```bash
# Standard build (first CUDA build takes 6+ minutes)
cargo build --release

# Development build
cargo build
```

### Testing
```bash
# Run all tests (unit + integration + property-based)
cargo test

# Run specific test
cargo test test_service_creation

# Run tests with output
cargo test -- --nocapture

# Run only integration tests
cargo test --test integration_tests

# Run only property-based tests  
cargo test --test property_tests
```

### Running

#### MCP Server Mode
```bash
# With Whisper model
./target/release/voice-to-text-mcp --mcp-server ggml-base.en.bin

# Without model (placeholder mode)
./target/release/voice-to-text-mcp --mcp-server
```

#### Interactive CLI Mode
```bash
# With model
./target/release/voice-to-text-mcp ggml-base.en.bin

# With debug mode
./target/release/voice-to-text-mcp --debug ggml-base.en.bin

# Test existing WAV file
./target/release/voice-to-text-mcp ggml-base.en.bin
# Then use: test debug/audio_20250710_194139_raw.wav
```

### Development Workflow

#### Debug Mode
Enable to save audio files for analysis:
```bash
# Environment variable
VOICE_DEBUG=true ./target/release/voice-to-text-mcp ggml-base.en.bin

# Command line flag  
./target/release/voice-to-text-mcp --debug --debug-dir ./my_debug ggml-base.en.bin
```

#### Testing Audio Pipeline
1. Run with debug mode enabled
2. Use `start` and `stop` commands to capture audio
3. Check saved WAV files in debug directory
4. Use `test <wav_file>` command to test transcription on saved files

## MCP Protocol Implementation

The server implements these MCP tools:
- `transcribe_file`: Process WAV files
- `start_recording`: Begin live audio capture
- `stop_recording`: Stop recording and get transcription
- `get_recording_status`: Check current recording state

### MCP Message Flow
1. Client sends JSON-RPC requests via stdio
2. Server parses `tools/call` messages
3. Executes corresponding async methods on `VoiceToTextMcpServer`
4. Returns structured responses with success/error status

## Key Configuration

### Whisper Models
Download from: https://huggingface.co/ggerganov/whisper.cpp
- `ggml-base.en.bin`: Recommended for development (good speed/accuracy balance)
- `ggml-tiny.en.bin`: Fastest for testing
- `ggml-small.en.bin`: Better accuracy

### CUDA Support
- Enabled via `whisper-rs = { version = "0.14.3", features = ["cuda"] }`
- Automatically falls back to CPU if CUDA unavailable
- No runtime configuration needed

### Debug Configuration
- `DebugConfig` struct controls audio file saving
- Supports environment variables (`VOICE_DEBUG`, `VOICE_DEBUG_DIR`)
- Files saved with timestamp format: `audio_YYYYMMDD_HHMMSS_{raw|processed}.wav`

## Important Implementation Notes

### Audio Processing
- Input: Any sample rate/channels via `cpal`
- Processing: Convert to 16kHz mono float32 for Whisper
- Whisper expects normalized audio in [-1.0, 1.0] range

### Error Handling
- `anyhow::Result` used throughout for error propagation
- MCP responses include structured success/error fields
- Audio device failures handled gracefully

### Testing Strategy
- **Unit tests**: Core functionality, state management, edge cases
- **Integration tests**: End-to-end workflows, recording cycles
- **Property-based tests**: Randomized input validation with `proptest`

### Concurrency
- Service is `Clone` but shares state via `Arc<Mutex<_>>`
- Only one recording session active at a time (enforced by `AtomicBool`)
- MCP server wraps service in `Arc<Mutex<_>>` for async safety