# Voice Recording Commands for Claude Code

This directory contains custom slash commands for easy voice recording and transcription within Claude Code.

## Available Commands

### ⚡ Ultra-Short Commands (2 Letters)
- **`/rc`** - Begin voice recording  
- **`/st`** - Stop recording and get transcription
- **`/cr`** - Check current recording state
- **`/tr [path]`** - Transcribe an existing WAV file

## Quick Start

1. **Start Recording:**
   ```
   /rc
   ```

2. **Speak your message** (at least 1-2 seconds)

3. **Stop Recording:**
   ```
   /st
   ```

4. **Check Status anytime:**
   ```
   /cr
   ```

## Workflow Example

```bash
# Check if anything is recording
/cr

# Start a new recording
/rc

# [Speak your message]

# Stop and get transcription
/st

# Transcribe an existing file
/tr debug/audio_20250710_194139_raw.wav
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
- Test with `/recording-status` first
- Ensure MCP server is connected

**Poor transcription quality?**
- Speak clearly and at normal volume
- Minimize background noise
- Record for at least 1-2 seconds
- Check audio levels with `/recording-status`

## Technical Details

- Audio captured at 44.1kHz, processed to 16kHz for Whisper
- Supports mono/stereo microphones (converted to mono)
- Uses CUDA acceleration when available, CPU fallback
- Debug audio files saved to `debug/` directory when enabled