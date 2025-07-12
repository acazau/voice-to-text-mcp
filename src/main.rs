use voice_to_text_mcp::{VoiceToTextService, DebugConfig, CommandConfig, VoiceCommandConfig};
use voice_to_text_mcp::mcp_server::run_mcp_server;
use anyhow::Result;
use std::io;
use std::path::PathBuf;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "voice-to-text-mcp")]
#[command(about = "A voice-to-text transcription server using Whisper", long_about = None)]
#[command(version)]
struct Args {
    /// Path to the Whisper model file (.bin format)
    #[arg(value_name = "MODEL_PATH")]
    model_path: Option<PathBuf>,

    /// Run as MCP server (communicates via stdio)
    #[arg(long)]
    mcp_server: bool,

    /// Enable debug mode to save WAV files for troubleshooting
    #[arg(short, long)]
    debug: bool,

    /// Directory to save debug audio files
    #[arg(long, value_name = "DIR", default_value = "./debug")]
    debug_dir: PathBuf,

    /// Save raw captured audio (only effective with --debug)
    #[arg(long, default_value = "true")]
    save_raw: bool,

    /// Save processed audio sent to Whisper (only effective with --debug)
    #[arg(long, default_value = "true")]
    save_processed: bool,

    /// Comma-separated list of start commands (e.g., "start,begin,record")
    #[arg(long, value_delimiter = ',')]
    start_commands: Option<Vec<String>>,

    /// Comma-separated list of stop commands (e.g., "stop,end,finish")
    #[arg(long, value_delimiter = ',')]
    stop_commands: Option<Vec<String>>,

    /// Comma-separated list of status commands (e.g., "status,check,info")
    #[arg(long, value_delimiter = ',')]
    status_commands: Option<Vec<String>>,

    /// Comma-separated list of toggle commands (e.g., "toggle,switch")
    #[arg(long, value_delimiter = ',')]
    toggle_commands: Option<Vec<String>>,

    /// Enable voice command recognition during recording
    #[arg(long)]
    voice_commands: bool,

    /// Duration in milliseconds for voice command detection chunks
    #[arg(long, default_value = "1500")]
    voice_chunk_duration: u64,

    /// Sensitivity for voice command detection (0.0-1.0)
    #[arg(long, default_value = "0.7")]
    voice_sensitivity: f32,

    /// Include detected voice commands in final transcription
    #[arg(long)]
    include_voice_commands: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Create debug configuration from CLI args and environment variables
    let env_debug = std::env::var("VOICE_DEBUG")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(false);
    
    let debug_config = DebugConfig {
        enabled: args.debug || env_debug,
        output_dir: if env_debug && !args.debug {
            // Use environment variable directory if set
            std::env::var("VOICE_DEBUG_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("./debug"))
        } else {
            args.debug_dir.clone()
        },
        save_raw: args.save_raw,
        save_processed: args.save_processed,
    };

    // Create command configuration from CLI args and environment variables
    let command_config = CommandConfig::from_cli_args(
        args.start_commands,
        args.stop_commands,
        args.status_commands,
        args.toggle_commands,
    );

    // Create voice command configuration from CLI args and environment variables
    let env_voice_commands = std::env::var("VOICE_COMMANDS_ENABLED")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(false);
    
    let env_chunk_duration = std::env::var("VOICE_CHUNK_DURATION")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1500);
    
    let env_sensitivity = std::env::var("VOICE_SENSITIVITY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.7);
    
    let env_include_commands = std::env::var("VOICE_INCLUDE_COMMANDS")
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(false);

    let voice_command_config = VoiceCommandConfig {
        enabled: if args.voice_commands || env_voice_commands {
            true
        } else if std::env::var("VOICE_COMMANDS_ENABLED").is_ok() {
            env_voice_commands
        } else {
            true  // Default to enabled when no explicit CLI args or env vars are set
        },
        chunk_duration_ms: if args.voice_chunk_duration != 1500 { args.voice_chunk_duration } else { env_chunk_duration },
        detection_sensitivity: if args.voice_sensitivity != 0.7 { args.voice_sensitivity } else { env_sensitivity },
        include_in_final_transcription: args.include_voice_commands || env_include_commands,
        command_config: command_config.clone(),
    };
    
    // Try to load a Whisper model if provided
    let service = if let Some(model_path) = args.model_path {
        if model_path.exists() {
            match VoiceToTextService::new_with_model_and_configs(model_path.to_str().unwrap(), debug_config.clone(), voice_command_config.clone()) {
                Ok(service) => {
                    if !args.mcp_server {
                        println!("✅ Whisper model loaded from: {}", model_path.display());
                        if voice_command_config.enabled {
                            println!("🎙️ Voice commands enabled (chunk: {}ms, sensitivity: {:.1})", 
                                   voice_command_config.chunk_duration_ms, voice_command_config.detection_sensitivity);
                        }
                    }
                    service
                }
                Err(e) => {
                    if !args.mcp_server {
                        println!("❌ Failed to load Whisper model: {}", e);
                        println!("   Falling back to placeholder mode");
                    }
                    VoiceToTextService::new_with_configs(debug_config.clone(), voice_command_config.clone())
                }
            }
        } else {
            if !args.mcp_server {
                println!("❌ Model file not found: {}", model_path.display());
                println!("   Falling back to placeholder mode");
            }
            VoiceToTextService::new_with_configs(debug_config.clone(), voice_command_config.clone())
        }
    } else {
        if !args.mcp_server {
            println!("💡 No Whisper model specified. Using placeholder mode.");
            println!("   To use actual transcription, run: cargo run -- <path-to-whisper-model>");
            println!("   To enable debug mode, set VOICE_DEBUG=true or use --debug");
            println!("   To enable voice commands, use --voice-commands");
            println!("   Download models from: https://huggingface.co/ggerganov/whisper.cpp");
        }
        VoiceToTextService::new_with_configs(debug_config.clone(), voice_command_config.clone())
    };

    // Check if running as MCP server
    if args.mcp_server {
        // Run as MCP server
        return run_mcp_server(service, command_config).await;
    }
    
    // Run as interactive CLI
    println!("Voice-to-Text MCP Server");
    println!("========================");
    
    if debug_config.enabled {
        println!("🔧 Debug mode enabled - audio files will be saved to: {}", debug_config.output_dir.display());
    }
    
    if voice_command_config.enabled {
        println!("🎙️ Voice commands enabled (chunk: {}ms, sensitivity: {:.1})", 
               voice_command_config.chunk_duration_ms, voice_command_config.detection_sensitivity);
        println!("   Voice commands: start, stop, status - speak these during recording");
    }
    
    println!("\nCommands:");
    println!("  'start' - Begin recording");
    println!("  'stop' - End recording and get transcription");
    println!("  'test <wav_file>' - Test transcription on existing WAV file");
    println!("  'status' - Check recording status");
    println!("  'quit' - Exit application");
    println!("\nTo run as MCP server: cargo run -- --mcp-server [model_path]");
    println!("To enable voice commands: cargo run -- --voice-commands [model_path]");
    
    loop {
        println!("\nEnter command:");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let command = input.trim().to_lowercase();
        
        if command.starts_with("test ") {
            let wav_file = command.strip_prefix("test ").unwrap().trim();
            if wav_file.is_empty() {
                println!("Usage: test <wav_file_path>");
                println!("Example: test debug/audio_20250710_194139_raw.wav");
            } else {
                match service.transcribe_wav_file(wav_file).await {
                    Ok(result) => println!("🎯 WAV Transcription: {}", result),
                    Err(e) => println!("❌ Error transcribing WAV file: {}", e),
                }
            }
        } else {
            match command.as_str() {
                "start" => {
                    match service.start_listening().await {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => println!("Error starting: {}", e),
                    }
                }
                "stop" => {
                    match service.stop_listening().await {
                        Ok(result) => println!("Transcription: {}", result),
                        Err(e) => println!("Error stopping: {}", e),
                    }
                }
                "status" => {
                    if service.is_recording() {
                        println!("Status: Recording ({} samples captured)", service.get_audio_sample_count());
                    } else {
                        println!("Status: Not recording");
                    }
                }
                "quit" | "exit" => {
                    println!("Goodbye!");
                    break;
                }
                _ => {
                    println!("Unknown command. Use 'start', 'stop', 'test <wav_file>', 'status', or 'quit'");
                }
            }
        }
    }
    
    Ok(())
}