# Voice Recording Commands for Claude Code

This directory contains custom slash commands for easy voice recording and transcription within Claude Code.

## Available Commands

### Voice Recording Commands
- **`/listen [command]`** - Unified voice control (start/stop/status/toggle)
- **`/transcribe [path]`** - Transcribe an existing WAV file

## Quick Start

1. **Start Recording:**
   ```
   /listen start
   ```

2. **Speak your message** (at least 1-2 seconds)

3. **Stop Recording:**
   ```
   /listen stop
   ```

4. **Check Status anytime:**
   ```
   /listen status
   ```

5. **Toggle Recording:**
   ```
   /listen
   ```

## Workflow Example

```bash
# Check if anything is recording
/listen status

# Start a new recording
/listen start

# [Speak your message]

# Stop and get transcription
/listen stop

# Transcribe an existing file
/transcribe debug/audio_20250710_194139_raw.wav
```

## Prerequisites

- Voice-to-text MCP server must be running
- Microphone access enabled
- Whisper model loaded for actual transcription
- MCP server configured in `.mcp.json`

## Troubleshooting

**Commands not appearing?**
- Restart Claude Code session
- Check that `.claude/commands/` directory exists
- Verify MCP server is running

**Recording not working?**
- Check microphone permissions
- Test with `/listen status` first to check status
- Ensure MCP server is connected

**Poor transcription quality?**
- Speak clearly and at normal volume
- Minimize background noise
- Record for at least 1-2 seconds
- Check audio levels with `/recording-status`

## Command Customization

You can customize voice commands via CLI arguments in your `.mcp.json` configuration or environment variables:

### MCP Configuration with Custom Commands
Edit your `.mcp.json` file:
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

### Environment Variables (Fallback)
```bash
export VOICE_START_COMMANDS="start,begin,go"
export VOICE_STOP_COMMANDS="stop,end,done"
export VOICE_STATUS_COMMANDS="status,check,info"
export VOICE_TOGGLE_COMMANDS="toggle,switch,"
```

### Multilingual Support
Example for Spanish/French commands in `.mcp.json`:
```json
{
  "mcpServers": {
    "voice-to-text": {
      "command": "./target/release/voice-to-text-mcp",
      "args": [
        "--mcp-server", "models/ggml-base.en.bin",
        "--start-commands", "start,iniciar,commencer",
        "--stop-commands", "stop,parar,arrêter",
        "--status-commands", "status,estado,statut"
      ]
    }
  }
}
```

## Technical Details

- Audio captured at 44.1kHz, processed to 16kHz for Whisper
- Supports mono/stereo microphones (converted to mono)
- Uses CUDA acceleration when available, CPU fallback
- Debug audio files saved to `debug/` directory when enabled
- Commands are case-insensitive and whitespace-tolerant