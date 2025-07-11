# Voice-to-Text MCP Server

A Model Context Protocol (MCP) server for voice-to-text transcription using Rust and OpenAI's Whisper with CUDA acceleration.

## Features

- **Full MCP Server Implementation** - JSON-RPC 2.0 compliant server
- **CUDA Acceleration** - GPU-accelerated Whisper transcription (with CPU fallback)
- **Real-time Audio Capture** - Live microphone recording
- **File Transcription** - Process existing WAV files
- **Cross-platform Support** - Works on Linux, macOS, and Windows
- **Debug Mode** - Save audio files for troubleshooting

## Current Status

✅ **Completed:**
- Full MCP server implementation with stdio transport
- CUDA-accelerated Whisper transcription (with CPU fallback)
- Real-time audio capture and processing
- File-based audio transcription
- Comprehensive command-line interface
- Debug mode with audio file saving
- Complete test suite

## Dependencies

- `rmcp` - Model Context Protocol implementation
- `whisper-rs` - Rust bindings for OpenAI Whisper (with CUDA support)
- `cpal` - Cross-platform audio I/O
- `tokio` - Async runtime
- `serde` - JSON serialization
- `anyhow` - Error handling

## Building

```bash
# Standard build
cargo build --release

# First build with CUDA will take 6+ minutes due to whisper-rs-sys compilation
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
- `start_recording` - Begin live audio recording  
- `stop_recording` - Stop recording and get transcription
- `get_recording_status` - Get current recording status

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

### Model Download
Download Whisper models from: https://huggingface.co/ggerganov/whisper.cpp

Recommended models:
- `ggml-tiny.en.bin` - Fastest, English only
- `ggml-base.en.bin` - Good balance of speed/accuracy, English only  
- `ggml-small.en.bin` - Better accuracy, English only
- `ggml-base.bin` - Multilingual support

## Testing

Run the full test suite:

```bash
cargo test
```

The project includes:
- **Unit Tests** (15 tests) - Core functionality testing
- **Integration Tests** (3 tests) - End-to-end workflow testing  
- **Property-Based Tests** (2 tests) - Randomized input validation

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

**Project-level configuration (`.mcp.json`):**
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

### Claude Desktop Integration
```json
{
  "mcpServers": {
    "voice-to-text": {
      "command": "/full/path/to/target/release/voice-to-text-mcp",
      "args": ["--mcp-server", "/full/path/to/models/ggml-base.en.bin"]
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
    "name": "start_recording",
    "arguments": {}
  }
}
```

## Development

The implementation provides a complete voice-to-text MCP server. Future enhancements could include:

1. **Audio Format Support** - Support for MP3, OGG, and other formats
2. **Streaming Transcription** - Real-time transcription as audio is captured
3. **Multi-language Models** - Automatic language detection
4. **Configuration API** - Runtime configuration of audio devices and models

## System Requirements

### Required
- Rust 1.70+
- Audio input device (microphone)
- On Linux: ALSA development libraries (`libasound2-dev` on Ubuntu/Debian)

### CUDA Support (Optional)
- **NVIDIA GPU** with CUDA support
- **CUDA Toolkit** 11.0+ installed
- If CUDA is not available, the system automatically falls back to CPU processing
- CPU fallback provides the same functionality but slower transcription speed

### Installation Notes
- First build with CUDA takes 6+ minutes due to compiling whisper.cpp with CUDA support
- Without CUDA: faster build times, slower transcription
- With CUDA: longer build times, faster transcription

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
- **Whisper Engine**: CUDA-accelerated speech recognition with CPU fallback
- **Audio Pipeline**: Real-time capture, resampling, and preprocessing
- **Debug System**: Audio file saving and analysis tools

## License

This project is open source. Please refer to the license file for details.