use rmcp::{
    handler::server::{ServerHandler, tool::{ToolRouter, Parameters}},
    model::{ServerCapabilities, ServerInfo, ListToolsResult, CallToolResult, CallToolRequestParam, PaginatedRequestParam, Content},
    service::{ServiceExt, RequestContext, RoleServer},
    tool, tool_router,
};
use std::future::Future;
use serde::Deserialize;
use schemars::JsonSchema;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::VoiceToTextService;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TranscribeFileRequest {
    #[schemars(description = "Path to the audio file to transcribe")]
    pub file_path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListenRequest {
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
}

#[tool_router]
impl VoiceToTextMcpServer {
    pub fn new(service: VoiceToTextService) -> Self {
        Self {
            tool_router: Self::tool_router(),
            service: Arc::new(Mutex::new(service)),
        }
    }
    
    #[tool(description = "Transcribe an audio file to text using Whisper")]
    pub async fn transcribe_file(
        &self,
        Parameters(TranscribeFileRequest { file_path }): Parameters<TranscribeFileRequest>,
    ) -> String {
        let service = self.service.lock().await;
        match service.transcribe_wav_file(&file_path).await {
            Ok(text) => text,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Start recording audio and return transcribed text when complete")]
    pub async fn listen(
        &self,
        Parameters(ListenRequest { timeout_ms, silence_timeout_ms, auto_stop }): Parameters<ListenRequest>,
    ) -> String {
        // Get parameters with defaults
        let timeout = timeout_ms.unwrap_or(30000);
        let silence_timeout = silence_timeout_ms.unwrap_or(2000);
        let auto_stop_enabled = auto_stop.unwrap_or(true);

        // Get debug status first
        let debug_enabled = {
            let service = self.service.lock().await;
            service.get_debug_config().enabled
        };
        
        if debug_enabled {
            eprintln!("🎤 MCP: Starting voice recording with timeout: {}ms, silence_timeout: {}ms, auto_stop: {}", 
                     timeout, silence_timeout, auto_stop_enabled);
        }

        // Use the VoiceToTextService directly
        let service = self.service.lock().await;
        match service.start_listening_with_options(timeout, silence_timeout, auto_stop_enabled).await {
            Ok(text) => {
                if debug_enabled {
                    eprintln!("🎤 MCP: Recording completed successfully");
                }
                text
            }
            Err(e) => {
                if debug_enabled {
                    eprintln!("🎤 MCP: Recording failed: {}", e);
                }
                format!("Error: {}", e)
            }
        }
    }
}

impl ServerHandler for VoiceToTextMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> std::result::Result<ListToolsResult, rmcp::Error> {
        let tools = self.tool_router.list_all();
        Ok(ListToolsResult { tools, next_cursor: None })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> std::result::Result<CallToolResult, rmcp::Error> {
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
                    Parameters(ListenRequest { timeout_ms, silence_timeout_ms, auto_stop })
                ).await;
                Ok(CallToolResult::success(vec![Content::text(result)]))
            },
            _ => Err(rmcp::Error::method_not_found::<rmcp::model::CallToolRequestMethod>()),
        }
    }
}

pub async fn run_mcp_server(service: VoiceToTextService) -> anyhow::Result<()> {
    // Import the platform compatibility layer to enable Send/Sync on macOS
    #[allow(unused_imports)]
    use crate::platform_compat::*;
    
    if service.get_debug_config().enabled {
        eprintln!("Voice-to-Text MCP Server started with rmcp 0.2.1");
    }
    
    let server = VoiceToTextMcpServer::new(service);
    
    // Create stdio transport
    let transport = (tokio::io::stdin(), tokio::io::stdout());
    
    // Serve the service using rmcp
    let server_instance = server.serve(transport).await
        .map_err(|e| anyhow::anyhow!("Failed to start MCP server: {}", e))?;
    
    // Wait for service shutdown
    let _quit_reason = server_instance.waiting().await
        .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))?;
    
    Ok(())
}