use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::VoiceToTextService;

#[derive(Debug, Deserialize)]
pub struct TranscribeFileRequest {
    /// Path to the audio file to transcribe
    pub file_path: String,
}

#[derive(Debug, Serialize)]
pub struct TranscriptionResponse {
    pub text: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RecordingStatusResponse {
    pub is_recording: bool,
    pub sample_count: usize,
}

#[derive(Clone)]
pub struct VoiceToTextMcpServer {
    service: Arc<Mutex<VoiceToTextService>>,
}

impl VoiceToTextMcpServer {
    pub fn new(service: VoiceToTextService) -> Self {
        Self {
            service: Arc::new(Mutex::new(service)),
        }
    }
    
    pub async fn transcribe_file(&self, request: TranscribeFileRequest) -> TranscriptionResponse {
        let service = self.service.lock().await;
        match service.transcribe_wav_file(&request.file_path).await {
            Ok(text) => TranscriptionResponse {
                text,
                success: true,
                error: None,
            },
            Err(e) => TranscriptionResponse {
                text: String::new(),
                success: false,
                error: Some(e.to_string()),
            },
        }
    }

    pub async fn start_recording(&self) -> TranscriptionResponse {
        let service = self.service.lock().await;
        match service.start_listening().await {
            Ok(message) => TranscriptionResponse {
                text: message,
                success: true,
                error: None,
            },
            Err(e) => TranscriptionResponse {
                text: String::new(),
                success: false,
                error: Some(e.to_string()),
            },
        }
    }

    pub async fn stop_recording(&self) -> TranscriptionResponse {
        let service = self.service.lock().await;
        match service.stop_listening().await {
            Ok(text) => TranscriptionResponse {
                text,
                success: true,
                error: None,
            },
            Err(e) => TranscriptionResponse {
                text: String::new(),
                success: false,
                error: Some(e.to_string()),
            },
        }
    }

    pub async fn get_recording_status(&self) -> RecordingStatusResponse {
        let service = self.service.lock().await;
        RecordingStatusResponse {
            is_recording: service.is_recording(),
            sample_count: service.get_audio_sample_count(),
        }
    }
}

// For now, create a simple stdio-based server that handles basic commands
pub async fn run_mcp_server(service: VoiceToTextService) -> Result<()> {
    use tokio::io::{stdin, stdout, AsyncBufReadExt, AsyncWriteExt, BufReader};
    use serde_json::{Value, json};
    
    let server = VoiceToTextMcpServer::new(service);
    let mut reader = BufReader::new(stdin());
    let mut stdout = stdout();
    
    // Send server info on startup
    let server_info = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "result": {
            "name": "voice-to-text-mcp",
            "version": "0.1.0",
            "capabilities": {
                "tools": {
                    "transcribe_file": {
                        "description": "Transcribe an audio file to text",
                        "input_schema": {
                            "type": "object",
                            "properties": {
                                "file_path": {
                                    "type": "string",
                                    "description": "Path to the audio file to transcribe"
                                }
                            },
                            "required": ["file_path"]
                        }
                    },
                    "start_recording": {
                        "description": "Start live audio recording",
                        "input_schema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    "stop_recording": {
                        "description": "Stop live audio recording and get transcription",
                        "input_schema": {
                            "type": "object",
                            "properties": {}
                        }
                    },
                    "get_recording_status": {
                        "description": "Get current recording status",
                        "input_schema": {
                            "type": "object",
                            "properties": {}
                        }
                    }
                }
            }
        }
    });
    
    eprintln!("Voice-to-Text MCP Server started");
    
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                if let Ok(request) = serde_json::from_str::<Value>(&line) {
                    if let Some(method) = request.get("method").and_then(|m| m.as_str()) {
                        let id = request.get("id");
                        let params = request.get("params");
                        
                        let response = match method {
                            "tools/call" => {
                                if let Some(params) = params {
                                    if let Some(name) = params.get("name").and_then(|n| n.as_str()) {
                                        let empty_args = json!({});
                                        let arguments = params.get("arguments").unwrap_or(&empty_args);
                                        
                                        let result = match name {
                                            "transcribe_file" => {
                                                if let Ok(req) = serde_json::from_value::<TranscribeFileRequest>(arguments.clone()) {
                                                    let response = server.transcribe_file(req).await;
                                                    serde_json::to_value(response).unwrap()
                                                } else {
                                                    json!({"error": "Invalid arguments for transcribe_file"})
                                                }
                                            },
                                            "start_recording" => {
                                                let response = server.start_recording().await;
                                                serde_json::to_value(response).unwrap()
                                            },
                                            "stop_recording" => {
                                                let response = server.stop_recording().await;
                                                serde_json::to_value(response).unwrap()
                                            },
                                            "get_recording_status" => {
                                                let response = server.get_recording_status().await;
                                                serde_json::to_value(response).unwrap()
                                            },
                                            _ => json!({"error": format!("Unknown tool: {}", name)})
                                        };
                                        
                                        json!({
                                            "jsonrpc": "2.0",
                                            "id": id,
                                            "result": result
                                        })
                                    } else {
                                        json!({
                                            "jsonrpc": "2.0",
                                            "id": id,
                                            "error": {"code": -32600, "message": "Invalid Request: missing tool name"}
                                        })
                                    }
                                } else {
                                    json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "error": {"code": -32600, "message": "Invalid Request: missing params"}
                                    })
                                }
                            },
                            "tools/list" => {
                                json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "tools": [
                                            {
                                                "name": "transcribe_file",
                                                "description": "Transcribe an audio file to text",
                                                "inputSchema": {
                                                    "type": "object",
                                                    "properties": {
                                                        "file_path": {
                                                            "type": "string",
                                                            "description": "Path to the audio file to transcribe"
                                                        }
                                                    },
                                                    "required": ["file_path"]
                                                }
                                            },
                                            {
                                                "name": "start_recording",
                                                "description": "Start live audio recording",
                                                "inputSchema": {
                                                    "type": "object",
                                                    "properties": {}
                                                }
                                            },
                                            {
                                                "name": "stop_recording",
                                                "description": "Stop live audio recording and get transcription",
                                                "inputSchema": {
                                                    "type": "object",
                                                    "properties": {}
                                                }
                                            },
                                            {
                                                "name": "get_recording_status",
                                                "description": "Get current recording status",
                                                "inputSchema": {
                                                    "type": "object",
                                                    "properties": {}
                                                }
                                            }
                                        ]
                                    }
                                })
                            },
                            "initialize" => {
                                json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "protocolVersion": "2024-11-05",
                                        "capabilities": {
                                            "tools": {}
                                        },
                                        "serverInfo": {
                                            "name": "voice-to-text-mcp",
                                            "version": "0.1.0"
                                        }
                                    }
                                })
                            },
                            _ => {
                                json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "error": {"code": -32601, "message": format!("Method not found: {}", method)}
                                })
                            }
                        };
                        
                        let response_str = serde_json::to_string(&response).unwrap();
                        stdout.write_all(response_str.as_bytes()).await?;
                        stdout.write_all(b"\n").await?;
                        stdout.flush().await?;
                    }
                }
            },
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}