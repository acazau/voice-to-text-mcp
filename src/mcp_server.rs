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

use crate::VoiceToTextService;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TranscribeFileRequest {
    #[schemars(description = "Path to the audio file to transcribe")]
    pub file_path: String,
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

    #[tool(description = "Start live audio recording")]
    async fn start_recording(&self) -> String {
        let service = self.service.lock().await;
        match service.start_listening().await {
            Ok(message) => message,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Stop live audio recording and get transcription")]
    async fn stop_recording(&self) -> String {
        let service = self.service.lock().await;
        match service.stop_listening().await {
            Ok(text) => text,
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Get current recording status")]
    async fn get_recording_status(&self) -> String {
        let service = self.service.lock().await;
        format!("Recording: {}, Samples: {}", 
                service.is_recording(), 
                service.get_audio_sample_count())
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
            "start_recording" => {
                let result = self.start_recording().await;
                Ok(CallToolResult::success(vec![Content::text(result)]))
            },
            "stop_recording" => {
                let result = self.stop_recording().await;
                Ok(CallToolResult::success(vec![Content::text(result)]))
            },
            "get_recording_status" => {
                let result = self.get_recording_status().await;
                Ok(CallToolResult::success(vec![Content::text(result)]))
            },
            _ => Err(rmcp::Error::method_not_found::<rmcp::model::CallToolRequestMethod>()),
        }
    }
}

pub async fn run_mcp_server(service: VoiceToTextService) -> Result<()> {
    eprintln!("Voice-to-Text MCP Server started with rmcp 0.2.1");
    
    let server = VoiceToTextMcpServer::new(service);
    
    // Create stdio transport
    let transport = (tokio::io::stdin(), tokio::io::stdout());
    
    // Serve the service using rmcp
    let server_instance = server.serve(transport).await?;
    
    // Wait for service shutdown
    let _quit_reason = server_instance.waiting().await?;
    
    Ok(())
}