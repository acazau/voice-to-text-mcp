# Voice Recording Control

Control voice recording with start/stop/status/toggle functionality.

**Usage:** `/listen [command] [options]`

**Commands:**
- `start` - Begin recording (blocks until complete with auto-stop)
- `stop` - End recording  
- `status` - Check recording status
- `toggle` - Switch recording state (non-blocking for backwards compatibility)
- (empty) - Toggle recording state

**Optional Parameters:**
- `timeout_ms` - Maximum recording duration in milliseconds (default: 30000)
- `silence_timeout_ms` - Auto-stop after silence duration in milliseconds (default: 2000)
- `auto_stop` - Enable automatic stopping on silence detection (default: true for start, false for toggle)
- `enable_voice_commands` - Enable voice command recognition during recording

**Examples:**
- `/listen start` - Start recording with auto-stop (blocks until complete)
- `/listen stop` - Stop recording
- `/listen status` - Check if recording
- `/listen` - Toggle recording on/off (non-blocking)

**Blocking Behavior:**
- `start` command now waits for voice input and returns transcription when complete
- Uses voice activity detection to auto-stop after silence
- `toggle` command maintains backwards compatibility with immediate return

Uses the `listen` MCP tool with configurable voice commands. See project README for custom command configuration.