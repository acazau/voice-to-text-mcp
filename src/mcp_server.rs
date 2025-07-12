use rmcp::{
    handler::server::{ServerHandler, tool::{ToolRouter, Parameters}},
    model::{ServerCapabilities, ServerInfo, ListToolsResult, CallToolResult, CallToolRequestParam, PaginatedRequestParam, Content},
    service::{ServiceExt, RequestContext, RoleServer},
    tool, tool_router,
};
use std::future::Future;
use serde::Deserialize;
use schemars::JsonSchema;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{VoiceToTextService, CommandConfig, CommandType};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TranscribeFileRequest {
    #[schemars(description = "Path to the audio file to transcribe")]
    pub file_path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListenRequest {
    #[schemars(description = "Command: 'start', 'stop', 'status', or empty for toggle")]
    pub command: Option<String>,
    #[schemars(description = "Enable voice command recognition for this session")]
    pub enable_voice_commands: Option<bool>,
    #[schemars(description = "Maximum recording duration in milliseconds (default: 30000)")]
    pub timeout_ms: Option<u64>,
    #[schemars(description = "Silence duration in milliseconds before auto-stop (default: 2000)")]
    pub silence_timeout_ms: Option<u64>,
    #[schemars(description = "Auto-stop recording on silence detection (default: true)")]
    pub auto_stop: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct VoiceToTextMcpServer {
    tool_router: ToolRouter<Self>,
    service: Arc<Mutex<VoiceToTextService>>,
    command_config: CommandConfig,
}

#[tool_router]
impl VoiceToTextMcpServer {
    pub fn new(service: VoiceToTextService) -> Self {
        Self::new_with_config(service, CommandConfig::default())
    }

    pub fn new_with_config(service: VoiceToTextService, command_config: CommandConfig) -> Self {
        Self {
            tool_router: Self::tool_router(),
            service: Arc::new(Mutex::new(service)),
            command_config,
        }
    }
    
    #[tool(description = "Transcribe an audio file to text using Whisper")]
    async fn transcribe_file(
        &self,
        Parameters(TranscribeFileRequest { file_path }): Parameters<TranscribeFileRequest>,
    ) -> String {
        let service = self.service.lock().await;
        match service.transcribe_wav_file(&file_path).await {
            Ok(text) => text,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Voice recording control: configurable commands for start/stop/status/toggle")]
    pub async fn listen(
        &self,
        Parameters(ListenRequest { command, enable_voice_commands, timeout_ms, silence_timeout_ms, auto_stop }): Parameters<ListenRequest>,
    ) -> String {
        let mut service = self.service.lock().await;
        
        // Update voice command setting if provided
        if let Some(voice_enabled) = enable_voice_commands {
            service.set_voice_commands_enabled(voice_enabled);
        }
        
        let cmd = command.as_deref().unwrap_or("");
        
        match self.command_config.match_command(cmd) {
            Some(CommandType::Start) => {
                let timeout = timeout_ms.unwrap_or(30000);
                let silence_timeout = silence_timeout_ms.unwrap_or(2000);
                // For explicit start commands, default auto_stop to true (blocking behavior)
                let auto_stop_enabled = auto_stop.unwrap_or(true);
                
                match service.start_listening_with_options(timeout, silence_timeout, auto_stop_enabled).await {
                    Ok(message) => message,
                    Err(e) => format!("Error: {}", e),
                }
            },
            Some(CommandType::Stop) => {
                match service.stop_listening().await {
                    Ok(text) => text,
                    Err(e) => format!("Error: {}", e),
                }
            },
            Some(CommandType::Status) => {
                format!("Recording: {}, Samples: {}, Voice Commands: {}", 
                        service.is_recording(), 
                        service.get_audio_sample_count(),
                        if service.is_voice_commands_enabled() { "enabled" } else { "disabled" })
            },
            Some(CommandType::Toggle) => {
                // Toggle behavior
                if service.is_recording() {
                    match service.stop_listening().await {
                        Ok(text) => text,
                        Err(e) => format!("Error: {}", e),
                    }
                } else {
                    let timeout = timeout_ms.unwrap_or(30000);
                    let silence_timeout = silence_timeout_ms.unwrap_or(2000);
                    // For toggle commands, default auto_stop to false for backwards compatibility
                    let auto_stop_enabled = auto_stop.unwrap_or(false);
                    
                    match service.start_listening_with_options(timeout, silence_timeout, auto_stop_enabled).await {
                        Ok(message) => message,
                        Err(e) => format!("Error: {}", e),
                    }
                }
            },
            None => {
                let available_commands = format!(
                    "Available commands:\n- Start: {}\n- Stop: {}\n- Status: {}\n- Toggle: {}",
                    self.command_config.start_commands.join(", "),
                    self.command_config.stop_commands.join(", "),
                    self.command_config.status_commands.join(", "),
                    self.command_config.toggle_commands.join(", ")
                );
                format!("Unknown command: '{}'. {}", cmd, available_commands)
            }
        }
    }
}

impl ServerHandler for VoiceToTextMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("A voice-to-text transcription server using Whisper".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, rmcp::Error> {
        let tools = self.tool_router.list_all();
        Ok(ListToolsResult { tools, next_cursor: None })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, rmcp::Error> {
        // Use the router to call the appropriate tool method
        match request.name.as_ref() {
            "transcribe_file" => {
                let file_path = request.arguments
                    .as_ref()
                    .and_then(|args| args.get("file_path"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| rmcp::Error::invalid_request("file_path parameter required", None))?;
                
                let result = self.transcribe_file(
                    Parameters(TranscribeFileRequest { file_path: file_path.to_string() })
                ).await;
                Ok(CallToolResult::success(vec![Content::text(result)]))
            },
            "listen" => {
                let command = request.arguments
                    .as_ref()
                    .and_then(|args| args.get("command"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                let enable_voice_commands = request.arguments
                    .as_ref()
                    .and_then(|args| args.get("enable_voice_commands"))
                    .and_then(|v| v.as_bool());
                
                let timeout_ms = request.arguments
                    .as_ref()
                    .and_then(|args| args.get("timeout_ms"))
                    .and_then(|v| v.as_u64());
                
                let silence_timeout_ms = request.arguments
                    .as_ref()
                    .and_then(|args| args.get("silence_timeout_ms"))
                    .and_then(|v| v.as_u64());
                
                let auto_stop = request.arguments
                    .as_ref()
                    .and_then(|args| args.get("auto_stop"))
                    .and_then(|v| v.as_bool());
                
                let result = self.listen(
                    Parameters(ListenRequest { command, enable_voice_commands, timeout_ms, silence_timeout_ms, auto_stop })
                ).await;
                Ok(CallToolResult::success(vec![Content::text(result)]))
            },
            _ => Err(rmcp::Error::method_not_found::<rmcp::model::CallToolRequestMethod>()),
        }
    }
}

pub async fn run_mcp_server(service: VoiceToTextService, command_config: CommandConfig) -> Result<()> {
    eprintln!("Voice-to-Text MCP Server started with rmcp 0.2.1");
    
    let server = VoiceToTextMcpServer::new_with_config(service, command_config);
    
    // Create stdio transport
    let transport = (tokio::io::stdin(), tokio::io::stdout());
    
    // Serve the service using rmcp
    let server_instance = server.serve(transport).await?;
    
    // Wait for service shutdown
    let _quit_reason = server_instance.waiting().await?;
    
    Ok(())
}