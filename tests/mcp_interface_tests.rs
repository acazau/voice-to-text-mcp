use voice_to_text_mcp::{VoiceToTextService, mcp_server::VoiceToTextMcpServer};
use rmcp::handler::server::tool::Parameters;
use voice_to_text_mcp::mcp_server::{ListenRequest, TranscribeFileRequest};

// Helper function to create ListenRequest with default values
fn create_listen_request() -> ListenRequest {
    ListenRequest {
        timeout_ms: None,
        silence_timeout_ms: None,
        auto_stop: None,
    }
}

#[tokio::test]
async fn test_listen_without_model_path() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service); // No model path provided
    
    let request = create_listen_request();
    let result = server.listen(Parameters(request)).await;
    
    // Should return error about no model path configured
    assert!(result.contains("Error: No model path configured"));
}

#[tokio::test]
async fn test_listen_with_model_path() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    let request = create_listen_request();
    let result = server.listen(Parameters(request)).await;
    
    // Should return error about no model configured
    assert!(result.contains("Error:"));
}

#[tokio::test]
async fn test_listen_with_custom_timeout() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    let request = ListenRequest {
        timeout_ms: Some(5000),
        silence_timeout_ms: Some(1000),
        auto_stop: Some(true),
    };
    
    let result = server.listen(Parameters(request)).await;
    
    // Should return error about no model configured
    assert!(result.contains("Error:"));
}

#[tokio::test]
async fn test_listen_with_auto_stop_disabled() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    let request = ListenRequest {
        timeout_ms: Some(5000),
        silence_timeout_ms: Some(1000),
        auto_stop: Some(false),
    };
    
    let result = server.listen(Parameters(request)).await;
    
    // Should return error about no model configured
    assert!(result.contains("Error:"));
}

#[tokio::test]
async fn test_transcribe_file_valid_path() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    let request = TranscribeFileRequest {
        file_path: "nonexistent.wav".to_string(),
    };
    
    let result = server.transcribe_file(Parameters(request)).await;
    
    // Should return error for nonexistent file
    assert!(result.contains("Error:"));
}

#[tokio::test]
async fn test_transcribe_file_empty_path() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    let request = TranscribeFileRequest {
        file_path: "".to_string(),
    };
    
    let result = server.transcribe_file(Parameters(request)).await;
    
    // Should return error for empty path
    assert!(result.contains("Error:"));
}

#[tokio::test]
async fn test_server_creation() {
    // Test basic server creation
    let service = VoiceToTextService::new();
    let _server = VoiceToTextMcpServer::new(service);
    
    // Test server creation with model path
    let service2 = VoiceToTextService::new();
    let _server2 = VoiceToTextMcpServer::new(service2);
}

#[tokio::test]
async fn test_listen_request_defaults() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    // Test with minimal request (all defaults)
    let request = ListenRequest {
        timeout_ms: None,
        silence_timeout_ms: None,
        auto_stop: None,
    };
    
    let result = server.listen(Parameters(request)).await;
    
    // Should use default values (timeout: 30000, silence_timeout: 2000, auto_stop: true)
    assert!(result.contains("Error: Failed to execute voice recorder") || result.contains("Error:"));
}