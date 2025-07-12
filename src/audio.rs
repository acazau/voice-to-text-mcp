use crate::config::*;
use crate::error::{Result, VoiceError};
use crate::platform::debug_eprintln;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavWriter, WavSpec, WavReader};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::path::PathBuf;
use std::fs;
use chrono::Utc;

pub struct AudioProcessor {
    debug_enabled: bool,
}

impl AudioProcessor {
    pub fn new(debug_enabled: bool) -> Self {
        Self { debug_enabled }
    }

    /// Prepare audio for Whisper transcription (convert to 16kHz mono)
    pub fn prepare_for_whisper(&self, audio_data: &[f32]) -> Result<Vec<f32>> {
        // Whisper expects 16kHz mono audio
        // Most audio capture happens at 44.1kHz, so we need to resample
        
        // First, resample from 44.1kHz to 16kHz using simple decimation
        let resampled = self.resample_audio(audio_data, DEFAULT_SAMPLE_RATE, WHISPER_SAMPLE_RATE);
        
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

    /// Resample audio from one sample rate to another
    pub fn resample_audio(&self, audio_data: &[f32], input_rate: u32, output_rate: u32) -> Vec<f32> {
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
        
        debug_eprintln!(self.debug_enabled, "🔄 Resampled {} samples ({}Hz) -> {} samples ({}Hz)", 
                audio_data.len(), input_rate, resampled.len(), output_rate);
        
        resampled
    }

    /// Check if recent audio contains voice activity
    pub fn has_voice_activity(&self, audio_data: &[f32]) -> bool {
        if audio_data.len() < samples_for_duration_ms(DEFAULT_SAMPLE_RATE, RECENT_SAMPLES_DURATION_MS) {
            return false;
        }
        
        let recent_samples = &audio_data[audio_data.len().saturating_sub(
            samples_for_duration_ms(DEFAULT_SAMPLE_RATE, RECENT_SAMPLES_DURATION_MS)
        )..];
        
        let rms = (recent_samples.iter().map(|&x| x * x).sum::<f32>() / recent_samples.len() as f32).sqrt();
        rms > SILENCE_THRESHOLD
    }

    /// Validate audio for transcription
    pub fn validate_audio(&self, audio_data: &[f32], sample_rate: u32) -> Result<()> {
        let duration_seconds = audio_data.len() as f32 / sample_rate as f32;
        let max_amplitude = audio_data.iter()
            .map(|&x| x.abs())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);
        let rms = (audio_data.iter().map(|&x| x * x).sum::<f32>() / audio_data.len() as f32).sqrt();
        
        debug_eprintln!(self.debug_enabled, "🎤 Audio stats: {:.2}s duration, max amplitude: {:.4}, RMS: {:.4}", 
                duration_seconds, max_amplitude, rms);
        
        // Check minimum requirements
        if duration_seconds < MIN_AUDIO_DURATION {
            return Err(VoiceError::AudioTooShort { duration: duration_seconds });
        }
        
        if max_amplitude < MIN_AUDIO_AMPLITUDE {
            return Err(VoiceError::AudioTooQuiet { amplitude: max_amplitude });
        }
        
        Ok(())
    }
}

pub struct AudioCapture {
    is_recording: Arc<AtomicBool>,
    audio_data: Arc<Mutex<Vec<f32>>>,
    audio_stream: Arc<Mutex<Option<cpal::Stream>>>,
    debug_enabled: bool,
}

impl AudioCapture {
    pub fn new(debug_enabled: bool) -> Self {
        Self {
            is_recording: Arc::new(AtomicBool::new(false)),
            audio_data: Arc::new(Mutex::new(Vec::new())),
            audio_stream: Arc::new(Mutex::new(None)),
            debug_enabled,
        }
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }

    pub fn get_audio_sample_count(&self) -> usize {
        self.audio_data.lock().unwrap().len()
    }

    pub fn start_capture(&self) -> Result<()> {
        if self.is_recording.load(Ordering::Relaxed) {
            return Err(VoiceError::AlreadyRecording);
        }

        self.is_recording.store(true, Ordering::Relaxed);
        
        // Clear previous audio data
        {
            let mut data = self.audio_data.lock().unwrap();
            data.clear();
        }

        // Start audio capture
        self.start_audio_stream()?;
        Ok(())
    }

    pub fn stop_capture(&self) -> Result<Vec<f32>> {
        if !self.is_recording.load(Ordering::Relaxed) {
            return Err(VoiceError::NotRecording);
        }

        self.is_recording.store(false, Ordering::Relaxed);

        // Stop and drop the audio stream
        {
            let mut stream_guard = self.audio_stream.lock().unwrap();
            *stream_guard = None;
        }

        // Get audio data
        let audio_data = {
            let data = self.audio_data.lock().unwrap();
            data.clone()
        };

        Ok(audio_data)
    }

    pub fn get_current_audio_data(&self) -> Vec<f32> {
        self.audio_data.lock().unwrap().clone()
    }

    fn start_audio_stream(&self) -> Result<()> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or(VoiceError::NoInputDevice)?;

        let config = device.default_input_config()?;
        let _sample_rate = config.sample_rate().0;
        let channels = config.channels();

        let audio_data = Arc::clone(&self.audio_data);
        let is_recording = Arc::clone(&self.is_recording);
        let debug_enabled = self.debug_enabled;

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
                debug_eprintln!(debug_enabled, "Audio stream error: {}", err);
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
}

pub struct AudioFileHandler {
    debug_config: DebugConfig,
}

impl AudioFileHandler {
    pub fn new(debug_config: DebugConfig) -> Self {
        Self { debug_config }
    }

    pub fn load_wav_file(&self, wav_path: &str) -> Result<Vec<f32>> {
        debug_eprintln!(self.debug_config.enabled, "📁 Loading WAV file: {}", wav_path);
        
        // Read the WAV file
        let mut reader = WavReader::open(wav_path)?;
        let spec = reader.spec();
        
        debug_eprintln!(self.debug_config.enabled, "🎵 WAV specs: {}Hz, {} channels, {} bits", 
                spec.sample_rate, spec.channels, spec.bits_per_sample);
        
        // Read all samples as f32
        let samples: std::result::Result<Vec<f32>, _> = match spec.sample_format {
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
        
        let mut audio_data = samples.map_err(|e| VoiceError::WavFile(e.to_string()))?;
        
        // Convert stereo to mono if needed
        if spec.channels == 2 {
            debug_eprintln!(self.debug_config.enabled, "🔄 Converting stereo to mono");
            let mono_data: Vec<f32> = audio_data.chunks(2)
                .map(|chunk| (chunk[0] + chunk.get(1).unwrap_or(&0.0)) / 2.0)
                .collect();
            audio_data = mono_data;
        }
        
        debug_eprintln!(self.debug_config.enabled, "📊 Loaded {} samples from WAV file", audio_data.len());
        Ok(audio_data)
    }

    pub fn save_debug_audio(&self, audio_data: &[f32], suffix: &str, sample_rate: u32) -> Result<()> {
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

        let mut writer = WavWriter::create(&filepath, spec)
            .map_err(|e| VoiceError::DebugFileSave(e.to_string()))?;
        
        for &sample in audio_data {
            writer.write_sample(sample)
                .map_err(|e| VoiceError::DebugFileSave(e.to_string()))?;
        }
        
        writer.finalize()
            .map_err(|e| VoiceError::DebugFileSave(e.to_string()))?;

        debug_eprintln!(self.debug_config.enabled, "🔧 Debug: Saved {} samples to {}", audio_data.len(), filepath.display());
        Ok(())
    }

    fn ensure_debug_directory(&self) -> Result<()> {
        if self.debug_config.enabled {
            fs::create_dir_all(&self.debug_config.output_dir)
                .map_err(|e| VoiceError::DebugDirectory(e.to_string()))?;
        }
        Ok(())
    }

    fn generate_debug_filename(&self, suffix: &str) -> PathBuf {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("audio_{}_{}.wav", timestamp, suffix);
        self.debug_config.output_dir.join(filename)
    }
}