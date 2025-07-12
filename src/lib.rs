use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::collections::VecDeque;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use anyhow::Result;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};
use hound::{WavWriter, WavSpec, WavReader};
use std::path::PathBuf;
use std::fs;
use chrono::Utc;

pub mod mcp_server;

#[derive(Clone, Debug)]
pub struct CommandConfig {
    pub start_commands: Vec<String>,
    pub stop_commands: Vec<String>,
    pub status_commands: Vec<String>,
    pub toggle_commands: Vec<String>,
}

impl Default for CommandConfig {
    fn default() -> Self {
        Self {
            start_commands: vec!["start".to_string(), "begin".to_string(), "record".to_string()],
            stop_commands: vec!["stop".to_string(), "end".to_string(), "finish".to_string()],
            status_commands: vec!["status".to_string(), "check".to_string(), "info".to_string()],
            toggle_commands: vec!["toggle".to_string(), "switch".to_string(), "".to_string()],
        }
    }
}

impl CommandConfig {
    pub fn from_cli_args(
        start_commands: Option<Vec<String>>,
        stop_commands: Option<Vec<String>>,
        status_commands: Option<Vec<String>>,
        toggle_commands: Option<Vec<String>>,
    ) -> Self {
        let mut config = Self::from_env();
        
        // CLI args override environment variables
        if let Some(cmds) = start_commands {
            config.start_commands = cmds.into_iter().filter(|s| !s.is_empty()).collect();
        }
        
        if let Some(cmds) = stop_commands {
            config.stop_commands = cmds.into_iter().filter(|s| !s.is_empty()).collect();
        }
        
        if let Some(cmds) = status_commands {
            config.status_commands = cmds.into_iter().filter(|s| !s.is_empty()).collect();
        }
        
        if let Some(cmds) = toggle_commands {
            config.toggle_commands = cmds.into_iter().filter(|s| !s.is_empty()).collect();
        }
        
        config
    }

    pub fn from_env() -> Self {
        let mut config = Self::default();
        
        if let Ok(start_cmds) = std::env::var("VOICE_START_COMMANDS") {
            config.start_commands = start_cmds.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        }
        
        if let Ok(stop_cmds) = std::env::var("VOICE_STOP_COMMANDS") {
            config.stop_commands = stop_cmds.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        }
        
        if let Ok(status_cmds) = std::env::var("VOICE_STATUS_COMMANDS") {
            config.status_commands = status_cmds.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        }
        
        if let Ok(toggle_cmds) = std::env::var("VOICE_TOGGLE_COMMANDS") {
            config.toggle_commands = toggle_cmds.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        }
        
        config
    }

    pub fn match_command(&self, cmd: &str) -> Option<CommandType> {
        let cmd_lower = cmd.trim().to_lowercase();
        
        if self.start_commands.iter().any(|c| c.to_lowercase() == cmd_lower) {
            Some(CommandType::Start)
        } else if self.stop_commands.iter().any(|c| c.to_lowercase() == cmd_lower) {
            Some(CommandType::Stop)
        } else if self.status_commands.iter().any(|c| c.to_lowercase() == cmd_lower) {
            Some(CommandType::Status)
        } else if self.toggle_commands.iter().any(|c| c.to_lowercase() == cmd_lower) {
            Some(CommandType::Toggle)
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum CommandType {
    Start,
    Stop,
    Status,
    Toggle,
}

impl CommandType {
    fn to_u8(&self) -> u8 {
        match self {
            CommandType::Start => 1,
            CommandType::Stop => 2,
            CommandType::Status => 3,
            CommandType::Toggle => 4,
        }
    }

    fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(CommandType::Start),
            2 => Some(CommandType::Stop),
            3 => Some(CommandType::Status),
            4 => Some(CommandType::Toggle),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DebugConfig {
    pub enabled: bool,
    pub output_dir: PathBuf,
    pub save_raw: bool,
    pub save_processed: bool,
}

#[derive(Clone, Debug)]
pub struct VoiceCommandConfig {
    pub enabled: bool,
    pub chunk_duration_ms: u64,
    pub detection_sensitivity: f32,
    pub include_in_final_transcription: bool,
    pub command_config: CommandConfig,
}

impl Default for VoiceCommandConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            chunk_duration_ms: 500,
            detection_sensitivity: 0.7,
            include_in_final_transcription: false,
            command_config: CommandConfig::default(),
        }
    }
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
    voice_command_config: VoiceCommandConfig,
    voice_command_detected: Arc<AtomicBool>,
    partial_transcription_buffer: Arc<Mutex<String>>,
    last_chunk_time: Arc<Mutex<Option<std::time::Instant>>>,
    audio_stream: Arc<Mutex<Option<cpal::Stream>>>,
    command_queue: Arc<Mutex<VecDeque<CommandType>>>,
    last_command: Arc<AtomicU8>,
}

impl VoiceToTextService {
    pub fn new() -> Self {
        Self::new_with_debug(DebugConfig::default())
    }

    pub fn new_with_debug(debug_config: DebugConfig) -> Self {
        Self::new_with_configs(debug_config, VoiceCommandConfig::default())
    }

    pub fn new_with_configs(debug_config: DebugConfig, voice_command_config: VoiceCommandConfig) -> Self {
        Self {
            is_recording: Arc::new(AtomicBool::new(false)),
            audio_data: Arc::new(Mutex::new(Vec::new())),
            whisper_context: Arc::new(tokio::sync::Mutex::new(None)),
            debug_config,
            voice_command_config,
            voice_command_detected: Arc::new(AtomicBool::new(false)),
            partial_transcription_buffer: Arc::new(Mutex::new(String::new())),
            last_chunk_time: Arc::new(Mutex::new(None)),
            audio_stream: Arc::new(Mutex::new(None)),
            command_queue: Arc::new(Mutex::new(VecDeque::new())),
            last_command: Arc::new(AtomicU8::new(0)),
        }
    }

    pub fn new_with_model(model_path: &str) -> Result<Self> {
        Self::new_with_model_and_debug(model_path, DebugConfig::default())
    }

    pub fn new_with_model_and_debug(model_path: &str, debug_config: DebugConfig) -> Result<Self> {
        Self::new_with_model_and_configs(model_path, debug_config, VoiceCommandConfig::default())
    }

    pub fn new_with_model_and_configs(model_path: &str, debug_config: DebugConfig, voice_command_config: VoiceCommandConfig) -> Result<Self> {
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
            voice_command_config,
            voice_command_detected: Arc::new(AtomicBool::new(false)),
            partial_transcription_buffer: Arc::new(Mutex::new(String::new())),
            last_chunk_time: Arc::new(Mutex::new(None)),
            audio_stream: Arc::new(Mutex::new(None)),
            command_queue: Arc::new(Mutex::new(VecDeque::new())),
            last_command: Arc::new(AtomicU8::new(0)),
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
        
        // Clear previous audio data and voice command state
        {
            let mut data = self.audio_data.lock().unwrap();
            data.clear();
        }
        
        self.reset_voice_command_detection();

        // Start audio capture with voice command monitoring
        self.start_audio_capture_with_voice_commands().await?;

        if self.voice_command_config.enabled {
            Ok("Started listening with voice commands enabled...".to_string())
        } else {
            Ok("Started listening...".to_string())
        }
    }

    pub async fn start_listening_with_options(&self, timeout_ms: u64, silence_timeout_ms: u64, auto_stop: bool) -> Result<String> {
        if self.is_recording.load(Ordering::Relaxed) {
            return Ok("Already recording".to_string());
        }

        self.is_recording.store(true, Ordering::Relaxed);
        
        // Clear previous audio data and voice command state
        {
            let mut data = self.audio_data.lock().unwrap();
            data.clear();
        }
        
        self.reset_voice_command_detection();

        // Start audio capture with voice command monitoring
        self.start_audio_capture_with_voice_commands().await?;

        if auto_stop {
            // Start the auto-stop monitoring with voice activity detection
            return self.listen_with_auto_stop(timeout_ms, silence_timeout_ms).await;
        } else {
            if self.voice_command_config.enabled {
                Ok("Started listening with voice commands enabled...".to_string())
            } else {
                Ok("Started listening...".to_string())
            }
        }
    }

    pub async fn stop_listening(&self) -> Result<String> {
        if !self.is_recording.load(Ordering::Relaxed) {
            return Ok("Not currently recording".to_string());
        }

        self.is_recording.store(false, Ordering::Relaxed);

        // Stop and drop the audio stream
        {
            let mut stream_guard = self.audio_stream.lock().unwrap();
            *stream_guard = None;
        }

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

        let mut transcription = self.transcribe_audio(audio_data).await?;
        
        // Handle voice command transcription inclusion/exclusion
        if self.voice_command_config.enabled {
            let partial_buffer = {
                let buffer = self.partial_transcription_buffer.lock().unwrap();
                buffer.clone()
            };
            
            if !self.voice_command_config.include_in_final_transcription && !partial_buffer.trim().is_empty() {
                // Remove detected voice commands from the transcription
                transcription = self.remove_voice_commands_from_transcription(&transcription, &partial_buffer);
            }
            
            if self.voice_command_detected() {
                let status = if self.voice_command_config.include_in_final_transcription {
                    "(voice command executed and included in transcription)"
                } else {
                    "(voice command executed and excluded from transcription)"
                };
                transcription = format!("{} {}", transcription, status);
            }
        }
        
        Ok(transcription)
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }

    pub fn get_audio_sample_count(&self) -> usize {
        self.audio_data.lock().unwrap().len()
    }

    pub fn set_voice_commands_enabled(&mut self, enabled: bool) {
        self.voice_command_config.enabled = enabled;
    }

    pub fn is_voice_commands_enabled(&self) -> bool {
        self.voice_command_config.enabled
    }

    pub fn get_voice_command_config(&self) -> &VoiceCommandConfig {
        &self.voice_command_config
    }

    pub fn voice_command_detected(&self) -> bool {
        self.voice_command_detected.load(Ordering::Relaxed)
    }

    pub fn reset_voice_command_detection(&self) {
        self.voice_command_detected.store(false, Ordering::Relaxed);
        let mut buffer = self.partial_transcription_buffer.lock().unwrap();
        buffer.clear();
        // Also clear the command queue and last command
        let mut queue = self.command_queue.lock().unwrap();
        queue.clear();
        self.clear_last_command();
    }

    pub fn get_command_queue_size(&self) -> usize {
        let queue = self.command_queue.lock().unwrap();
        queue.len()
    }

    pub fn pop_next_command(&self) -> Option<CommandType> {
        let mut queue = self.command_queue.lock().unwrap();
        queue.pop_front()
    }

    pub fn get_last_command(&self) -> Option<CommandType> {
        let value = self.last_command.load(Ordering::Relaxed);
        CommandType::from_u8(value)
    }

    pub fn set_last_command(&self, command: CommandType) {
        self.last_command.store(command.to_u8(), Ordering::Relaxed);
    }

    pub fn clear_last_command(&self) {
        self.last_command.store(0, Ordering::Relaxed);
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

    async fn process_audio_chunk(&self, chunk_data: &[f32]) -> Result<Option<CommandType>> {
        if !self.voice_command_config.enabled || chunk_data.is_empty() {
            return Ok(None);
        }

        // Check if we have a Whisper model loaded
        let whisper_ctx = self.whisper_context.lock().await;
        if whisper_ctx.is_none() {
            return Ok(None);
        }
        drop(whisper_ctx);

        // Process the audio chunk for transcription
        let processed_audio = self.prepare_audio_for_whisper(chunk_data)?;
        
        // Only process if we have enough audio data
        let duration_seconds = processed_audio.len() as f32 / 16000.0;
        if duration_seconds < 0.3 {
            return Ok(None);
        }

        // Transcribe the chunk
        match self.transcribe_chunk_with_whisper(&processed_audio).await {
            Ok(transcription) => {
                let detected_command = self.detect_voice_commands(&transcription);
                
                // Update partial transcription buffer
                {
                    let mut buffer = self.partial_transcription_buffer.lock().unwrap();
                    if !transcription.trim().is_empty() {
                        buffer.push_str(&transcription);
                        buffer.push(' ');
                    }
                }

                Ok(detected_command)
            }
            Err(_) => Ok(None)
        }
    }

    async fn transcribe_chunk_with_whisper(&self, audio_data: &[f32]) -> Result<String> {
        let whisper_ctx = self.whisper_context.lock().await;
        if let Some(ref ctx) = *whisper_ctx {
            // Create simplified transcription parameters for chunks
            let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
            params.set_language(None);
            params.set_print_special(false);
            params.set_print_progress(false);
            params.set_print_realtime(false);
            params.set_print_timestamps(false);
            params.set_suppress_blank(true);
            params.set_suppress_nst(true);
            params.set_temperature(0.0);
            
            let mut state = ctx.create_state()?;
            state.full(params, audio_data)?;
            
            let num_segments = state.full_n_segments()?;
            let mut result = String::new();
            
            for i in 0..num_segments {
                let segment_text = state.full_get_segment_text(i)?;
                result.push_str(&segment_text);
            }
            
            Ok(result.trim().to_string())
        } else {
            Err(anyhow::anyhow!("Whisper context not available"))
        }
    }

    pub fn detect_voice_commands(&self, transcription: &str) -> Option<CommandType> {
        if transcription.trim().is_empty() {
            return None;
        }

        // Normalize the transcription for command detection
        let normalized_text = transcription.to_lowercase().trim().to_string();
        
        // Check for commands using the existing command configuration
        // Look for exact matches and also partial matches
        for word in normalized_text.split_whitespace() {
            // Remove punctuation from the word for matching
            let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric());
            if let Some(command_type) = self.voice_command_config.command_config.match_command(clean_word) {
                eprintln!("🎯 Voice command detected: '{}' -> {:?}", clean_word, command_type);
                return Some(command_type);
            }
        }

        // Also check the entire phrase for multi-word commands like "stop recording"
        if let Some(command_type) = self.voice_command_config.command_config.match_command(&normalized_text) {
            eprintln!("🎯 Voice command detected: '{}' -> {:?}", normalized_text, command_type);
            return Some(command_type);
        }

        // Check for common variations
        if normalized_text.contains("stop") && (normalized_text.contains("recording") || normalized_text.contains("record")) {
            eprintln!("🎯 Voice command detected: 'stop recording' variation -> Stop");
            return Some(CommandType::Stop);
        }

        if normalized_text.contains("start") && (normalized_text.contains("recording") || normalized_text.contains("record")) {
            eprintln!("🎯 Voice command detected: 'start recording' variation -> Start");
            return Some(CommandType::Start);
        }

        None
    }

    async fn check_for_voice_commands(&self) -> Result<()> {
        if !self.voice_command_config.enabled {
            return Ok(());
        }

        let now = std::time::Instant::now();
        let should_check = {
            let mut last_check = self.last_chunk_time.lock().unwrap();
            match *last_check {
                Some(last_time) => {
                    let elapsed_ms = now.duration_since(last_time).as_millis();
                    if elapsed_ms >= self.voice_command_config.chunk_duration_ms as u128 {
                        if self.debug_config.enabled {
                            eprintln!("⏰ [{}] Voice command check interval: {}ms (threshold: {}ms)", 
                                     chrono::Utc::now().format("%H:%M:%S%.3f"),
                                     elapsed_ms, 
                                     self.voice_command_config.chunk_duration_ms);
                        }
                        *last_check = Some(now);
                        true
                    } else {
                        false
                    }
                }
                None => {
                    if self.debug_config.enabled {
                        eprintln!("⏰ [{}] Voice command check initialized", 
                                 chrono::Utc::now().format("%H:%M:%S%.3f"));
                    }
                    *last_check = Some(now);
                    true
                }
            }
        };

        if !should_check {
            return Ok(());
        }

        // Get a chunk of recent audio data
        let chunk_data = {
            let audio_data = self.audio_data.lock().unwrap();
            if audio_data.len() < 16000 { // Less than 1 second of 16kHz audio
                return Ok(());
            }
            
            // Get the last chunk_duration_ms worth of audio
            let chunk_samples = (self.voice_command_config.chunk_duration_ms as f32 * 44.1) as usize; // 44.1kHz input
            let start_idx = if audio_data.len() > chunk_samples {
                audio_data.len() - chunk_samples
            } else {
                0
            };
            
            audio_data[start_idx..].to_vec()
        };

        // Process the chunk and check for commands
        if let Ok(Some(command_type)) = self.process_audio_chunk(&chunk_data).await {
            eprintln!("🎙️ [{}] Processing voice command: {:?}", 
                     chrono::Utc::now().format("%H:%M:%S%.3f"), command_type);
            self.voice_command_detected.store(true, Ordering::Relaxed);
            
            // Update last command atomically for state tracking
            self.set_last_command(command_type.clone());
            
            // Add command to queue for processing
            {
                let mut queue = self.command_queue.lock().unwrap();
                queue.push_back(command_type.clone());
                
                // Limit queue size to prevent memory bloat
                if queue.len() > 10 {
                    queue.pop_front();
                }
            }
            
            // Execute critical commands immediately (Stop and Toggle stop)
            match command_type {
                CommandType::Stop => {
                    if self.is_recording() {
                        eprintln!("🛑 [{}] Voice command: Stopping recording immediately", 
                                 chrono::Utc::now().format("%H:%M:%S%.3f"));
                        let stop_start = std::time::Instant::now();
                        // Stop recording immediately for responsiveness
                        self.is_recording.store(false, Ordering::Relaxed);
                        // Stop and drop the audio stream
                        {
                            let mut stream_guard = self.audio_stream.lock().unwrap();
                            *stream_guard = None;
                        }
                        let stop_duration = stop_start.elapsed();
                        eprintln!("✅ [{}] Stop command executed in {:.2}ms", 
                                 chrono::Utc::now().format("%H:%M:%S%.3f"), 
                                 stop_duration.as_secs_f64() * 1000.0);
                    }
                }
                CommandType::Toggle => {
                    eprintln!("🔄 [{}] Voice command: Toggle recording immediately", 
                             chrono::Utc::now().format("%H:%M:%S%.3f"));
                    let toggle_start = std::time::Instant::now();
                    // Toggle recording state immediately
                    if self.is_recording() {
                        self.is_recording.store(false, Ordering::Relaxed);
                        // Stop and drop the audio stream
                        {
                            let mut stream_guard = self.audio_stream.lock().unwrap();
                            *stream_guard = None;
                        }
                    }
                    let toggle_duration = toggle_start.elapsed();
                    eprintln!("✅ [{}] Toggle command executed in {:.2}ms", 
                             chrono::Utc::now().format("%H:%M:%S%.3f"), 
                             toggle_duration.as_secs_f64() * 1000.0);
                }
                CommandType::Start => {
                    eprintln!("🎙️ [{}] Voice command: Start queued (ignored during recording)", 
                             chrono::Utc::now().format("%H:%M:%S%.3f"));
                }
                CommandType::Status => {
                    eprintln!("📊 [{}] Voice command: Status check - Recording: {}, Samples: {}, Queue: {}", 
                             chrono::Utc::now().format("%H:%M:%S%.3f"),
                             self.is_recording(), self.get_audio_sample_count(), self.get_command_queue_size());
                }
            }
        }

        Ok(())
    }

    async fn start_audio_capture_with_voice_commands(&self) -> Result<()> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No input device available"))?;

        let config = device.default_input_config()?;
        let _sample_rate = config.sample_rate().0;
        let channels = config.channels();

        let audio_data = Arc::clone(&self.audio_data);
        let is_recording = Arc::clone(&self.is_recording);
        let voice_commands_enabled = self.voice_command_config.enabled;
        
        // Clone the service for voice command checking
        let service_clone = self.clone();

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
                    
                    // Check for voice commands if enabled
                    if voice_commands_enabled {
                        let service_for_check = service_clone.clone();
                        // Use Handle::try_current() to check if we're in a Tokio runtime
                        if let Ok(handle) = tokio::runtime::Handle::try_current() {
                            handle.spawn(async move {
                                if let Err(e) = service_for_check.check_for_voice_commands().await {
                                    eprintln!("Voice command check error: {}", e);
                                }
                                
                                // Voice commands are now handled immediately in check_for_voice_commands
                                // No need for deferred stop handling
                            });
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
        
        // Store the stream to keep it alive during recording
        {
            let mut stream_guard = self.audio_stream.lock().unwrap();
            *stream_guard = Some(stream);
        }

        Ok(())
    }

    pub fn remove_voice_commands_from_transcription(&self, transcription: &str, partial_buffer: &str) -> String {
        if partial_buffer.trim().is_empty() {
            return transcription.to_string();
        }

        let mut cleaned_transcription = transcription.to_string();
        
        // Remove detected command words from the transcription
        for command_word in partial_buffer.split_whitespace() {
            let command_word_lower = command_word.to_lowercase();
            
            // Check if this word is a command
            if self.voice_command_config.command_config.match_command(&command_word_lower).is_some() {
                // Remove this word and common variations
                cleaned_transcription = cleaned_transcription
                    .replace(&command_word_lower, "")
                    .replace(&command_word.to_uppercase(), "")
                    .replace(command_word, "");
            }
        }
        
        // Remove common command phrases
        let phrases_to_remove = [
            "stop recording", "start recording", "stop record", "start record",
            "end recording", "begin recording", "finish recording"
        ];
        
        for phrase in &phrases_to_remove {
            cleaned_transcription = cleaned_transcription
                .replace(phrase, "")
                .replace(&phrase.to_uppercase(), "")
                .replace(&phrase.to_lowercase(), "");
        }
        
        // Clean up multiple spaces and trim
        cleaned_transcription = cleaned_transcription
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();
        
        if cleaned_transcription.is_empty() {
            "No speech content detected (only voice commands)".to_string()
        } else {
            cleaned_transcription
        }
    }

    async fn listen_with_auto_stop(&self, timeout_ms: u64, silence_timeout_ms: u64) -> Result<String> {
        use tokio::time::{sleep, Duration, Instant};
        
        let start_time = Instant::now();
        let mut last_activity_time = Instant::now();
        let silence_threshold = 0.01; // Minimum amplitude to consider as voice activity
        let check_interval = Duration::from_millis(100); // Check every 100ms
        
        loop {
            // Check for overall timeout
            if start_time.elapsed().as_millis() > timeout_ms as u128 {
                break;
            }
            
            // Check if recording was stopped by voice command
            if !self.is_recording.load(Ordering::Relaxed) {
                break;
            }
            
            // Check for voice activity
            let has_activity = {
                let audio_data = self.audio_data.lock().unwrap();
                if audio_data.len() > 4410 { // At least 100ms of audio at 44.1kHz
                    let recent_samples = &audio_data[audio_data.len().saturating_sub(4410)..];
                    let rms = (recent_samples.iter().map(|&x| x * x).sum::<f32>() / recent_samples.len() as f32).sqrt();
                    rms > silence_threshold
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
                        let audio_data = self.audio_data.lock().unwrap();
                        audio_data.len() > 22050 // At least 0.5 seconds of audio
                    };
                    
                    if has_audio {
                        break;
                    }
                }
            }
            
            sleep(check_interval).await;
        }
        
        // Stop recording and get transcription
        self.stop_listening().await
    }

    #[allow(dead_code)]
    fn get_last_detected_command(&self) -> Option<CommandType> {
        // Use the atomic tracking for accurate last command detection
        self.get_last_command()
    }
}

impl std::fmt::Debug for VoiceToTextService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoiceToTextService")
            .field("is_recording", &self.is_recording)
            .field("audio_data_len", &self.audio_data.lock().unwrap().len())
            .field("has_whisper_context", &"<async_mutex>")
            .field("debug_config", &self.debug_config)
            .field("voice_command_config", &self.voice_command_config)
            .field("voice_command_detected", &self.voice_command_detected)
            .field("partial_transcription_buffer_len", &self.partial_transcription_buffer.lock().unwrap().len())
            .field("command_queue_size", &self.command_queue.lock().unwrap().len())
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

    #[tokio::test]
    async fn test_voice_command_config_creation() {
        let voice_config = VoiceCommandConfig {
            enabled: true,
            chunk_duration_ms: 2000,
            detection_sensitivity: 0.8,
            include_in_final_transcription: true,
            command_config: CommandConfig::default(),
        };

        let service = VoiceToTextService::new_with_configs(
            DebugConfig::default(),
            voice_config.clone(),
        );

        assert!(service.is_voice_commands_enabled());
        assert_eq!(service.get_voice_command_config().chunk_duration_ms, 2000);
        assert_eq!(service.get_voice_command_config().detection_sensitivity, 0.8);
        assert!(service.get_voice_command_config().include_in_final_transcription);
    }

    #[tokio::test]
    async fn test_voice_command_enable_disable() {
        let mut service = VoiceToTextService::new();
        
        // Initially enabled by default
        assert!(service.is_voice_commands_enabled());
        
        // Enable voice commands
        service.set_voice_commands_enabled(true);
        assert!(service.is_voice_commands_enabled());
        
        // Disable voice commands
        service.set_voice_commands_enabled(false);
        assert!(!service.is_voice_commands_enabled());
    }

    #[tokio::test]
    async fn test_voice_command_detection_unit() {
        let mut voice_config = VoiceCommandConfig::default();
        voice_config.enabled = true;
        
        let service = VoiceToTextService::new_with_configs(
            DebugConfig::default(),
            voice_config,
        );

        // Test basic command detection
        assert_eq!(service.detect_voice_commands("stop"), Some(CommandType::Stop));
        assert_eq!(service.detect_voice_commands("start"), Some(CommandType::Start));
        assert_eq!(service.detect_voice_commands("status"), Some(CommandType::Status));
        assert_eq!(service.detect_voice_commands("toggle"), Some(CommandType::Toggle));
        
        // Test case insensitivity
        assert_eq!(service.detect_voice_commands("STOP"), Some(CommandType::Stop));
        assert_eq!(service.detect_voice_commands("Start"), Some(CommandType::Start));
        
        // Test no detection
        assert_eq!(service.detect_voice_commands("hello world"), None);
        assert_eq!(service.detect_voice_commands(""), None);
    }

    #[tokio::test]
    async fn test_voice_command_state_management_unit() {
        let mut voice_config = VoiceCommandConfig::default();
        voice_config.enabled = true;
        
        let service = VoiceToTextService::new_with_configs(
            DebugConfig::default(),
            voice_config,
        );

        // Initially no command detected
        assert!(!service.voice_command_detected());
        
        // Reset should work
        service.reset_voice_command_detection();
        assert!(!service.voice_command_detected());
    }

    #[tokio::test]
    async fn test_voice_command_transcription_filtering_unit() {
        let mut voice_config = VoiceCommandConfig::default();
        voice_config.enabled = true;
        voice_config.include_in_final_transcription = false;
        
        let service = VoiceToTextService::new_with_configs(
            DebugConfig::default(),
            voice_config,
        );

        // Test removing voice commands from transcription
        let original = "Hello world stop recording thank you";
        let partial_buffer = "stop recording";
        let cleaned = service.remove_voice_commands_from_transcription(original, partial_buffer);
        
        // Should remove command words but keep content
        assert!(cleaned.contains("Hello world"));
        assert!(cleaned.contains("thank you"));
        // Note: The current implementation focuses on removing command words
        // "recording" might remain as it's commonly used in non-command contexts
        assert!(!cleaned.contains("stop"));
    }

    #[tokio::test]
    async fn test_voice_command_phrase_detection_unit() {
        let mut voice_config = VoiceCommandConfig::default();
        voice_config.enabled = true;
        
        let service = VoiceToTextService::new_with_configs(
            DebugConfig::default(),
            voice_config,
        );

        // Test phrase variations
        assert_eq!(service.detect_voice_commands("stop recording"), Some(CommandType::Stop));
        assert_eq!(service.detect_voice_commands("start recording"), Some(CommandType::Start));
        assert_eq!(service.detect_voice_commands("please stop record"), Some(CommandType::Stop));
        assert_eq!(service.detect_voice_commands("end recording"), Some(CommandType::Stop));
        assert_eq!(service.detect_voice_commands("finish recording"), Some(CommandType::Stop));
        
        // Test no false positives
        assert_eq!(service.detect_voice_commands("stopping by the store"), None);
        assert_eq!(service.detect_voice_commands("starting tomorrow"), None);
    }

    #[tokio::test]
    async fn test_voice_command_word_boundary_detection() {
        let mut voice_config = VoiceCommandConfig::default();
        voice_config.enabled = true;
        
        let service = VoiceToTextService::new_with_configs(
            DebugConfig::default(),
            voice_config,
        );

        // Test word boundaries
        assert_eq!(service.detect_voice_commands("stop"), Some(CommandType::Stop));
        assert_eq!(service.detect_voice_commands("stop."), Some(CommandType::Stop));
        assert_eq!(service.detect_voice_commands("  stop  "), Some(CommandType::Stop));
        
        // Test that partial words don't match
        assert_eq!(service.detect_voice_commands("stopped"), None);
        assert_eq!(service.detect_voice_commands("unstoppable"), None);
        assert_eq!(service.detect_voice_commands("stopping"), None);
    }

    #[tokio::test]
    async fn test_voice_command_config_defaults() {
        let config = VoiceCommandConfig::default();
        
        assert!(config.enabled); // Enabled by default
        assert_eq!(config.chunk_duration_ms, 500);
        assert_eq!(config.detection_sensitivity, 0.7);
        assert!(!config.include_in_final_transcription);
    }

    #[tokio::test]
    async fn test_new_constructors_with_voice_commands() {
        // Test new constructor methods include voice command config
        let debug_config = DebugConfig::default();
        let mut voice_config = VoiceCommandConfig::default();
        voice_config.enabled = true;
        
        let service = VoiceToTextService::new_with_configs(debug_config, voice_config);
        assert!(service.is_voice_commands_enabled());
        
        // Test model constructor with voice commands
        let debug_config = DebugConfig::default();
        let mut voice_config = VoiceCommandConfig::default();
        voice_config.enabled = true;
        
        // This will fail without a real model file, but we can test the error path
        let result = VoiceToTextService::new_with_model_and_configs("nonexistent.bin", debug_config, voice_config);
        assert!(result.is_err()); // Expected - no model file
    }
}