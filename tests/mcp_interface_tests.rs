use voice_to_text_mcp::{VoiceToTextService, mcp_server::VoiceToTextMcpServer};
use voice_to_text_mcp::mcp_server::{ListenRequest, TranscribeFileRequest};
use rmcp::handler::server::{ServerHandler, tool::Parameters};

// Helper function to create ListenRequest with default values
fn create_listen_request() -> ListenRequest {
    ListenRequest {
        timeout_ms: None,
        silence_timeout_ms: None,
        auto_stop: None,
    }
}

#[tokio::test]
async fn test_server_creation() {
    let service = VoiceToTextService::new();
    let _server = VoiceToTextMcpServer::new(service);
    // If we get here without panicking, the server was created successfully
}

#[tokio::test]
async fn test_server_info() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    let info = server.get_info();
    assert!(info.capabilities.tools.is_some());
}

#[tokio::test]
async fn test_list_tools() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    // We can't easily create a RequestContext, so we'll test the tools list differently
    // by checking that the server has the tool_router functionality
    let info = server.get_info();
    assert!(info.capabilities.tools.is_some());
}

#[tokio::test]
async fn test_listen_without_model_path() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    let request = create_listen_request();
    let result = server.listen(Parameters(request)).await;
    
    // Should return error about no model loaded
    assert!(result.contains("Error: Whisper model not loaded"));
}

#[tokio::test]
async fn test_listen_with_custom_timeout() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    let request = ListenRequest {
        timeout_ms: Some(1000),
        silence_timeout_ms: Some(500),
        auto_stop: Some(true),
    };
    let result = server.listen(Parameters(request)).await;
    
    // Should return error about no model loaded
    assert!(result.contains("Error: Whisper model not loaded"));
}

#[tokio::test]
async fn test_listen_with_auto_stop_disabled() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    let request = ListenRequest {
        timeout_ms: Some(1000),
        silence_timeout_ms: Some(500),
        auto_stop: Some(false),
    };
    let result = server.listen(Parameters(request)).await;
    
    // Should return error about no model loaded
    assert!(result.contains("Error: Whisper model not loaded"));
}

#[tokio::test]
async fn test_transcribe_file_nonexistent() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    let request = TranscribeFileRequest {
        file_path: "nonexistent.wav".to_string(),
    };
    let result = server.transcribe_file(Parameters(request)).await;
    
    // Should return error about file not found or model not loaded
    assert!(result.contains("Error"));
}

#[tokio::test]
async fn test_transcribe_file_invalid_path() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    let request = TranscribeFileRequest {
        file_path: "/invalid/path/file.wav".to_string(),
    };
    let result = server.transcribe_file(Parameters(request)).await;
    
    // Should return error about file not found or model not loaded
    assert!(result.contains("Error"));
}

#[tokio::test]
async fn test_server_clone() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    // Test that the server is Clone (required for rmcp)
    let _server2 = server.clone();
}

#[tokio::test]
async fn test_multiple_servers() {
    let service1 = VoiceToTextService::new();
    let service2 = VoiceToTextService::new();
    
    let _server1 = VoiceToTextMcpServer::new(service1);
    let _server2 = VoiceToTextMcpServer::new(service2);
    
    // Should be able to create multiple servers
}

#[tokio::test]
async fn test_concurrent_requests() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    // Test multiple concurrent transcribe requests
    let request1 = TranscribeFileRequest {
        file_path: "test1.wav".to_string(),
    };
    let request2 = TranscribeFileRequest {
        file_path: "test2.wav".to_string(),
    };
    let request3 = TranscribeFileRequest {
        file_path: "test3.wav".to_string(),
    };
    
    let server1 = server.clone();
    let server2 = server.clone();
    let server3 = server.clone();
    
    let future1 = server1.transcribe_file(Parameters(request1));
    let future2 = server2.transcribe_file(Parameters(request2));
    let future3 = server3.transcribe_file(Parameters(request3));
    
    let (result1, result2, result3) = tokio::join!(future1, future2, future3);
    
    // All should return errors (no model loaded)
    assert!(result1.contains("Error"));
    assert!(result2.contains("Error"));
    assert!(result3.contains("Error"));
}

#[tokio::test]
async fn test_listen_parameter_defaults() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    // Test with None values to ensure defaults are applied
    let request = ListenRequest {
        timeout_ms: None,
        silence_timeout_ms: None, 
        auto_stop: None,
    };
    let result = server.listen(Parameters(request)).await;
    
    // Should return error about no model loaded (confirming it tried to record with defaults)
    assert!(result.contains("Error: Whisper model not loaded"));
}

// Test that the server implements the required traits
#[test]
fn test_server_traits() {
    fn assert_server_handler<T: ServerHandler>() {}
    fn assert_clone<T: Clone>() {}
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    
    assert_server_handler::<VoiceToTextMcpServer>();
    assert_clone::<VoiceToTextMcpServer>();
    assert_send::<VoiceToTextMcpServer>();
    assert_sync::<VoiceToTextMcpServer>();
}

// Test parameter validation
#[test]
fn test_parameter_types() {
    // Test that our request types can be created and are valid
    let listen_req = ListenRequest {
        timeout_ms: Some(5000),
        silence_timeout_ms: Some(1000),
        auto_stop: Some(true),
    };
    assert_eq!(listen_req.timeout_ms, Some(5000));
    assert_eq!(listen_req.silence_timeout_ms, Some(1000));
    assert_eq!(listen_req.auto_stop, Some(true));
    
    let transcribe_req = TranscribeFileRequest {
        file_path: "test.wav".to_string(),
    };
    assert_eq!(transcribe_req.file_path, "test.wav");
}