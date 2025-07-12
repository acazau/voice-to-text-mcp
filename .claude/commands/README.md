# Voice Recording Commands for Claude Code

This directory contains custom slash commands for voice recording and transcription within Claude Code.

## Available Commands

### Voice Recording Commands
- **`/listen`** - Record audio and return transcribed text (blocking)
- **`/transcribe_file`** - Transcribe an existing audio file

## Quick Start

1. **Record Audio:**
   ```
   /listen
   ```
   *Speak your message, then wait for silence or timeout*

2. **Record with Custom Timeout:**
   ```
   /listen timeout_ms=60000
   ```

3. **Transcribe Existing File:**
   ```
   /transcribe_file file_path="debug/audio_20250710_194139_raw.wav"
   ```

## Command Examples

### Basic Recording
```bash
# Record with default settings (30s max, 2s silence timeout)
/listen

# Record for up to 60 seconds  
/listen timeout_ms=60000

# Wait 3 seconds of silence before stopping
/listen silence_timeout_ms=3000

# Disable auto-stop (record until timeout)
/listen auto_stop=false
```

### File Transcription
```bash
# Transcribe a debug audio file
/transcribe_file file_path="debug/audio_20250710_194139_raw.wav"

# Transcribe external file
/transcribe_file file_path="/path/to/meeting.wav"
```

## Architecture

**Simplified Blocking Design:**
- `/listen` calls `voice-recorder` binary which blocks until complete
- Returns transcribed text when recording finishes
- Process isolation ensures reliability
- Natural 30-second timeout handling

**Key Changes from Previous Version:**
- No more start/stop/status commands - just simple `/listen`
- Blocking operation returns final transcription
- Uses separate `voice-recorder` binary for actual recording

## Prerequisites

- Voice-to-text MCP server running with model file
- Microphone access enabled
- Whisper model file (e.g., `ggml-base.en.bin`)
- MCP server configured in Claude Desktop settings

## Configuration

Add to your Claude Desktop configuration:

```json
{
  "mcpServers": {
    "voice-to-text": {
      "command": "/path/to/target/release/voice-to-text-mcp",
      "args": ["--mcp-server", "/path/to/ggml-base.en.bin"]
    }
  }
}
```

## Troubleshooting

**Commands not appearing?**
- Restart Claude Desktop
- Check MCP server is configured correctly
- Verify binary paths are correct

**Recording not working?**
- Check microphone permissions
- Ensure Whisper model file exists
- Check MCP server logs

**No transcription returned?**
- Speak clearly for at least 1-2 seconds
- Minimize background noise
- Check if model file is loading correctly

## Technical Details

- **Audio Processing**: 44.1kHz → 16kHz mono for Whisper
- **Hardware Acceleration**: Metal/CoreML (macOS), CUDA (Linux/Windows)
- **Timeout**: Natural 30-second maximum per recording
- **Format Support**: WAV files for transcription
- **Debug Mode**: Set `VOICE_DEBUG=true` to save audio files