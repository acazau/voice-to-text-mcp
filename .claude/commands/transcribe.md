# Transcribe Audio File

Transcribe audio from a WAV file to text using Whisper.

**Usage:** `/transcribe_file file_path=<path>`

**Parameters:**
- `file_path` - Path to audio file (WAV format recommended)

**Examples:**
- `/transcribe_file file_path="debug/audio_20250710_194139_raw.wav"`
- `/transcribe_file file_path="/path/to/meeting.wav"`
- `/transcribe_file file_path="./recordings/interview.wav"`

**Supported Formats:**
- WAV files (recommended)
- Other formats supported by the underlying audio processing library

**Technical Details:**
- Processes audio through the VoiceToTextService
- Automatically handles audio resampling and normalization
- Supports the same hardware acceleration as live recording
- Works with existing audio files from debug mode or external sources

**Note:** File paths with spaces should be quoted. Relative paths are resolved from the current working directory.