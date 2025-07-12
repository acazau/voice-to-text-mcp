use thiserror::Error;

#[derive(Error, Debug)]
pub enum VoiceError {
    #[error("Audio device not available")]
    AudioDeviceNotAvailable,
    
    #[error("No input device found")]
    NoInputDevice,
    
    #[error("Audio stream error: {0}")]
    AudioStream(String),
    
    #[error("Whisper model loading failed: {0}")]
    WhisperModelLoad(String),
    
    #[error("Whisper model not loaded - use new_with_model() to load a model")]
    WhisperModelNotLoaded,
    
    #[error("Whisper transcription failed: {0}")]
    WhisperTranscription(String),
    
    #[error("Audio too short: {duration:.2}s (need at least 0.5s)")]
    AudioTooShort { duration: f32 },
    
    #[error("Audio too quiet: max amplitude {amplitude:.6}")]
    AudioTooQuiet { amplitude: f32 },
    
    #[error("WAV file error: {0}")]
    WavFile(String),
    
    #[error("Debug directory creation failed: {0}")]
    DebugDirectory(String),
    
    #[error("Debug file saving failed: {0}")]
    DebugFileSave(String),
    
    #[error("Recording already in progress")]
    AlreadyRecording,
    
    #[error("Not currently recording")]
    NotRecording,
    
    #[error("Keyboard control initialization failed: {0}")]
    KeyboardControl(String),
    
    #[error("Platform operation failed: {0}")]
    Platform(String),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Audio processing error: {0}")]
    AudioProcessing(String),
}

impl From<anyhow::Error> for VoiceError {
    fn from(error: anyhow::Error) -> Self {
        VoiceError::Platform(error.to_string())
    }
}

impl From<cpal::BuildStreamError> for VoiceError {
    fn from(error: cpal::BuildStreamError) -> Self {
        VoiceError::AudioStream(error.to_string())
    }
}

impl From<cpal::PlayStreamError> for VoiceError {
    fn from(error: cpal::PlayStreamError) -> Self {
        VoiceError::AudioStream(error.to_string())
    }
}

impl From<cpal::DefaultStreamConfigError> for VoiceError {
    fn from(error: cpal::DefaultStreamConfigError) -> Self {
        VoiceError::AudioStream(error.to_string())
    }
}

impl From<hound::Error> for VoiceError {
    fn from(error: hound::Error) -> Self {
        VoiceError::WavFile(error.to_string())
    }
}

impl From<whisper_rs::WhisperError> for VoiceError {
    fn from(error: whisper_rs::WhisperError) -> Self {
        VoiceError::WhisperTranscription(error.to_string())
    }
}

// Type alias for convenience
pub type Result<T> = std::result::Result<T, VoiceError>;