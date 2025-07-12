use crate::error::{Result, VoiceError};
use whisper_rs::{WhisperContext, WhisperContextParameters};

// Conditional logging macros
macro_rules! debug_println {
    ($debug_enabled:expr, $($arg:tt)*) => {
        if $debug_enabled {
            println!($($arg)*);
        }
    };
}

macro_rules! debug_eprintln {
    ($debug_enabled:expr, $($arg:tt)*) => {
        if $debug_enabled {
            eprintln!($($arg)*);
        }
    };
}

pub(crate) use debug_println;
pub(crate) use debug_eprintln;

/// Load a Whisper model with optional output suppression
pub fn load_whisper_context(model_path: &str, debug_enabled: bool) -> Result<WhisperContext> {
    log_acceleration_status(debug_enabled);
    
    if debug_enabled {
        // In debug mode, show all logs
        WhisperContext::new_with_params(
            model_path,
            WhisperContextParameters::default(),
        ).map_err(|e| VoiceError::WhisperModelLoad(e.to_string()))
    } else {
        // In non-debug mode, suppress whisper.cpp logs
        load_whisper_quietly(model_path)
    }
}

/// Create a Whisper state with optional output suppression
pub fn create_whisper_state(ctx: &WhisperContext, debug_enabled: bool) -> Result<whisper_rs::WhisperState> {
    if debug_enabled {
        ctx.create_state().map_err(|e| VoiceError::WhisperTranscription(e.to_string()))
    } else {
        create_state_quietly(ctx)
    }
}

/// Run transcription with optional output suppression
pub fn run_whisper_transcription(
    state: &mut whisper_rs::WhisperState, 
    params: whisper_rs::FullParams, 
    audio_data: &[f32],
    debug_enabled: bool
) -> Result<()> {
    if debug_enabled {
        state.full(params, audio_data)
            .map(|_| ())
            .map_err(|e| VoiceError::WhisperTranscription(e.to_string()))
    } else {
        run_transcription_quietly(state, params, audio_data)
    }
}

fn load_whisper_quietly(model_path: &str) -> Result<WhisperContext> {
    use gag::Gag;
    
    // Suppress stderr during model loading
    let _stderr_gag = Gag::stderr().map_err(|e| VoiceError::Platform(format!("Failed to redirect stderr: {}", e)))?;
    
    let ctx = WhisperContext::new_with_params(
        model_path,
        WhisperContextParameters::default(),
    ).map_err(|e| VoiceError::WhisperModelLoad(e.to_string()))?;
    
    // _stderr_gag is dropped here, restoring stderr automatically
    Ok(ctx)
}

fn create_state_quietly(ctx: &WhisperContext) -> Result<whisper_rs::WhisperState> {
    use gag::Gag;
    
    // Suppress stderr during state creation
    let _stderr_gag = Gag::stderr().map_err(|e| VoiceError::Platform(format!("Failed to redirect stderr: {}", e)))?;
    
    let state = ctx.create_state().map_err(|e| VoiceError::WhisperTranscription(e.to_string()))?;
    
    // _stderr_gag is dropped here, restoring stderr automatically
    Ok(state)
}

fn run_transcription_quietly(
    state: &mut whisper_rs::WhisperState, 
    params: whisper_rs::FullParams, 
    audio_data: &[f32]
) -> Result<()> {
    use gag::Gag;
    
    // Suppress stderr during transcription
    let _stderr_gag = Gag::stderr().map_err(|e| VoiceError::Platform(format!("Failed to redirect stderr: {}", e)))?;
    
    state.full(params, audio_data)
        .map(|_| ())
        .map_err(|e| VoiceError::WhisperTranscription(e.to_string()))?;
    
    // _stderr_gag is dropped here, restoring stderr automatically
    Ok(())
}

pub fn log_acceleration_status(debug_enabled: bool) {
    debug_println!(debug_enabled, "Initializing Whisper with hardware acceleration:");
    
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    debug_println!(debug_enabled, "  Platform: macOS Apple Silicon (Metal + CoreML enabled)");
    
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    debug_println!(debug_enabled, "  Platform: macOS Intel (Metal enabled)");
    
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    debug_println!(debug_enabled, "  Platform: Linux x86_64 (CUDA enabled)");
    
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    debug_println!(debug_enabled, "  Platform: Windows x86_64 (CUDA enabled)");
    
    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "x86_64")
    )))]
    debug_println!(debug_enabled, "  Platform: No hardware acceleration (CPU-only mode)");
}