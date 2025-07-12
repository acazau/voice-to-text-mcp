use voice_to_text_mcp::{VoiceToTextService, DebugConfig};
use voice_to_text_mcp::mcp_server::run_mcp_server;
use anyhow::Result;
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


    /// Maximum recording duration in milliseconds (default: 30000)
    #[arg(long, default_value = "30000")]
    timeout_ms: u64,

    /// Silence duration in milliseconds before auto-stop (default: 2000)
    #[arg(long, default_value = "2000")]
    silence_timeout_ms: u64,

    /// Disable automatic stopping on silence detection
    #[arg(long)]
    no_auto_stop: bool,
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

    
    // Create the voice service
    let service = if let Some(ref model_path) = args.model_path {
        if model_path.exists() {
            match VoiceToTextService::new_with_model_and_debug(model_path.to_str().unwrap(), debug_config.clone()) {
                Ok(service) => service,
                Err(e) => {
                    eprintln!("Error: Failed to load Whisper model: {}", e);
                    std::process::exit(1);
                }
            }
        } else {
            eprintln!("Error: Model file not found: {}", model_path.display());
            std::process::exit(1);
        }
    } else {
        // Model will be required for non-MCP mode, but create service anyway for MCP mode
        VoiceToTextService::new_with_debug(debug_config.clone())
    };

    // Check if running as MCP server
    if args.mcp_server {
        // Set environment variable to disable keyboard raw mode in MCP server mode
        std::env::set_var("MCP_SERVER_MODE", "1");
        // Run as MCP server
        return run_mcp_server(service).await;
    }
    
    // Run as blocking voice recorder (like the old voice-recorder binary)
    if args.model_path.is_none() {
        eprintln!("Error: Whisper model path is required");
        eprintln!("Usage: voice-to-text-mcp <MODEL_PATH>");
        std::process::exit(1);
    }

    // Record audio and get transcription (blocking operation)
    let auto_stop = !args.no_auto_stop;
    
    match service.start_listening_with_options(args.timeout_ms, args.silence_timeout_ms, auto_stop).await {
        Ok(transcription) => {
            // Print the transcription result to stdout
            println!("{}", transcription);
        }
        Err(e) => {
            eprintln!("Error: Failed to record audio: {}", e);
            std::process::exit(1);
        }
    }
    
    Ok(())
}