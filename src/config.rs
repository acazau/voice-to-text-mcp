use std::path::PathBuf;

// Audio processing constants
pub const DEFAULT_SAMPLE_RATE: u32 = 44100;
pub const WHISPER_SAMPLE_RATE: u32 = 16000;
pub const SILENCE_THRESHOLD: f32 = 0.01;
pub const MIN_AUDIO_DURATION: f32 = 0.5;
pub const MIN_AUDIO_AMPLITUDE: f32 = 0.001;
pub const CHECK_INTERVAL_MS: u64 = 100;
pub const RECENT_SAMPLES_DURATION_MS: u64 = 100;

// Default timeout values
pub const DEFAULT_TIMEOUT_MS: u64 = 30000;
pub const DEFAULT_SILENCE_TIMEOUT_MS: u64 = 2000;

// Audio buffer calculation helpers
pub const fn samples_for_duration_ms(sample_rate: u32, duration_ms: u64) -> usize {
    ((sample_rate as u64 * duration_ms) / 1000) as usize
}


#[derive(Clone, Debug)]
pub struct DebugConfig {
    pub enabled: bool,
    pub output_dir: PathBuf,
    pub save_raw: bool,
    pub save_processed: bool,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            output_dir: PathBuf::from("./debug"),
            save_raw: true,
            save_processed: true,
        }
    }
}

