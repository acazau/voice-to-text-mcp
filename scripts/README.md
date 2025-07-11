# Scripts Directory

This directory contains utility scripts for the Voice-to-Text MCP project.

## Available Scripts

### `download-models.sh`
Interactive script to download Whisper models for speech-to-text transcription.

**Features:**
- 🎯 Interactive menu with model recommendations
- 📊 Shows model sizes, descriptions, and use cases
- ✅ Checks for existing models (avoids re-downloading)
- 🔄 Resume capability for interrupted downloads
- 💾 Disk space validation before downloading
- 🌈 Colorful, user-friendly interface

**Usage:**
```bash
# Run the interactive downloader
./scripts/download-models.sh
```

**Quick Downloads:**
The script categorizes models by use case:
- **Development**: `ggml-tiny.en.bin` (75MB) - Fast for testing
- **Recommended**: `ggml-base.en.bin` (142MB) - Best balance ⭐
- **High Quality**: `ggml-small.en.bin` (466MB) - Better accuracy
- **Multilingual**: Various international language support
- **Production**: Large models for production environments

**Requirements:**
- `wget` or `curl` for downloading
- Sufficient disk space (varies by model)
- Internet connection

## Future Scripts

This directory can be extended with additional utility scripts such as:
- Model benchmarking scripts
- Performance testing utilities
- Configuration helpers
- Development environment setup scripts

## Contributing

When adding new scripts:
1. Make them executable: `chmod +x scripts/script-name.sh`
2. Add clear documentation in this README
3. Include usage examples
4. Follow the existing naming convention
5. Add error handling and user-friendly output