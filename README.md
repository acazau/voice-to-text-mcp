# Voice-to-Text MCP Server

A Model Context Protocol (MCP) server for voice-to-text transcription using Rust and OpenAI's Whisper with hardware acceleration support for macOS (Metal/CoreML), Linux (CUDA), and Windows (CUDA).

## Features

- **Full MCP Server Implementation** - JSON-RPC 2.0 compliant server
- **Hardware Acceleration** - Platform-specific GPU acceleration:
  - macOS: Metal GPU + CoreML (Apple Neural Engine) on Apple Silicon, Metal on Intel
  - Linux/Windows: CUDA GPU acceleration for NVIDIA GPUs
  - Automatic CPU fallback on all platforms
- **Real-time Audio Capture** - Live microphone recording
- **File Transcription** - Process existing WAV files
- **Cross-platform Support** - Works on Linux, macOS, and Windows
- **Voice Command Recognition** - Automatic voice command detection during recording
- **Debug Mode** - Save audio files for troubleshooting

## Current Status

✅ **Completed:**
- Full MCP server implementation with stdio transport
- Hardware-accelerated Whisper transcription (Metal/CoreML on macOS, CUDA on Linux/Windows)
- Real-time audio capture and processing
- File-based audio transcription
- Comprehensive command-line interface
- Debug mode with audio file saving
- Complete test suite
- Voice command recognition for automatic stop triggers

## Dependencies

- `rmcp` - Model Context Protocol implementation
- `whisper-rs` - Rust bindings for OpenAI Whisper (with Metal/CoreML/CUDA support)
- `cpal` - Cross-platform audio I/O
- `tokio` - Async runtime
- `serde` - JSON serialization
- `anyhow` - Error handling

## Building

```bash
# Standard build
cargo build --release

# Note: First build with hardware acceleration takes longer:
# - CUDA (Linux/Windows): 6+ minutes due to whisper-rs-sys compilation
# - Metal/CoreML (macOS): 2-3 minutes
# Subsequent builds are much faster
```

## Usage

### MCP Server Mode

Run as an MCP server for integration with MCP clients:

```bash
# Run as MCP server with Whisper model
./target/release/voice-to-text-mcp --mcp-server models/ggml-base.en.bin

# Run as MCP server without model (placeholder mode)
./target/release/voice-to-text-mcp --mcp-server
```

**Available MCP Tools:**
- `transcribe_file` - Transcribe an audio file to text
- `listen` - Unified voice control with configurable commands (start/stop/status/toggle)
  - Optional `enable_voice_commands` parameter for runtime voice command control

### Interactive CLI Mode

Run in interactive mode for testing and development:

```bash
# Download models to models/ directory
cd models
wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin
cd ..

# Run with the model
./target/release/voice-to-text-mcp models/ggml-base.en.bin

# Run without model (placeholder mode)
./target/release/voice-to-text-mcp

# See all available options
./target/release/voice-to-text-mcp --help
```

**Available CLI Commands:**
- `start` - Begin microphone recording
- `stop` - Stop recording and get transcription
- `test <wav_file>` - Test transcription on existing WAV file
- `status` - Check recording status and sample count
- `quit` - Exit the application

### Debug Mode
Enable debug mode to save WAV files for troubleshooting:

```bash
# Using environment variable
VOICE_DEBUG=true ./target/release/voice-to-text-mcp models/ggml-base.en.bin

# Using command line flag
./target/release/voice-to-text-mcp --debug models/ggml-base.en.bin

# Custom debug directory
./target/release/voice-to-text-mcp --debug --debug-dir ./my_debug_folder models/ggml-base.en.bin

# MCP server with debug mode
./target/release/voice-to-text-mcp --mcp-server --debug models/ggml-base.en.bin

# Control what gets saved
./target/release/voice-to-text-mcp --debug --save-raw --save-processed models/ggml-base.en.bin
```

**Debug Features:**
- Saves raw captured audio as `audio_YYYYMMDD_HHMMSS_raw.wav`
- Saves processed audio as `audio_YYYYMMDD_HHMMSS_processed.wav`
- Automatic debug directory creation
- Timestamp-based file naming
- Helpful for troubleshooting audio issues and Whisper input validation

### Voice Command Recognition

Enable automatic voice command detection during recording sessions:

```bash
# Enable voice commands with default settings
./target/release/voice-to-text-mcp --voice-commands models/ggml-base.en.bin

# Customize voice command settings
./target/release/voice-to-text-mcp --voice-commands \
  --voice-chunk-duration 2000 \
  --voice-sensitivity 0.8 \
  --include-voice-commands \
  models/ggml-base.en.bin

# Enable for MCP server mode
./target/release/voice-to-text-mcp --mcp-server --voice-commands models/ggml-base.en.bin
```

**Voice Command Features:**
- **Enabled by Default**: Voice commands are automatically enabled for better user experience
- **Real-time Detection**: Automatically detects spoken commands during recording
- **Automatic Stop**: Say "stop", "stop recording", or "end" to automatically stop recording
- **Configurable Sensitivity**: Adjust detection sensitivity (0.0-1.0)
- **Customizable Timing**: Control audio chunk processing duration
- **Transcription Control**: Choose whether to include voice commands in final transcription
- **MCP Integration**: Enable/disable voice commands per MCP session

**Environment Variables:**
```bash
export VOICE_COMMANDS_ENABLED=true
export VOICE_CHUNK_DURATION=1500     # milliseconds
export VOICE_SENSITIVITY=0.7         # 0.0-1.0
export VOICE_INCLUDE_COMMANDS=false  # include in transcription
```

**Example Workflow:**
```bash
# Start recording with voice commands enabled
./target/release/voice-to-text-mcp --voice-commands models/ggml-base.en.bin

# In CLI: type 'start' or use voice command
> start
Started listening with voice commands enabled...

# Speak your content, then say "stop recording" to automatically end
# The system will detect the voice command and stop recording
```

### Model Download

Use our interactive download script (recommended):
```bash
./scripts/download-models.sh
```

The script provides:
- 🎯 Interactive menu with model recommendations by use case
- 📊 Model sizes, descriptions, and performance info
- ✅ Automatic detection of existing models (avoids re-downloading)
- 🔄 Resume capability for interrupted downloads
- 💾 Disk space validation before downloading
- 🌈 User-friendly colorized interface

**Quick recommendations:**
- **Development**: `ggml-tiny.en.bin` (75MB) - Fastest for testing
- **Most users**: `ggml-base.en.bin` (142MB) - Best balance ⭐
- **High quality**: `ggml-small.en.bin` (466MB) - Better accuracy
- **Multilingual**: `ggml-base.bin` (142MB) - Good for non-English

**Manual download alternative:**
```bash
cd models/
wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin
```

## Testing

Run the full test suite:

```bash
cargo test
```

The project includes:
- **Unit Tests** (20 tests) - Core functionality and hardware acceleration testing
- **Integration Tests** (5 tests) - End-to-end workflow and acceleration performance testing  
- **Property-Based Tests** (2 tests) - Randomized input validation
- **Voice Command Tests** (12 tests) - Voice command detection, configuration, and MCP integration
- **MCP Interface Tests** (13 tests) - Complete MCP protocol testing with configurable commands

### Check Hardware Acceleration

To verify your platform's acceleration configuration:

```bash
# Check platform detection and acceleration features
cargo test test_hardware_acceleration_runtime_info -- --nocapture

# Run acceleration integration tests
cargo test test_hardware_acceleration -- --nocapture
```

Test coverage includes:
- Service creation and state management
- Audio capture and processing
- Recording workflow (start/stop cycles)
- Concurrent operations
- Edge cases and error conditions
- **Whisper model loading and transcription**
- **Audio normalization and preprocessing**
- **Debug configuration and WAV file saving**
- **Timestamp-based file naming**
- **Voice command detection and configuration**
- **MCP voice command integration**

## MCP Integration

This server can be integrated with any MCP-compatible client.

### Claude Code Integration

**Add the server to your project:**
```bash
# Build the project first
cargo build --release

# Add to Claude Code (project scope)
claude mcp add --scope project voice-to-text -- ./target/release/voice-to-text-mcp --mcp-server models/ggml-base.en.bin

# Add to Claude Code (user scope - available across all projects)
claude mcp add --scope user voice-to-text -- /full/path/to/target/release/voice-to-text-mcp --mcp-server /full/path/to/models/ggml-base.en.bin

# Verify configuration
claude mcp list
```

**Quick Voice Recording Shortcuts:**
This project includes custom Claude Code slash commands for easy voice recording:

- **`/rc`** - Begin voice recording
- **`/st`** - Stop recording and get transcription
- **`/cr`** - Check current recording state
- **`/tr [path]`** - Transcribe an existing WAV file

**Example workflow:**
```bash
/rc            # Start recording
# [Speak your message] 
/st            # Get transcription
/cr            # Check status anytime
```

The slash commands are automatically available when you open this project in Claude Code.

## Configurable Voice Commands

Voice commands can be customized via CLI arguments or environment variables:

### CLI Arguments
```bash
# Custom start/stop commands
./target/release/voice-to-text-mcp --mcp-server models/ggml-base.en.bin \
  --start-commands "go,begin,record" \
  --stop-commands "halt,finish,done"

# Multilingual commands (Spanish/French)  
./target/release/voice-to-text-mcp --mcp-server models/ggml-base.en.bin \
  --start-commands "start,iniciar,commencer" \
  --stop-commands "stop,parar,arrêter" \
  --status-commands "status,estado,statut"

# Personal preference commands
./target/release/voice-to-text-mcp --mcp-server models/ggml-base.en.bin \
  --start-commands "begin,record" \
  --stop-commands "end,transcribe" \
  --toggle-commands "switch,toggle"
```

### Environment Variables (Fallback)
```bash
export VOICE_START_COMMANDS="start,begin,go"
export VOICE_STOP_COMMANDS="stop,end,done"
export VOICE_STATUS_COMMANDS="status,check,info"
export VOICE_TOGGLE_COMMANDS="toggle,switch,"

# Then run without CLI args
./target/release/voice-to-text-mcp --mcp-server models/ggml-base.en.bin
```

### Configuration Priority
1. CLI arguments (highest priority)
2. Environment variables 
3. Default commands: `start,begin,record` / `stop,end,finish` / `status,check,info` / `toggle,switch`

**Features:**
- Commands are case-insensitive
- Whitespace is automatically trimmed
- Multiple aliases per command type
- Backward compatible with original commands
- Easy MCP integration via CLI args

### MCP Configuration Examples

**Basic setup (`.mcp.json`):**
```json
{
  "mcpServers": {
    "voice-to-text": {
      "command": "./target/release/voice-to-text-mcp",
      "args": ["--mcp-server", "models/ggml-base.en.bin"]
    }
  }
}
```

**Custom commands:**
```json
{
  "mcpServers": {
    "voice-to-text": {
      "command": "./target/release/voice-to-text-mcp",
      "args": [
        "--mcp-server", 
        "models/ggml-base.en.bin",
        "--start-commands", "go,begin,record",
        "--stop-commands", "halt,finish,done", 
        "--status-commands", "check,info"
      ]
    }
  }
}
```

**Multilingual setup (Spanish/French):**
```json
{
  "mcpServers": {
    "voice-to-text": {
      "command": "./target/release/voice-to-text-mcp",
      "args": [
        "--mcp-server",
        "models/ggml-base.en.bin",
        "--start-commands", "start,iniciar,commencer",
        "--stop-commands", "stop,parar,arrêter",
        "--status-commands", "status,estado,statut"
      ]
    }
  }
}
```

**Claude Desktop Integration:**
```json
{
  "mcpServers": {
    "voice-to-text": {
      "command": "/full/path/to/target/release/voice-to-text-mcp",
      "args": [
        "--mcp-server", 
        "/full/path/to/models/ggml-base.en.bin",
        "--start-commands", "record,begin",
        "--stop-commands", "transcribe,done"
      ]
    }
  }
}
```

### Example MCP Tool Calls
```json
// Transcribe a file
{
  "method": "tools/call",
  "params": {
    "name": "transcribe_file",
    "arguments": {
      "file_path": "debug/audio_20250710_194139_raw.wav"
    }
  }
}

// Start recording
{
  "method": "tools/call", 
  "params": {
    "name": "listen",
    "arguments": {
      "command": "start"
    }
  }
}

// Start recording (voice commands are enabled by default)
{
  "method": "tools/call", 
  "params": {
    "name": "listen",
    "arguments": {
      "command": "start"
    }
  }
}

// Optional: Disable voice commands for a specific session
{
  "method": "tools/call", 
  "params": {
    "name": "listen",
    "arguments": {
      "command": "start",
      "enable_voice_commands": false
    }
  }
}

// Stop recording and get transcription
{
  "method": "tools/call",
  "params": {
    "name": "listen", 
    "arguments": {
      "command": "stop"
    }
  }
}

// Check recording status
{
  "method": "tools/call",
  "params": {
    "name": "listen",
    "arguments": {
      "command": "status"
    }
  }
}

// Toggle recording (start if stopped, stop if started)
{
  "method": "tools/call",
  "params": {
    "name": "listen",
    "arguments": {
      "command": ""
    }
  }
}
```

## Development

The implementation provides a complete voice-to-text MCP server with advanced voice command recognition. Future enhancements could include:

1. **Audio Format Support** - Support for MP3, OGG, and other formats
2. **Streaming Transcription** - Real-time transcription as audio is captured
3. **Multi-language Models** - Automatic language detection
4. **Configuration API** - Runtime configuration of audio devices and models
5. **Advanced Voice Commands** - Support for more complex voice interactions and custom phrases
6. **Voice Command Training** - Custom voice command recognition training

## System Requirements

### Required
- Rust 1.70+
- Audio input device (microphone)
- On Linux: ALSA development libraries (`libasound2-dev` on Ubuntu/Debian)

### Hardware Acceleration (Optional)

#### macOS
- **Apple Silicon (M1/M2/M3)**: Automatic Metal GPU + CoreML (Apple Neural Engine) acceleration
- **Intel Mac**: Automatic Metal GPU acceleration
- No additional installation required - uses built-in macOS frameworks

#### Linux/Windows
- **NVIDIA GPU** with CUDA support
- **CUDA Toolkit** 11.0+ installed

#### All Platforms
- If hardware acceleration is not available, the system automatically falls back to CPU processing
- CPU fallback provides the same functionality but slower transcription speed

### Installation Notes

#### Build Times
- First build with hardware acceleration takes longer:
  - **CUDA** (Linux/Windows): 6+ minutes
  - **Metal/CoreML** (macOS): 2-3 minutes
- Subsequent builds are much faster

#### Performance Notes
- **macOS Apple Silicon**: Up to 3x faster with CoreML, 2-3x faster with NEON SIMD
- **macOS Intel**: 1.5-2x faster with Metal GPU acceleration
- **Linux/Windows**: 2-4x faster with CUDA GPU acceleration
- **CoreML Note**: First run takes 15-20 minutes for model compilation, then cached for future use

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   MCP Client    │───▶│   MCP Server    │───▶│ Whisper Engine  │
│ (Claude, VSCode)│    │   (JSON-RPC)    │    │ (CUDA/CPU)      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                │
                                ▼
                       ┌─────────────────┐
                       │ Audio Capture   │
                       │ & Processing    │
                       │     (cpal)      │
                       └─────────────────┘
```

### Components
- **MCP Server**: JSON-RPC 2.0 server with stdio transport
- **Whisper Engine**: Hardware-accelerated speech recognition (Metal/CoreML/CUDA) with CPU fallback
- **Audio Pipeline**: Real-time capture, resampling, and preprocessing
- **Debug System**: Audio file saving and analysis tools
- **Model Downloader**: Interactive script for easy Whisper model management (`scripts/download-models.sh`)

### Project Structure
```
voice-to-text-mcp/
├── src/                     # Rust source code
│   ├── lib.rs              # Core VoiceToTextService
│   ├── mcp_server.rs       # MCP protocol implementation
│   └── main.rs             # CLI entry point
├── scripts/                # Utility scripts
│   └── download-models.sh  # Interactive model downloader
├── models/                 # Whisper model files (downloaded)
├── tests/                  # Test suites
└── target/                 # Build artifacts
```

## License

This project is open source. Please refer to the license file for details.