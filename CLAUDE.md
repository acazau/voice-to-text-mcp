# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a **Voice-to-Text MCP Server** that provides speech-to-text transcription capabilities via the Model Context Protocol (MCP). The project has two operational modes:

1. **MCP Server Mode**: JSON-RPC 2.0 compliant server for integration with MCP clients (with `--mcp-server` flag)
2. **Blocking Recording Mode**: Simple command-line tool that records audio and returns transcription (default behavior)

## Core Architecture

### Key Components

- **`voice-to-text-mcp`** (`src/main.rs`): Unified binary with MCP server and interactive CLI modes
- **`VoiceToTextService`** (`src/lib.rs`): Core service handling audio capture, processing, and Whisper transcription  
- **`VoiceToTextMcpServer`** (`src/mcp_server.rs`): MCP protocol implementation using VoiceToTextService directly

### Critical Design Patterns

- **Unified Binary Architecture**: Single binary handles both MCP server and CLI modes with consistent behavior
- **Direct Service Integration**: MCP server uses VoiceToTextService directly (no subprocess calls)
- **Configurable Timeouts**: Both CLI and MCP modes support `--timeout-ms` and `--silence-timeout-ms` parameters
- **Hardware Acceleration**: Platform-specific acceleration with automatic CPU fallback
  - **macOS Apple Silicon**: Metal GPU + CoreML (Apple Neural Engine)
  - **macOS Intel**: Metal GPU acceleration
  - **Linux/Windows x86_64**: CUDA GPU acceleration
- **Debug Mode**: Configurable audio file saving for troubleshooting with timestamp-based naming

### Audio Pipeline

1. **Capture**: `cpal` captures audio from default input device
2. **Processing**: Convert stereo→mono, resample 44.1kHz→16kHz, normalize amplitude
3. **Transcription**: Whisper processes 16kHz mono float32 audio
4. **Debug**: Optionally save raw and processed audio as WAV files

## Essential Commands

### Building
```bash
# Standard build
cargo build --release

# Development build
cargo build

# Note: First build with hardware acceleration takes longer:
# - CUDA (Linux/Windows): 6+ minutes
# - Metal/CoreML (macOS): 2-3 minutes
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
# With Whisper model (required for recording)
./target/release/voice-to-text-mcp --mcp-server ggml-base.en.bin
```

#### Blocking Voice Recording Mode (Default)
```bash
# Simple blocking recording with custom timeouts
./target/release/voice-to-text-mcp --timeout-ms 10000 --silence-timeout-ms 1000 ggml-base.en.bin

# With debug mode
./target/release/voice-to-text-mcp --debug --timeout-ms 5000 ggml-base.en.bin

# Disable auto-stop (record for full timeout)
./target/release/voice-to-text-mcp --no-auto-stop --timeout-ms 5000 ggml-base.en.bin
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
- `transcribe_file`: Process existing WAV files
- `listen`: Record audio and return transcribed text (blocking operation)

**Key Changes:**
- Simplified to single `listen` tool (no more start/stop/status commands)  
- Blocking operation - returns transcribed text when recording complete
- Uses VoiceToTextService directly (no subprocess calls)
- Configurable timeout handling via MCP parameters

### Keyboard Controls

The server supports keyboard shortcuts for controlling recording in interactive CLI mode. **Note**: Keyboard controls are disabled in MCP server mode since stdin/stdout are reserved for MCP protocol communication.

**Default Keyboard Shortcuts:**
- **Ctrl+R**: Start recording
- **Ctrl+S**: Stop recording  
- **Space**: Toggle recording (start if stopped, stop if recording)
- **Ctrl+I**: Show status information
- **Esc**: Exit keyboard controls

**Configuration Options:**
```bash
# Enable keyboard controls
./target/release/voice-to-text-mcp --keyboard-controls ggml-base.en.bin

# Custom key bindings
./target/release/voice-to-text-mcp --keyboard-controls \
  --keyboard-start-key "r" \
  --keyboard-stop-key "s" \
  --keyboard-toggle-key " " \
  --keyboard-status-key "i" \
  ggml-base.en.bin

# Disable Ctrl modifier requirement (except for toggle key)
./target/release/voice-to-text-mcp --keyboard-controls --keyboard-no-ctrl ggml-base.en.bin
```

**Environment Variables:**
- `KEYBOARD_CONTROLS_ENABLED=true` - Enable keyboard controls
- Works alongside all other voice and debug configurations

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

### Hardware Acceleration Support

#### macOS Apple Silicon (ARM64)
- **Metal**: GPU acceleration via Apple's Metal framework
- **CoreML**: Apple Neural Engine (ANE) acceleration (3x+ speedup)
- **NEON**: ARM64 SIMD instructions (automatic, 2-3x speedup over x86)
- **Important**: First CoreML run takes 15-20 minutes (model compilation)
- Enabled via `whisper-rs = { version = "0.14.3", features = ["metal", "coreml"] }`

#### macOS Intel (x86_64)
- **Metal**: GPU acceleration via Apple's Metal framework
- Enabled via `whisper-rs = { version = "0.14.3", features = ["metal"] }`

#### Linux/Windows (x86_64)
- **CUDA**: NVIDIA GPU acceleration
- Enabled via `whisper-rs = { version = "0.14.3", features = ["cuda"] }`

All platforms automatically fall back to CPU if acceleration is unavailable.

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