# Voice Recording

Record audio and return transcribed text using Whisper.

**Usage:** `/listen [options]`

**Behavior:**
- Records audio from your microphone
- Automatically stops after silence or timeout
- Returns transcribed text when complete
- Blocks until recording is finished (within 30 seconds)

**Optional Parameters:**
- `timeout_ms` - Maximum recording duration in milliseconds (default: 30000)
- `silence_timeout_ms` - Auto-stop after silence duration in milliseconds (default: 2000)  
- `auto_stop` - Enable automatic stopping on silence detection (default: true)

**Examples:**
- `/listen` - Record with default settings (30s max, 2s silence timeout)
- `/listen timeout_ms=60000` - Record for up to 60 seconds
- `/listen silence_timeout_ms=3000` - Wait 3 seconds of silence before stopping
- `/listen auto_stop=false` - Disable auto-stop (record until timeout)

**Technical Details:**
- Uses blocking MCP server architecture for reliability
- Processes audio through unified `voice-to-text-mcp` binary
- Requires Whisper model file for transcription
- Supports hardware acceleration (Metal/CoreML/CUDA) when available

**Note:** This tool requires a microphone and Whisper model file to function properly.