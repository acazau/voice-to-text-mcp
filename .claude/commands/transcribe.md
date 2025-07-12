# Transcribe Audio File

Transcribe audio from a WAV file to text using Whisper.

**Usage:** `/transcribe [file_path]`

**Arguments:**
- `file_path` - Path to WAV audio file

**Example:**
- `/transcribe debug/audio_20250710_194139_raw.wav`
- `/transcribe recordings/meeting.wav`

Uses the `transcribe_file` MCP tool to process audio files. Supports WAV format with automatic audio processing and normalization.