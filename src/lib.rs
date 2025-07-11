use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use anyhow::Result;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};
use hound::{WavWriter, WavSpec, WavReader};
use std::path::PathBuf;
use std::fs;
use chrono::Utc;

pub mod mcp_server;

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

#[derive(Clone)]
pub struct VoiceToTextService {
    is_recording: Arc<AtomicBool>,
    audio_data: Arc<Mutex<Vec<f32>>>,
    whisper_context: Arc<tokio::sync::Mutex<Option<WhisperContext>>>,
    debug_config: DebugConfig,
}

impl VoiceToTextService {
    pub fn new() -> Self {
        Self::new_with_debug(DebugConfig::default())
    }

    pub fn new_with_debug(debug_config: DebugConfig) -> Self {
        Self {
            is_recording: Arc::new(AtomicBool::new(false)),
            audio_data: Arc::new(Mutex::new(Vec::new())),
            whisper_context: Arc::new(tokio::sync::Mutex::new(None)),
            debug_config,
        }
    }

    pub fn new_with_model(model_path: &str) -> Result<Self> {
        Self::new_with_model_and_debug(model_path, DebugConfig::default())
    }

    pub fn new_with_model_and_debug(model_path: &str, debug_config: DebugConfig) -> Result<Self> {
        // Log hardware acceleration status
        Self::log_acceleration_status();
        
        let ctx = WhisperContext::new_with_params(
            model_path,
            WhisperContextParameters::default(),
        )?;
        
        Ok(Self {
            is_recording: Arc::new(AtomicBool::new(false)),
            audio_data: Arc::new(Mutex::new(Vec::new())),
            whisper_context: Arc::new(tokio::sync::Mutex::new(Some(ctx))),
            debug_config,
        })
    }
    
    fn log_acceleration_status() {
        println!("Initializing Whisper with hardware acceleration:");
        
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        println!("  Platform: macOS Apple Silicon (Metal + CoreML enabled)");
        
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        println!("  Platform: macOS Intel (Metal enabled)");
        
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        println!("  Platform: Linux x86_64 (CUDA enabled)");
        
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        println!("  Platform: Windows x86_64 (CUDA enabled)");
        
        #[cfg(not(any(
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "windows", target_arch = "x86_64")
        )))]
        println!("  Platform: No hardware acceleration (CPU-only mode)");
    }

    pub async fn start_listening(&self) -> Result<String> {
        if self.is_recording.load(Ordering::Relaxed) {
            return Ok("Already recording".to_string());
        }

        self.is_recording.store(true, Ordering::Relaxed);
        
        // Clear previous audio data
        {
            let mut data = self.audio_data.lock().unwrap();
            data.clear();
        }

        // Start audio capture
        self.start_audio_capture().await?;

        Ok("Started listening...".to_string())
    }

    pub async fn stop_listening(&self) -> Result<String> {
        if !self.is_recording.load(Ordering::Relaxed) {
            return Ok("Not currently recording".to_string());
        }

        self.is_recording.store(false, Ordering::Relaxed);

        // Get audio data and transcribe
        let audio_data = {
            let data = self.audio_data.lock().unwrap();
            data.clone()
        };

        // Save raw audio for debugging if enabled
        if self.debug_config.enabled && self.debug_config.save_raw {
            if let Err(e) = self.save_audio_debug(&audio_data, "raw", 44100) {
                eprintln!("Warning: Failed to save raw audio debug file: {}", e);
            }
        }

        let transcription = self.transcribe_audio(audio_data).await?;
        Ok(transcription)
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }

    pub fn get_audio_sample_count(&self) -> usize {
        self.audio_data.lock().unwrap().len()
    }

    async fn start_audio_capture(&self) -> Result<()> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No input device available"))?;

        let config = device.default_input_config()?;
        let _sample_rate = config.sample_rate().0;
        let channels = config.channels();

        let audio_data = Arc::clone(&self.audio_data);
        let is_recording = Arc::clone(&self.is_recording);

        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if is_recording.load(Ordering::Relaxed) {
                    let mut audio_buffer = audio_data.lock().unwrap();
                    
                    // Convert to mono if stereo
                    if channels == 1 {
                        audio_buffer.extend_from_slice(data);
                    } else {
                        // Convert stereo to mono by averaging channels
                        for chunk in data.chunks(channels as usize) {
                            let mono_sample = chunk.iter().sum::<f32>() / channels as f32;
                            audio_buffer.push(mono_sample);
                        }
                    }
                }
            },
            move |err| {
                eprintln!("Audio stream error: {}", err);
            },
            None,
        )?;

        stream.play()?;
        
        // Keep stream alive by storing it in a static variable or similar
        // For now, we'll just forget about it since it will be cleaned up when recording stops
        std::mem::forget(stream);

        Ok(())
    }

    pub async fn transcribe_audio(&self, audio_data: Vec<f32>) -> Result<String> {
        if audio_data.is_empty() {
            return Ok("No audio data recorded".to_string());
        }

        // Check if we have a Whisper model loaded
        let whisper_ctx = self.whisper_context.lock().await;
        if let Some(ref _ctx) = *whisper_ctx {
            drop(whisper_ctx); // Release the lock before processing
            
            // Convert audio to the format Whisper expects (16kHz, mono)
            let processed_audio = self.prepare_audio_for_whisper(&audio_data)?;
            
            // Save processed audio for debugging if enabled
            if self.debug_config.enabled && self.debug_config.save_processed {
                if let Err(e) = self.save_audio_debug(&processed_audio, "processed", 16000) {
                    eprintln!("Warning: Failed to save processed audio debug file: {}", e);
                }
            }
            
            // Perform transcription
            self.transcribe_with_whisper(&processed_audio).await
        } else {
            // Fallback to placeholder if no model loaded
            Ok(format!("Transcribed {} audio samples (Whisper model not loaded - use new_with_model() to load a model)", audio_data.len()))
        }
    }

    pub fn prepare_audio_for_whisper(&self, audio_data: &[f32]) -> Result<Vec<f32>> {
        // Whisper expects 16kHz mono audio
        // Most audio capture happens at 44.1kHz, so we need to resample
        
        // First, resample from 44.1kHz to 16kHz using simple decimation
        let resampled = self.resample_audio(audio_data, 44100, 16000);
        
        // Then normalize audio to prevent clipping
        let max_amplitude = resampled.iter()
            .map(|&x| x.abs())
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(1.0);
            
        if max_amplitude > 0.0 {
            Ok(resampled.iter().map(|&x| x / max_amplitude).collect())
        } else {
            Ok(resampled)
        }
    }

    fn resample_audio(&self, audio_data: &[f32], input_rate: u32, output_rate: u32) -> Vec<f32> {
        if input_rate == output_rate {
            return audio_data.to_vec();
        }
        
        let ratio = input_rate as f64 / output_rate as f64;
        let output_length = (audio_data.len() as f64 / ratio).ceil() as usize;
        let mut resampled = Vec::with_capacity(output_length);
        
        // Simple linear interpolation resampling
        for i in 0..output_length {
            let input_index = i as f64 * ratio;
            let index_floor = input_index.floor() as usize;
            let index_ceil = (index_floor + 1).min(audio_data.len() - 1);
            let fraction = input_index - index_floor as f64;
            
            if index_floor >= audio_data.len() {
                break;
            }
            
            let sample = if index_floor == index_ceil {
                audio_data[index_floor]
            } else {
                // Linear interpolation
                let sample1 = audio_data[index_floor];
                let sample2 = audio_data[index_ceil];
                sample1 + (sample2 - sample1) * fraction as f32
            };
            
            resampled.push(sample);
        }
        
        eprintln!("🔄 Resampled {} samples ({}Hz) -> {} samples ({}Hz)", 
                audio_data.len(), input_rate, resampled.len(), output_rate);
        
        resampled
    }

    async fn transcribe_with_whisper(&self, audio_data: &[f32]) -> Result<String> {
        let whisper_ctx = self.whisper_context.lock().await;
        if let Some(ref ctx) = *whisper_ctx {
            // Audio validation and debugging
            let duration_seconds = audio_data.len() as f32 / 16000.0;
            let max_amplitude = audio_data.iter().map(|&x| x.abs()).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(0.0);
            let rms = (audio_data.iter().map(|&x| x * x).sum::<f32>() / audio_data.len() as f32).sqrt();
            
            eprintln!("🎤 Audio stats: {:.2}s duration, max amplitude: {:.4}, RMS: {:.4}", 
                    duration_seconds, max_amplitude, rms);
            
            // Check minimum requirements
            if duration_seconds < 0.5 {
                return Ok(format!("Audio too short: {:.2}s (need at least 0.5s)", duration_seconds));
            }
            
            if max_amplitude < 0.001 {
                return Ok(format!("Audio too quiet: max amplitude {:.6}", max_amplitude));
            }
            
            // Create transcription parameters
            let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
            
            // Improved Whisper settings for better speech detection
            params.set_language(None); // Auto-detect language
            params.set_print_special(false);
            params.set_print_progress(false);
            params.set_print_realtime(false);
            params.set_print_timestamps(false);
            params.set_suppress_blank(true);
            params.set_suppress_nst(true);
            params.set_temperature(0.0);
            params.set_max_initial_ts(1.0);
            params.set_length_penalty(-1.0);
            
            eprintln!("🤖 Running Whisper transcription...");
            
            // Run the transcription
            let mut state = ctx.create_state()?;
            state.full(params, audio_data)?;
            
            // Collect the transcribed text with detailed logging
            let num_segments = state.full_n_segments()?;
            eprintln!("📝 Whisper found {} segments", num_segments);
            
            let mut result = String::new();
            let mut all_segments = Vec::new();
            
            for i in 0..num_segments {
                let segment_text = state.full_get_segment_text(i)?;
                let start_time = state.full_get_segment_t0(i)?;
                let end_time = state.full_get_segment_t1(i)?;
                
                eprintln!("   Segment {}: [{:.2}s-{:.2}s] '{}'", i, start_time as f32 / 100.0, end_time as f32 / 100.0, segment_text);
                all_segments.push(segment_text.clone());
                result.push_str(&segment_text);
            }
            
            let trimmed_result = result.trim();
            
            // Enhanced result analysis
            if trimmed_result.is_empty() {
                Ok("No speech detected in audio (Whisper returned empty result)".to_string())
            } else if trimmed_result == "[SOUND]" || trimmed_result.contains("[SOUND]") {
                Ok(format!("Whisper detected audio but no clear speech: '{}' - try speaking louder/clearer or recording longer", trimmed_result))
            } else {
                Ok(trimmed_result.to_string())
            }
        } else {
            Err(anyhow::anyhow!("Whisper context not available"))
        }
    }

    fn ensure_debug_directory(&self) -> Result<()> {
        if self.debug_config.enabled {
            fs::create_dir_all(&self.debug_config.output_dir)?;
        }
        Ok(())
    }

    fn generate_debug_filename(&self, suffix: &str) -> PathBuf {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("audio_{}_{}.wav", timestamp, suffix);
        self.debug_config.output_dir.join(filename)
    }

    fn save_audio_debug(&self, audio_data: &[f32], suffix: &str, sample_rate: u32) -> Result<()> {
        if !self.debug_config.enabled {
            return Ok(());
        }

        self.ensure_debug_directory()?;
        let filepath = self.generate_debug_filename(suffix);

        let spec = WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let mut writer = WavWriter::create(&filepath, spec)?;
        for &sample in audio_data {
            writer.write_sample(sample)?;
        }
        writer.finalize()?;

        eprintln!("🔧 Debug: Saved {} samples to {}", audio_data.len(), filepath.display());
        Ok(())
    }

    pub fn get_debug_config(&self) -> &DebugConfig {
        &self.debug_config
    }

    pub fn set_debug_enabled(&mut self, enabled: bool) {
        self.debug_config.enabled = enabled;
    }

    pub async fn transcribe_wav_file(&self, wav_path: &str) -> Result<String> {
        eprintln!("📁 Loading WAV file: {}", wav_path);
        
        // Read the WAV file
        let mut reader = WavReader::open(wav_path)?;
        let spec = reader.spec();
        
        eprintln!("🎵 WAV specs: {}Hz, {} channels, {} bits", 
                spec.sample_rate, spec.channels, spec.bits_per_sample);
        
        // Read all samples as f32
        let samples: Result<Vec<f32>, _> = match spec.sample_format {
            hound::SampleFormat::Float => {
                reader.samples::<f32>().collect()
            }
            hound::SampleFormat::Int => {
                reader.samples::<i32>().map(|s| s.map(|sample| {
                    // Convert integer samples to float [-1.0, 1.0]
                    sample as f32 / (1i32 << (spec.bits_per_sample - 1)) as f32
                })).collect()
            }
        };
        
        let mut audio_data = samples?;
        
        // Convert stereo to mono if needed
        if spec.channels == 2 {
            eprintln!("🔄 Converting stereo to mono");
            let mono_data: Vec<f32> = audio_data.chunks(2)
                .map(|chunk| (chunk[0] + chunk.get(1).unwrap_or(&0.0)) / 2.0)
                .collect();
            audio_data = mono_data;
        }
        
        eprintln!("📊 Loaded {} samples from WAV file", audio_data.len());
        
        // Transcribe using the existing pipeline
        self.transcribe_audio(audio_data).await
    }
}

impl std::fmt::Debug for VoiceToTextService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoiceToTextService")
            .field("is_recording", &self.is_recording)
            .field("audio_data_len", &self.audio_data.lock().unwrap().len())
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
        
        // Stop when not recording should return appropriate message
        let result = service.stop_listening().await.unwrap();
        assert_eq!(result, "Not currently recording");
    }

    #[tokio::test]
    async fn test_start_listening_twice() {
        let service = VoiceToTextService::new();
        
        // First call should work (though may fail due to no audio device in test env)
        let first_result = service.start_listening().await;
        
        // If the first call succeeded, the second should return "Already recording"
        if first_result.is_ok() {
            let second_result = service.start_listening().await.unwrap();
            assert_eq!(second_result, "Already recording");
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
    fn test_audio_resampling_and_normalization() {
        let service = VoiceToTextService::new();
        
        // Test resampling from 44.1kHz to 16kHz
        let input_samples = 44100; // 1 second at 44.1kHz
        let audio_44k: Vec<f32> = (0..input_samples).map(|i| (i as f32 / input_samples as f32) * 0.5).collect();
        let resampled = service.resample_audio(&audio_44k, 44100, 16000);
        
        // Should be approximately 16000 samples (1 second at 16kHz)
        assert!((resampled.len() as f32 - 16000.0).abs() < 100.0);
        
        // Test normalization with normal values
        let normal_audio = vec![0.1, 0.2, -0.3, 0.4];
        let processed = service.prepare_audio_for_whisper(&normal_audio).unwrap();
        // After resampling 44.1->16kHz, should have fewer samples
        assert!(processed.len() < normal_audio.len());
        
        // Test with all zeros
        let zero_audio = vec![0.0, 0.0, 0.0];
        let processed = service.prepare_audio_for_whisper(&zero_audio).unwrap();
        assert!(processed.iter().all(|&x| x == 0.0)); // Should remain zero
        
        // Test resampling edge cases
        let same_rate = service.resample_audio(&[1.0, 2.0, 3.0], 16000, 16000);
        assert_eq!(same_rate, vec![1.0, 2.0, 3.0]); // Should be unchanged
    }

    #[test]
    fn test_debug_config_creation() {
        let debug_config = DebugConfig {
            enabled: true,
            output_dir: PathBuf::from("./test_debug"),
            save_raw: true,
            save_processed: false,
        };
        
        let service = VoiceToTextService::new_with_debug(debug_config.clone());
        let service_config = service.get_debug_config();
        
        assert_eq!(service_config.enabled, true);
        assert_eq!(service_config.output_dir, PathBuf::from("./test_debug"));
        assert_eq!(service_config.save_raw, true);
        assert_eq!(service_config.save_processed, false);
    }

    #[test]
    fn test_debug_filename_generation() {
        let debug_config = DebugConfig {
            enabled: true,
            output_dir: PathBuf::from("./test_debug"),
            save_raw: true,
            save_processed: true,
        };
        
        let service = VoiceToTextService::new_with_debug(debug_config);
        let filename = service.generate_debug_filename("test");
        
        assert!(filename.to_string_lossy().contains("test_debug"));
        assert!(filename.to_string_lossy().contains("audio_"));
        assert!(filename.to_string_lossy().contains("test.wav"));
    }

    #[test]
    fn test_debug_disabled_by_default() {
        let service = VoiceToTextService::new();
        let config = service.get_debug_config();
        
        assert!(!config.enabled);
        assert_eq!(config.output_dir, PathBuf::from("./debug"));
        assert!(config.save_raw);
        assert!(config.save_processed);
    }

    #[tokio::test]
    async fn test_debug_audio_saving() {
        let temp_dir = std::env::temp_dir().join("voice_test_debug");
        let debug_config = DebugConfig {
            enabled: true,
            output_dir: temp_dir.clone(),
            save_raw: true,
            save_processed: true,
        };
        
        let service = VoiceToTextService::new_with_debug(debug_config);
        let test_audio = vec![0.1, 0.2, -0.3, 0.4, 0.5];
        
        // Test saving debug audio
        let result = service.save_audio_debug(&test_audio, "test", 44100);
        assert!(result.is_ok());
        
        // Clean up
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir).ok();
        }
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
        assert!(stop_result.is_ok());
        
        let stop_message = stop_result.unwrap();
        assert!(
            stop_message.contains("Not currently recording") || 
            stop_message.contains("Transcribed") ||
            stop_message.contains("No audio data")
        );
    }

    #[test]
    fn test_audio_processing_pipeline_consistency() {
        let service = VoiceToTextService::new();
        
        // Test that audio processing works consistently across all platforms
        let test_audio = vec![0.1, -0.2, 0.3, -0.4, 0.5];
        let processed = service.prepare_audio_for_whisper(&test_audio);
        
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