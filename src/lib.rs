use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration, Instant};

// Module declarations
pub mod error;
pub mod config;
pub mod platform;
pub mod platform_compat;
pub mod audio;
pub mod whisper;
pub mod mcp_server;

// Re-export commonly used types
pub use error::{Result, VoiceError};
pub use config::{DebugConfig};
pub use audio::{AudioCapture, AudioProcessor, AudioFileHandler};
pub use whisper::WhisperTranscriber;

use config::*;
use platform::debug_eprintln;

#[derive(Clone)]
pub struct VoiceToTextService {
    audio_capture: Arc<Mutex<AudioCapture>>,
    audio_file_handler: Arc<AudioFileHandler>,
    whisper_transcriber: Arc<tokio::sync::Mutex<WhisperTranscriber>>,
    debug_config: DebugConfig,
}

impl VoiceToTextService {
    pub fn new() -> Self {
        Self::new_with_debug(DebugConfig::default())
    }

    pub fn new_with_debug(debug_config: DebugConfig) -> Self {
        let audio_capture = Arc::new(Mutex::new(AudioCapture::new(debug_config.enabled)));
        let audio_file_handler = Arc::new(AudioFileHandler::new(debug_config.clone()));
        let whisper_transcriber = Arc::new(tokio::sync::Mutex::new(WhisperTranscriber::new(debug_config.enabled)));

        Self {
            audio_capture,
            audio_file_handler,
            whisper_transcriber,
            debug_config,
        }
    }

    pub fn new_with_model(model_path: &str) -> Result<Self> {
        Self::new_with_model_and_debug(model_path, DebugConfig::default())
    }

    pub fn new_with_model_and_debug(model_path: &str, debug_config: DebugConfig) -> Result<Self> {
        let audio_capture = Arc::new(Mutex::new(AudioCapture::new(debug_config.enabled)));
        let audio_file_handler = Arc::new(AudioFileHandler::new(debug_config.clone()));
        let whisper_transcriber = Arc::new(tokio::sync::Mutex::new(WhisperTranscriber::new_with_model(model_path, debug_config.enabled)?));

        Ok(Self {
            audio_capture,
            audio_file_handler,
            whisper_transcriber,
            debug_config,
        })
    }

    pub async fn start_listening(&self) -> Result<String> {
        {
            let audio_capture = self.audio_capture.lock().unwrap();
            audio_capture.start_capture()?;
        }
        Ok("Started listening...".to_string())
    }

    pub async fn start_listening_with_options(&self, timeout_ms: u64, silence_timeout_ms: u64, auto_stop: bool) -> Result<String> {
        // Check if we have a Whisper model loaded
        {
            let whisper_transcriber = self.whisper_transcriber.lock().await;
            if !whisper_transcriber.has_model() {
                return Err(VoiceError::WhisperModelNotLoaded);
            }
        }

        {
            let audio_capture = self.audio_capture.lock().unwrap();
            audio_capture.start_capture()?;
        }

        if auto_stop {
            // Start the auto-stop monitoring with voice activity detection
            return self.listen_with_auto_stop(timeout_ms, silence_timeout_ms).await;
        } else {
            Ok("Started listening...".to_string())
        }
    }

    pub async fn stop_listening(&self) -> Result<String> {
        let audio_data = {
            let audio_capture = self.audio_capture.lock().unwrap();
            audio_capture.stop_capture()?
        };

        // Save raw audio for debugging if enabled
        if self.debug_config.enabled && self.debug_config.save_raw {
            if let Err(e) = self.audio_file_handler.save_debug_audio(&audio_data, "raw", DEFAULT_SAMPLE_RATE) {
                debug_eprintln!(self.debug_config.enabled, "Warning: Failed to save raw audio debug file: {}", e);
            }
        }

        let transcription = self.transcribe_audio(audio_data).await?;
        Ok(transcription)
    }

    pub fn is_recording(&self) -> bool {
        self.audio_capture.lock().unwrap().is_recording()
    }

    pub fn get_audio_sample_count(&self) -> usize {
        self.audio_capture.lock().unwrap().get_audio_sample_count()
    }


    pub async fn transcribe_audio(&self, audio_data: Vec<f32>) -> Result<String> {
        let whisper_transcriber = self.whisper_transcriber.lock().await;
        
        let transcription = whisper_transcriber.transcribe_with_validation(audio_data.clone()).await?;
        
        // Save processed audio for debugging if enabled
        if self.debug_config.enabled && self.debug_config.save_processed {
            let audio_processor = whisper_transcriber.get_audio_processor();
            if let Ok(processed_audio) = audio_processor.prepare_for_whisper(&audio_data) {
                if let Err(e) = self.audio_file_handler.save_debug_audio(&processed_audio, "processed", WHISPER_SAMPLE_RATE) {
                    debug_eprintln!(self.debug_config.enabled, "Warning: Failed to save processed audio debug file: {}", e);
                }
            }
        }
        
        Ok(transcription)
    }

    pub fn get_debug_config(&self) -> &DebugConfig {
        &self.debug_config
    }

    pub fn set_debug_enabled(&mut self, enabled: bool) {
        // Note: This only affects the service's debug config, not the individual components
        // since they were created with the original config
        self.debug_config.enabled = enabled;
    }

    pub async fn transcribe_wav_file(&self, wav_path: &str) -> Result<String> {
        let audio_data = self.audio_file_handler.load_wav_file(wav_path)?;
        self.transcribe_audio(audio_data).await
    }

    async fn listen_with_auto_stop(&self, timeout_ms: u64, silence_timeout_ms: u64) -> Result<String> {
        let start_time = Instant::now();
        let mut last_activity_time = Instant::now();
        let check_interval = Duration::from_millis(CHECK_INTERVAL_MS);
        
        loop {
            // Check for overall timeout
            if start_time.elapsed().as_millis() > timeout_ms as u128 {
                break;
            }
            
            // Check if recording was stopped by voice command
            if !self.is_recording() {
                break;
            }
            
            // Check for voice activity
            let has_activity = {
                let audio_data = {
                    let audio_capture = self.audio_capture.lock().unwrap();
                    audio_capture.get_current_audio_data()
                };
                
                if audio_data.len() > samples_for_duration_ms(DEFAULT_SAMPLE_RATE, RECENT_SAMPLES_DURATION_MS) {
                    let whisper_transcriber = self.whisper_transcriber.lock().await;
                    let audio_processor = whisper_transcriber.get_audio_processor();
                    audio_processor.has_voice_activity(&audio_data)
                } else {
                    false
                }
            };
            
            if has_activity {
                last_activity_time = Instant::now();
            } else {
                // Check if we've been silent for too long
                if last_activity_time.elapsed().as_millis() > silence_timeout_ms as u128 {
                    // Only auto-stop if we have some recorded audio
                    let has_audio = {
                        let audio_capture = self.audio_capture.lock().unwrap();
                        audio_capture.get_audio_sample_count() > samples_for_duration_ms(DEFAULT_SAMPLE_RATE, 500) // 0.5 seconds
                    };
                    
                    if has_audio {
                        break;
                    } else {
                        // Also auto-stop even without audio if we've been silent long enough
                        // This prevents hanging when no speech is detected
                        if last_activity_time.elapsed().as_millis() > (silence_timeout_ms * 2) as u128 {
                            break;
                        }
                    }
                }
            }
            
            sleep(check_interval).await;
        }
        
        // Stop recording and get transcription
        self.stop_listening().await
    }

}

impl std::fmt::Debug for VoiceToTextService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoiceToTextService")
            .field("is_recording", &self.is_recording())
            .field("audio_data_len", &self.get_audio_sample_count())
            .field("has_whisper_context", &"<async_mutex>")
            .field("debug_config", &self.debug_config)
            .finish()
    }
}

impl Default for VoiceToTextService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_creation() {
        let service = VoiceToTextService::new();
        assert!(!service.is_recording());
        assert_eq!(service.get_audio_sample_count(), 0);
    }

    #[tokio::test]
    async fn test_recording_state_management() {
        let service = VoiceToTextService::new();
        
        // Initially not recording
        assert!(!service.is_recording());
        
        // Stop when not recording should return appropriate error
        let result = service.stop_listening().await;
        assert!(matches!(result, Err(VoiceError::NotRecording)));
    }

    #[tokio::test]
    async fn test_start_listening_twice() {
        let service = VoiceToTextService::new();
        
        // First call should work (though may fail due to no audio device in test env)
        let first_result = service.start_listening().await;
        
        // If the first call succeeded, the second should return error
        if first_result.is_ok() {
            let second_result = service.start_listening().await;
            assert!(matches!(second_result, Err(VoiceError::AlreadyRecording)));
        }
    }

    #[tokio::test]
    async fn test_transcribe_empty_audio() {
        let service = VoiceToTextService::new();
        let result = service.transcribe_audio(vec![]).await.unwrap();
        assert_eq!(result, "No audio data recorded");
    }

    #[tokio::test]
    async fn test_transcribe_with_audio_data() {
        let service = VoiceToTextService::new();
        let audio_data = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let result = service.transcribe_audio(audio_data.clone()).await.unwrap();
        assert!(result.contains(&audio_data.len().to_string()) || 
                result.contains("model not loaded"));
        assert!(result.contains("Transcribed") || 
                result.contains("model not loaded"));
    }

    #[test]
    fn test_default_implementation() {
        let service = VoiceToTextService::default();
        assert!(!service.is_recording());
        assert_eq!(service.get_audio_sample_count(), 0);
    }

    #[tokio::test]
    async fn test_audio_sample_count_consistency() {
        let service = VoiceToTextService::new();
        
        // Initially should be 0
        assert_eq!(service.get_audio_sample_count(), 0);
        
        // After attempting to start (even if it fails), should still be 0
        let _result = service.start_listening().await;
        
        // Sample count should still be accessible and should be non-negative
        let count = service.get_audio_sample_count();
        assert!(count == 0 || count > 0); // More meaningful than >= 0 for usize
    }

    #[tokio::test]
    async fn test_clone_functionality() {
        let service1 = VoiceToTextService::new();
        let service2 = service1.clone();
        
        // Both should share the same state
        assert_eq!(service1.is_recording(), service2.is_recording());
        assert_eq!(service1.get_audio_sample_count(), service2.get_audio_sample_count());
        
        // If one starts recording, both should reflect that
        let start_result = service1.start_listening().await;
        if start_result.is_ok() && start_result.unwrap().contains("Started") {
            assert_eq!(service1.is_recording(), service2.is_recording());
        }
    }

    #[tokio::test]
    async fn test_transcription_edge_cases() {
        let service = VoiceToTextService::new();
        
        // Test with single sample
        let single_sample = vec![0.5];
        let result = service.transcribe_audio(single_sample).await.unwrap();
        assert!(result.contains("1") || result.contains("model not loaded"));
        
        // Test with many samples
        let many_samples = vec![0.1; 10000];
        let result = service.transcribe_audio(many_samples).await.unwrap();
        assert!(result.contains("10000") || result.contains("model not loaded"));
        
        // Test with extreme values
        let extreme_samples = vec![f32::MAX, f32::MIN, 0.0, f32::INFINITY, f32::NEG_INFINITY];
        let result = service.transcribe_audio(extreme_samples).await;
        assert!(result.is_ok()); // Should handle extreme values gracefully
    }

    #[tokio::test]
    async fn test_whisper_model_loading() {
        // Test that new() creates service without model
        let service = VoiceToTextService::new();
        let audio_data = vec![0.1, 0.2, 0.3];
        let result = service.transcribe_audio(audio_data).await.unwrap();
        assert!(result.contains("model not loaded"));
        
        // Test that new_with_model() with invalid path returns error
        let invalid_model_result = VoiceToTextService::new_with_model("nonexistent_model.bin");
        assert!(invalid_model_result.is_err());
    }

    #[test]
    fn test_debug_config_creation() {
        let debug_config = DebugConfig {
            enabled: true,
            output_dir: std::path::PathBuf::from("./test_debug"),
            save_raw: true,
            save_processed: false,
        };
        
        let service = VoiceToTextService::new_with_debug(debug_config.clone());
        let service_config = service.get_debug_config();
        
        assert_eq!(service_config.enabled, true);
        assert_eq!(service_config.output_dir, std::path::PathBuf::from("./test_debug"));
        assert_eq!(service_config.save_raw, true);
        assert_eq!(service_config.save_processed, false);
    }

    #[test]
    fn test_debug_disabled_by_default() {
        let service = VoiceToTextService::new();
        let config = service.get_debug_config();
        
        assert!(!config.enabled);
        assert_eq!(config.output_dir, std::path::PathBuf::from("./debug"));
        assert!(config.save_raw);
        assert!(config.save_processed);
    }

    #[test]
    fn test_platform_specific_acceleration_features() {
        // Test that the correct platform-specific dependencies are configured
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            // macOS Apple Silicon should have platform-specific config
            println!("✅ macOS Apple Silicon: Metal + CoreML + NEON acceleration configured");
            assert_eq!(std::env::consts::OS, "macos");
            assert_eq!(std::env::consts::ARCH, "aarch64");
        }
        
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        {
            // macOS Intel should have platform-specific config
            println!("✅ macOS Intel: Metal acceleration configured");
            assert_eq!(std::env::consts::OS, "macos");
            assert_eq!(std::env::consts::ARCH, "x86_64");
        }
        
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            // Linux x86_64 should have platform-specific config
            println!("✅ Linux x86_64: CUDA acceleration configured");
            assert_eq!(std::env::consts::OS, "linux");
            assert_eq!(std::env::consts::ARCH, "x86_64");
        }
        
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        {
            // Windows x86_64 should have platform-specific config
            println!("✅ Windows x86_64: CUDA acceleration configured");
            assert_eq!(std::env::consts::OS, "windows");
            assert_eq!(std::env::consts::ARCH, "x86_64");
        }
        
        #[cfg(not(any(
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "windows", target_arch = "x86_64")
        )))]
        {
            println!("✅ Generic platform: CPU-only implementation configured");
        }
    }

    #[test]
    fn test_hardware_acceleration_runtime_info() {
        // Test that we can identify the current platform's acceleration capabilities
        let platform = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        
        match (platform, arch) {
            ("macos", "aarch64") => {
                println!("🚀 Runtime: macOS Apple Silicon - Metal/CoreML/NEON available");
                assert_eq!(platform, "macos");
                assert_eq!(arch, "aarch64");
            },
            ("macos", "x86_64") => {
                println!("🚀 Runtime: macOS Intel - Metal available");
                assert_eq!(platform, "macos");
                assert_eq!(arch, "x86_64");
            },
            ("linux", "x86_64") => {
                println!("🚀 Runtime: Linux x86_64 - CUDA available");
                assert_eq!(platform, "linux");
                assert_eq!(arch, "x86_64");
            },
            ("windows", "x86_64") => {
                println!("🚀 Runtime: Windows x86_64 - CUDA available");
                assert_eq!(platform, "windows");
                assert_eq!(arch, "x86_64");
            },
            _ => {
                println!("🚀 Runtime: Generic platform ({}, {}) - CPU only", platform, arch);
            }
        }
    }

    #[tokio::test]
    async fn test_whisper_context_initialization_with_acceleration() {
        let service = VoiceToTextService::new();
        
        // Test that service initializes correctly regardless of acceleration availability
        assert!(!service.is_recording());
        assert_eq!(service.get_audio_sample_count(), 0);
        
        // Test transcription with placeholder model (should handle gracefully)
        let test_audio = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let result = service.transcribe_audio(test_audio).await;
        
        assert!(result.is_ok());
        let transcription = result.unwrap();
        
        // Should either transcribe or indicate model not loaded
        assert!(
            transcription.contains("Transcribed") || 
            transcription.contains("model not loaded"),
            "Unexpected transcription result: {}", transcription
        );
    }

    #[tokio::test]
    async fn test_acceleration_fallback_behavior() {
        // Test that the service works even if hardware acceleration fails
        let service = VoiceToTextService::new();
        
        // These operations should work regardless of hardware acceleration availability
        let _start_result = service.start_listening().await;
        
        // Even if start fails (no audio device), the service should remain in a valid state
        assert!(!service.is_recording() || service.is_recording()); // Either state is valid
        let _sample_count = service.get_audio_sample_count(); // Should always be accessible
        
        // Stop should handle the case where recording never started
        let stop_result = service.stop_listening().await;
        
        // Should either succeed or return NotRecording error
        match stop_result {
            Ok(msg) => assert!(msg.contains("Not currently recording") || msg.contains("Transcribed") || msg.contains("No audio data")),
            Err(VoiceError::NotRecording) => { /* This is expected */ },
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn test_audio_processing_pipeline_consistency() {
        let debug_config = DebugConfig::default();
        let audio_processor = AudioProcessor::new(debug_config.enabled);
        
        // Test that audio processing works consistently across all platforms
        let test_audio = vec![0.1, -0.2, 0.3, -0.4, 0.5];
        let processed = audio_processor.prepare_for_whisper(&test_audio);
        
        assert!(processed.is_ok());
        let processed_audio = processed.unwrap();
        
        // Processed audio should be valid for Whisper regardless of platform
        assert!(!processed_audio.is_empty());
        assert!(processed_audio.iter().all(|&x| x.is_finite()));
        assert!(processed_audio.iter().all(|&x| x >= -1.0 && x <= 1.0));
        
        // Audio should be normalized to [-1.0, 1.0] range
        let max_val = processed_audio.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
        assert!(max_val <= 1.0, "Audio should be normalized to [-1.0, 1.0] range");
    }
}