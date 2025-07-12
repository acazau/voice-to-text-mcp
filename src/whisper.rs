use crate::config::*;
use crate::error::{Result, VoiceError};
use crate::platform::{debug_eprintln, load_whisper_context, create_whisper_state, run_whisper_transcription};
use crate::audio::AudioProcessor;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext};

pub struct WhisperTranscriber {
    context: Option<WhisperContext>,
    audio_processor: AudioProcessor,
    debug_enabled: bool,
}

impl WhisperTranscriber {
    pub fn new(debug_enabled: bool) -> Self {
        Self {
            context: None,
            audio_processor: AudioProcessor::new(debug_enabled),
            debug_enabled,
        }
    }

    pub fn new_with_model(model_path: &str, debug_enabled: bool) -> Result<Self> {
        let context = load_whisper_context(model_path, debug_enabled)?;
        
        Ok(Self {
            context: Some(context),
            audio_processor: AudioProcessor::new(debug_enabled),
            debug_enabled,
        })
    }

    pub fn has_model(&self) -> bool {
        self.context.is_some()
    }

    pub async fn transcribe_audio(&self, audio_data: Vec<f32>) -> Result<String> {
        if audio_data.is_empty() {
            return Ok("No audio data recorded".to_string());
        }

        if let Some(ref ctx) = self.context {
            // Convert audio to the format Whisper expects (16kHz, mono)
            let processed_audio = self.audio_processor.prepare_for_whisper(&audio_data)?;
            
            // Validate the processed audio
            self.audio_processor.validate_audio(&processed_audio, WHISPER_SAMPLE_RATE)?;
            
            // Perform transcription
            self.transcribe_with_whisper(ctx, &processed_audio).await
        } else {
            Err(VoiceError::WhisperModelNotLoaded)
        }
    }

    pub async fn transcribe_with_validation(&self, audio_data: Vec<f32>) -> Result<String> {
        if audio_data.is_empty() {
            return Ok("No audio data recorded".to_string());
        }

        if let Some(ref ctx) = self.context {
            // Convert audio to the format Whisper expects (16kHz, mono)
            let processed_audio = self.audio_processor.prepare_for_whisper(&audio_data)?;
            
            // Perform transcription with enhanced result analysis
            self.transcribe_with_whisper(ctx, &processed_audio).await
        } else {
            // Fallback to placeholder if no model loaded
            Ok(format!("Transcribed {} audio samples (Whisper model not loaded - use new_with_model() to load a model)", audio_data.len()))
        }
    }

    async fn transcribe_with_whisper(&self, ctx: &WhisperContext, audio_data: &[f32]) -> Result<String> {
        // Audio validation and debugging
        let duration_seconds = audio_data.len() as f32 / WHISPER_SAMPLE_RATE as f32;
        let max_amplitude = audio_data.iter().map(|&x| x.abs()).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(0.0);
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
        
        debug_eprintln!(self.debug_enabled, "🤖 Running Whisper transcription...");
        
        // Create state and run the transcription
        let mut state = create_whisper_state(ctx, self.debug_enabled)?;
        run_whisper_transcription(&mut state, params, audio_data, self.debug_enabled)?;
        
        // Collect the transcribed text with detailed logging
        let num_segments = state.full_n_segments()?;
        debug_eprintln!(self.debug_enabled, "📝 Whisper found {} segments", num_segments);
        
        let mut result = String::new();
        let mut all_segments = Vec::new();
        
        for i in 0..num_segments {
            let segment_text = state.full_get_segment_text(i)?;
            let start_time = state.full_get_segment_t0(i)?;
            let end_time = state.full_get_segment_t1(i)?;
            
            debug_eprintln!(self.debug_enabled, "   Segment {}: [{:.2}s-{:.2}s] '{}'", i, start_time as f32 / 100.0, end_time as f32 / 100.0, segment_text);
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
    }

    pub fn get_audio_processor(&self) -> &AudioProcessor {
        &self.audio_processor
    }
}