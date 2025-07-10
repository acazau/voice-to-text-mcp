# Whisper Models

This directory contains Whisper model files for voice-to-text transcription.

## Download Models

Download models from: https://huggingface.co/ggerganov/whisper.cpp

### Recommended Models

```bash
# Base model (recommended for development)
wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin

# Tiny model (fastest)
wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin

# Small model (better accuracy)
wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin

# Multilingual base model
wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin
```

### Usage

```bash
# Run with model
./target/release/voice-to-text-mcp models/ggml-base.en.bin

# MCP server mode
./target/release/voice-to-text-mcp --mcp-server models/ggml-base.en.bin
```

## Model Comparison

| Model | Size | Speed | Accuracy | Languages |
|-------|------|-------|----------|-----------|
| ggml-tiny.en.bin | ~39MB | Fastest | Basic | English only |
| ggml-base.en.bin | ~142MB | Fast | Good | English only |
| ggml-small.en.bin | ~244MB | Medium | Better | English only |
| ggml-base.bin | ~142MB | Fast | Good | Multilingual |

**Note:** Model files are ignored by git due to their large size. Download them locally as needed.