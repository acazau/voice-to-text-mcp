# Whisper Models Directory

This directory contains downloaded Whisper models for the Voice-to-Text MCP server.

## Quick Start

Use our interactive download script (recommended):
```bash
./scripts/download-models.sh
```

## Available Models

| Model | Size | Language | Use Case | Description |
|-------|------|----------|----------|-------------|
| `ggml-tiny.en.bin` | 75MB | English | Development | Fastest inference, good for testing |
| `ggml-base.en.bin` | 142MB | English | **Recommended** | Best balance of speed and accuracy ⭐ |
| `ggml-small.en.bin` | 466MB | English | High Quality | Better accuracy, slower inference |
| `ggml-tiny.bin` | 75MB | Multilingual | Development | Fastest multilingual inference |
| `ggml-base.bin` | 142MB | Multilingual | Recommended | Good multilingual balance |
| `ggml-small.bin` | 466MB | Multilingual | High Quality | Better multilingual accuracy |
| `ggml-medium.en.bin` | 1.5GB | English | Production | High accuracy for production use |
| `ggml-medium.bin` | 1.5GB | Multilingual | Production | High accuracy multilingual |
| `ggml-large-v3.bin` | 2.9GB | Multilingual | Enterprise | Highest accuracy available |

## Usage Examples

### MCP Server Mode
```bash
# Recommended model for most users
./target/release/voice-to-text-mcp --mcp-server models/ggml-base.en.bin

# High quality transcription
./target/release/voice-to-text-mcp --mcp-server models/ggml-small.en.bin

# Multilingual support
./target/release/voice-to-text-mcp --mcp-server models/ggml-base.bin
```

### Interactive CLI Mode
```bash
# Test with recommended model
./target/release/voice-to-text-mcp models/ggml-base.en.bin

# Development testing with fast model
./target/release/voice-to-text-mcp models/ggml-tiny.en.bin
```

### Claude Code Integration
```bash
# Add to Claude Code with recommended model
claude mcp add --scope project voice-to-text -- ./target/release/voice-to-text-mcp --mcp-server models/ggml-base.en.bin
```

## Model Selection Guide

### For English Transcription
- **Development/Testing**: `ggml-tiny.en.bin` - Fast and lightweight
- **General Use**: `ggml-base.en.bin` - Recommended balance ⭐
- **High Quality**: `ggml-small.en.bin` - Better accuracy
- **Production**: `ggml-medium.en.bin` - Professional quality

### For Multilingual Transcription
- **Development**: `ggml-tiny.bin` - Fast multilingual
- **General Use**: `ggml-base.bin` - Good multilingual balance
- **High Quality**: `ggml-small.bin` - Better multilingual accuracy
- **Enterprise**: `ggml-large-v3.bin` - Best multilingual quality

## Hardware Acceleration

Models automatically benefit from platform-specific acceleration:
- **macOS Apple Silicon**: Metal GPU + CoreML (Apple Neural Engine) + NEON
- **macOS Intel**: Metal GPU acceleration
- **Linux/Windows**: CUDA GPU acceleration (with compatible hardware)
- **All Platforms**: Automatic CPU fallback

## Manual Download (Alternative)

If you prefer manual downloads:
```bash
cd models/
wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin
wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin
```

**Note:** The interactive script (`./scripts/download-models.sh`) is recommended as it provides better error handling, progress indicators, and model recommendations.

## Model Sources

- **Source**: https://huggingface.co/ggerganov/whisper.cpp
- **License**: MIT License by OpenAI
- **Format**: GGML format optimized for whisper.cpp