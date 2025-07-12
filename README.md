# Voice-to-Text MCP Server

A Model Context Protocol (MCP) server for voice-to-text transcription using Rust and OpenAI's Whisper with hardware acceleration support for macOS (Metal/CoreML), Linux (CUDA), and Windows (CUDA).

## Quick Start

```bash
# 1. Build the server
cargo build --release

# 2. Download a model
./scripts/download-models.sh

# 3. Add to Claude Code (project scope)
claude mcp add --scope project voice-to-text -- \
  ./target/release/voice-to-text-mcp --mcp-server models/ggml-base.en.bin

# 4. Use in Claude Code
/listen
```

## Table of Contents

- [Quick Start](#quick-start)
- [Features](#features)
- [Current Status](#current-status)
- [Dependencies](#dependencies)
- [Building](#building)
- [Usage](#usage)
  - [MCP Server Mode](#mcp-server-mode)
  - [Blocking CLI Mode](#blocking-cli-mode)
  - [Debug Mode](#debug-mode)
- [Model Download](#model-download)
- [Testing](#testing)
- [MCP Integration](#mcp-integration)
  - [Claude Code Integration](#claude-code-integration)
  - [Other MCP Clients](#other-mcp-clients)
- [Development](#development)
- [System Requirements](#system-requirements)
- [Architecture](#architecture)
- [License](#license)

## Features

- **Full MCP Server Implementation** - JSON-RPC 2.0 compliant server
- **Quiet Operation** - Clean output by default for MCP clients (use `--debug` for verbose logging)
- **Hardware Acceleration** - Platform-specific GPU acceleration:
  - macOS: Metal GPU + CoreML (Apple Neural Engine) on Apple Silicon, Metal on Intel
  - Linux/Windows: CUDA GPU acceleration for NVIDIA GPUs
  - Automatic CPU fallback on all platforms
- **Real-time Audio Capture** - Live microphone recording
- **File Transcription** - Process existing WAV files
- **Cross-platform Support** - Works on Linux, macOS, and Windows
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
- **Modular architecture** with separated concerns (audio, whisper, config, platform)
- **Structured error handling** with `thiserror` for better debugging
- **Enhanced safety** with `gag` crate replacing unsafe code

## Dependencies

- `rmcp` - Model Context Protocol implementation
- `whisper-rs` - Rust bindings for OpenAI Whisper (with Metal/CoreML/CUDA support)
- `cpal` - Cross-platform audio I/O
- `crossterm` - Cross-platform terminal manipulation (legacy keyboard controls)
- `tokio` - Async runtime
- `serde` - JSON serialization
- `anyhow` - Error handling
- `thiserror` - Structured error types
- `gag` - Safe output suppression

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
- `listen` - Voice recording with configurable timeout and auto-stop parameters

### Blocking CLI Mode

Run in blocking mode for single recording operations:

```bash
# Download models to models/ directory
cd models
wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin
cd ..

# Record audio with default settings (30s max, 2s silence timeout)
./target/release/voice-to-text-mcp models/ggml-base.en.bin

# Record with custom timeouts
./target/release/voice-to-text-mcp --timeout-ms 60000 --silence-timeout-ms 3000 models/ggml-base.en.bin

# Record without auto-stop (record for full timeout)
./target/release/voice-to-text-mcp --no-auto-stop --timeout-ms 10000 models/ggml-base.en.bin

# See all available options
./target/release/voice-to-text-mcp --help
```

**CLI Behavior:**
- Records audio from microphone immediately on startup
- Automatically stops after silence or timeout
- Returns transcribed text and exits
- Supports debug mode with `--debug` flag

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
- **Unit Tests** (17 tests) - Core functionality and hardware acceleration testing
- **Integration Tests** (5 tests) - End-to-end workflow and acceleration performance testing  
- **Property-Based Tests** (2 tests) - Randomized input validation
- **MCP Interface Tests** (14 tests) - Complete MCP protocol testing with comprehensive coverage

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

## MCP Integration

This server can be integrated with any MCP-compatible client.

### Claude Code Integration

The voice-to-text MCP server integrates with Claude Code to provide speech-to-text transcription capabilities directly within your development workflow. This allows you to dictate code, comments, documentation, or any text content using your voice.

#### Prerequisites for Claude Code

1. **Claude Code CLI installed** with MCP support
2. **Built voice-to-text-mcp binary** (see Building section above)
3. **Whisper model downloaded** (see Model Download section above)
4. **Microphone** available on your system

#### Installation Options

<details>
<summary><strong>Option 1: Project-Level Installation (Recommended for Development)</strong></summary>

Install the MCP server for a specific project. This keeps the voice-to-text capability isolated to projects where you need it.

```bash
# From the voice-to-text-mcp repository directory
# Build the project first
cargo build --release

# Add to Claude Code for this project only
claude mcp add --scope project voice-to-text -- \
  ./target/release/voice-to-text-mcp --mcp-server models/ggml-base.en.bin

# Verify installation
claude mcp list --scope project
```

**Use Case**: Perfect for when you're working on this voice-to-text project or other projects where you frequently need to dictate code or documentation.
</details>

<details>
<summary><strong>Option 2: User-Level Installation (Global Access)</strong></summary>

Install the MCP server globally for your user account. This makes voice-to-text available across all Claude Code sessions.

```bash
# From the voice-to-text-mcp repository directory
# Get absolute paths for the binary and model
BINARY_PATH=$(pwd)/target/release/voice-to-text-mcp
MODEL_PATH=$(pwd)/models/ggml-base.en.bin

# Add to Claude Code globally
claude mcp add --scope user voice-to-text -- \
  "$BINARY_PATH" --mcp-server "$MODEL_PATH"

# Verify installation
claude mcp list --scope user
```

**Use Case**: Ideal when you want voice-to-text available in all your development projects and don't want to configure it per-project.
</details>

<details>
<summary><strong>Option 3: Development with Debug Mode</strong></summary>

For debugging or when working on the voice-to-text server itself:

```bash
# Project-level with debug enabled
claude mcp add --scope project voice-to-text-debug -- \
  ./target/release/voice-to-text-mcp --mcp-server --debug models/ggml-base.en.bin

# Check debug output
claude mcp list --scope project
```
</details>

#### Basic Usage in Claude Code

Once installed, use these commands in Claude Code:

```bash
# Record with default settings (30s max, 2s silence timeout)
/listen

# Record with custom timeout (60 seconds)
/listen timeout_ms=60000

# Record with longer silence timeout (5 seconds)
/listen silence_timeout_ms=5000

# Record for full duration without auto-stop
/listen timeout_ms=10000 auto_stop=false

# Transcribe an existing audio file
transcribe_file file_path="debug/audio_20250112_143022_raw.wav"
```

#### Practical Usage Scenarios

<details>
<summary><strong>Dictating Code</strong></summary>

```bash
# Record your voice describing the code you want
/listen

# Example transcription result:
# "Create a function called calculate total that takes a list of numbers and returns the sum"

# Then ask Claude to convert to code:
# Convert this to Python code: "Create a function called calculate total that takes a list of numbers and returns the sum"
```
</details>

<details>
<summary><strong>Code Documentation</strong></summary>

```bash
# Dictate function documentation
/listen timeout_ms=45000

# Example result:
# "This function implements a binary search algorithm. It takes a sorted array and a target value as parameters. Returns the index of the target if found, otherwise returns negative one."
```
</details>

<details>
<summary><strong>Commit Messages</strong></summary>

```bash
# Dictate commit messages
/listen timeout_ms=15000

# Example result:
# "Fix authentication bug in user login flow and add unit tests for edge cases"
```
</details>

<details>
<summary><strong>Issue Descriptions</strong></summary>

```bash
# Describe bugs or features
/listen timeout_ms=60000

# Example result:
# "When users click the submit button on the contact form, the page doesn't show any loading indicator and sometimes the form gets submitted multiple times causing duplicate entries in the database"
```
</details>

#### Workflow Integration Examples

<details>
<summary><strong>Voice-Driven Development Workflow</strong></summary>

```bash
# 1. Start with a voice description of what you want to build
/listen timeout_ms=45000
# Result: "I need to create a REST API endpoint that accepts user registration data validates the email format checks if the user already exists and saves the new user to the database"

# 2. Ask Claude to create the implementation
# Based on this description, create a Python Flask endpoint for user registration

# 3. Dictate test cases
/listen timeout_ms=30000
# Result: "Test with valid email, test with invalid email format, test with duplicate email, test with missing required fields"

# 4. Ask Claude to implement the tests
# Create pytest test cases for these scenarios
```
</details>

<details>
<summary><strong>Documentation Workflow</strong></summary>

```bash
# 1. Dictate API documentation
/listen timeout_ms=60000
# Result: "The get users endpoint returns a paginated list of users. It accepts optional query parameters page for page number limit for items per page and search for filtering by username or email"

# 2. Format as documentation
# Convert this to OpenAPI/Swagger documentation format

# 3. Add implementation notes
/listen timeout_ms=30000
# Result: "Note that this endpoint requires authentication and users can only see other users if they have admin role otherwise they only see their own profile"
```
</details>

#### Configuration Management

```bash
# View current configuration
claude mcp list

# List project-specific servers
claude mcp list --scope project

# List user-level servers
claude mcp list --scope user

# Remove existing configuration
claude mcp remove --scope project voice-to-text

# Update configuration
claude mcp add --scope project voice-to-text -- \
  ./target/release/voice-to-text-mcp --mcp-server --debug models/ggml-base.en.bin
```

#### Troubleshooting Claude Code Integration

<details>
<summary><strong>Common Issues and Solutions</strong></summary>

**MCP Server Not Found:**
```bash
# Error: "MCP server not found"
# Solution: Verify the binary path is correct
ls -la target/release/voice-to-text-mcp

# Re-add with correct path
claude mcp remove --scope project voice-to-text
claude mcp add --scope project voice-to-text -- \
  ./target/release/voice-to-text-mcp --mcp-server models/ggml-base.en.bin
```

**Model File Not Found:**
```bash
# Error: "Model file not found"
# Solution: Verify model exists and path is correct
ls -la models/ggml-base.en.bin

# Download model if missing
./scripts/download-models.sh
```

**Audio Device Issues:**
```bash
# Error: "No input device available"
# Solution: Check system audio settings and permissions

# Test with debug mode to see detailed errors
claude mcp remove --scope project voice-to-text
claude mcp add --scope project voice-to-text -- \
  ./target/release/voice-to-text-mcp --mcp-server --debug models/ggml-base.en.bin

# Try recording to see debug output
/listen
```

**Permission Issues:**
```bash
# Error: "Permission denied"
# Solution: Ensure binary is executable
chmod +x target/release/voice-to-text-mcp

# On macOS, you might need to allow the binary in Security & Privacy settings
```
</details>

#### Best Practices for Voice Input

- **Clear Speech**: Speak clearly and at a moderate pace in a quiet environment
- **Structured Dictation**: Organize your thoughts before recording
- **Provide Context**: Include context in your dictation (e.g., "For the Python Flask application...")
- **Iterative Refinement**: Start with basic structure, then add details and edge cases

#### Performance Optimization

**Model Selection by Use Case:**

| Model | Size | Speed | Accuracy | Best For |
|-------|------|-------|----------|----------|
| `ggml-tiny.en.bin` | 75MB | Fastest | Good | Quick notes, prototyping |
| `ggml-base.en.bin` | 142MB | Fast | Better | General development work |
| `ggml-small.en.bin` | 466MB | Slower | Best | Documentation, formal content |

**Recording Settings Optimization:**
```bash
# For quick commands (shorter timeout)
/listen timeout_ms=10000 silence_timeout_ms=1000

# For detailed explanations (longer timeout)
/listen timeout_ms=120000 silence_timeout_ms=3000

# For continuous dictation (no auto-stop)
/listen timeout_ms=300000 auto_stop=false
```

## Other MCP Clients

For integration with other MCP-compatible clients (like Claude Desktop), you can use standard MCP configuration:

**Basic `.mcp.json` setup:**
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

**With debug mode:**
```json
{
  "mcpServers": {
    "voice-to-text": {
      "command": "./target/release/voice-to-text-mcp",
      "args": ["--mcp-server", "--debug", "models/ggml-base.en.bin"]
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

// Record audio with default settings (30s max, 2s silence timeout, auto-stop enabled)
{
  "method": "tools/call", 
  "params": {
    "name": "listen",
    "arguments": {}
  }
}

// Record audio with custom timeout (60 seconds)
{
  "method": "tools/call", 
  "params": {
    "name": "listen",
    "arguments": {
      "timeout_ms": 60000
    }
  }
}

// Record audio with custom silence timeout (3 seconds)
{
  "method": "tools/call",
  "params": {
    "name": "listen", 
    "arguments": {
      "silence_timeout_ms": 3000
    }
  }
}

// Record for full timeout without auto-stop
{
  "method": "tools/call",
  "params": {
    "name": "listen",
    "arguments": {
      "timeout_ms": 10000,
      "auto_stop": false
    }
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

#### Platform Compatibility
- **macOS**: Uses platform-specific compatibility layer to handle CoreAudio threading constraints
- **Linux/Windows**: Standard threading model with CUDA acceleration
- All platforms maintain identical functionality and API

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
│   ├── lib.rs              # Main service coordination
│   ├── main.rs             # CLI entry point
│   ├── mcp_server.rs       # MCP protocol implementation
│   ├── platform_compat.rs  # Cross-platform compatibility layer
│   ├── audio.rs            # Audio capture and processing
│   ├── whisper.rs          # Whisper transcription logic
│   ├── config.rs           # Configuration and constants
│   ├── platform.rs         # Platform-specific implementations
│   ├── keyboard.rs         # Keyboard control functionality (legacy)
│   └── error.rs            # Structured error types
├── scripts/                # Utility scripts
│   └── download-models.sh  # Interactive model downloader
├── models/                 # Whisper model files (downloaded)
├── tests/                  # Test suites
└── target/                 # Build artifacts
```

## License

This project is open source. Please refer to the license file for details.