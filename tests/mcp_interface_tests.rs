use voice_to_text_mcp::{VoiceToTextService, mcp_server::VoiceToTextMcpServer, CommandConfig};
use rmcp::handler::server::tool::Parameters;
use voice_to_text_mcp::mcp_server::ListenRequest;

// Helper function to create ListenRequest with default values
fn create_listen_request(command: Option<String>) -> ListenRequest {
    ListenRequest {
        command,
        enable_voice_commands: None,
        timeout_ms: None,
        silence_timeout_ms: None,
        auto_stop: None,
    }
}

#[tokio::test]
async fn test_listen_command_start() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    let request = create_listen_request(Some("start".to_string()));
    let result = server.listen(Parameters(request)).await;
    
    println!("Start command result: '{}'", result);
    
    // With new blocking behavior, should return transcription or indicate audio device unavailable
    assert!(result.contains("Transcribed") || result.contains("Error") || result.contains("Failed to") || result.contains("No audio data"));
}

#[tokio::test]
async fn test_listen_command_stop() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    let request = create_listen_request(Some("stop".to_string()));
    let result = server.listen(Parameters(request)).await;
    
    // Should indicate not recording or provide transcription
    assert!(result.contains("Not currently recording") || result.contains("Transcribed") || result.contains("No audio data"));
}

#[tokio::test]
async fn test_listen_command_status() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    let request = create_listen_request(Some("status".to_string()));
    let result = server.listen(Parameters(request)).await;
    
    // Should show recording status, sample count, and voice commands status
    assert!(result.contains("Recording:"));
    assert!(result.contains("Voice Commands:"));
    assert!(result.contains("Samples:"));
    assert!(result.contains("Voice Commands:"));
}

#[tokio::test]
async fn test_listen_command_toggle_empty() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    // Empty command should toggle
    let request = create_listen_request(None);
    let result = server.listen(Parameters(request)).await;
    
    println!("Toggle empty result: '{}'", result);
    
    // Toggle starts with auto_stop=false by default for backwards compatibility or returns transcription
    assert!(result.contains("Started listening") || result.contains("Transcribed") || result.contains("Error") || result.contains("No audio data"));
}

#[tokio::test]
async fn test_listen_command_toggle_empty_string() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    // Empty string should toggle
    let request = create_listen_request(Some("".to_string()));
    let result = server.listen(Parameters(request)).await;
    
    println!("Toggle empty string result: '{}'", result);
    
    // Toggle starts with auto_stop=false by default for backwards compatibility or returns transcription  
    assert!(result.contains("Started listening") || result.contains("Transcribed") || result.contains("Error") || result.contains("No audio data"));
}

#[tokio::test]
async fn test_listen_command_invalid() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    let request = create_listen_request(Some("invalid".to_string()));
    let result = server.listen(Parameters(request)).await;
    
    // Should indicate unknown command
    assert!(result.contains("Unknown command"));
    assert!(result.contains("invalid"));
}

#[tokio::test]
async fn test_listen_command_case_insensitive() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    // Test various cases
    let test_cases = ["START", "Start", "StArT", "STATUS", "Status", "STOP", "Stop"];
    
    for case in test_cases {
        let request = create_listen_request(Some(case.to_string()));
        let result = server.listen(Parameters(request)).await;
        
        // Should not return unknown command error
        assert!(!result.contains("Unknown command"), "Case '{}' should be recognized", case);
    }
}

#[tokio::test]
async fn test_listen_command_whitespace_handling() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    // Test commands with whitespace
    let test_cases = [" start ", "\tstart\t", "\nstart\n", "  status  "];
    
    for case in test_cases {
        let request = create_listen_request(Some(case.to_string()));
        let result = server.listen(Parameters(request)).await;
        
        // Should not return unknown command error
        assert!(!result.contains("Unknown command"), "Command '{}' should be recognized", case);
    }
}

#[tokio::test]
async fn test_listen_workflow_start_stop() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    // Test workflow: start with auto_stop disabled -> status -> stop
    let start_request = ListenRequest {
        command: Some("start".to_string()),
        enable_voice_commands: None,
        timeout_ms: Some(1000), // Short timeout for testing
        silence_timeout_ms: Some(500),
        auto_stop: Some(false), // Disable auto-stop for manual testing
    };
    
    // Start recording without auto-stop should return immediately
    let start_result = server.listen(Parameters(start_request)).await;
    
    if start_result.contains("Started listening") {
        // Check status - recording should be active
        let status_request = create_listen_request(Some("status".to_string()));
        let status_result = server.listen(Parameters(status_request)).await;
        assert!(status_result.contains("Recording: true"));
        
        // Stop recording manually
        let stop_request = create_listen_request(Some("stop".to_string()));
        let stop_result = server.listen(Parameters(stop_request)).await;
        assert!(stop_result.contains("Transcribed") || stop_result.contains("No audio data"));
        
        // Check status again - should not be recording
        let status_request2 = create_listen_request(Some("status".to_string()));
        let status_result2 = server.listen(Parameters(status_request2)).await;
        assert!(status_result2.contains("Recording: false"));
    } else {
        // If start failed due to no audio device, verify it returns transcription or error
        assert!(start_result.contains("Transcribed") || start_result.contains("Error") || start_result.contains("No audio data"));
    }
}

#[tokio::test]
async fn test_configurable_commands_custom_config() {
    let service = VoiceToTextService::new();
    
    // Create custom command config using from_cli_args
    let config = CommandConfig::from_cli_args(
        Some(vec!["begin".to_string(), "go".to_string(), "record".to_string()]),
        Some(vec!["finish".to_string(), "done".to_string(), "halt".to_string()]),
        Some(vec!["info".to_string(), "state".to_string()]),
        None,
    );
    
    let server = VoiceToTextMcpServer::new_with_config(service, config);
    
    // Test custom start commands
    let request = create_listen_request(Some("begin".to_string()));
    let result = server.listen(Parameters(request)).await;
    assert!(result.contains("Transcribed") || result.contains("Error") || result.contains("No audio data"));
    
    // Test custom stop commands
    let request = create_listen_request(Some("finish".to_string()));
    let result = server.listen(Parameters(request)).await;
    assert!(result.contains("Not currently recording") || result.contains("Transcribed") || result.contains("No audio data"));
    
    // Test custom status commands
    let request = create_listen_request(Some("info".to_string()));
    let result = server.listen(Parameters(request)).await;
    assert!(result.contains("Recording:"));
    assert!(result.contains("Voice Commands:"));
}

#[tokio::test]
async fn test_configurable_commands_backward_compatibility() {
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service); // Uses default config
    
    // Original commands should still work
    let test_cases = ["start", "stop", "status"];
    
    for case in test_cases {
        let request = create_listen_request(Some(case.to_string()));
        let result = server.listen(Parameters(request)).await;
        
        // Should not return unknown command error
        assert!(!result.contains("Unknown command"), "Command '{}' should be recognized", case);
    }
}

#[tokio::test]
async fn test_configurable_commands_multilingual() {
    let service = VoiceToTextService::new();
    
    // Create Spanish/French command config using from_cli_args
    let config = CommandConfig::from_cli_args(
        Some(vec!["iniciar".to_string(), "commencer".to_string(), "start".to_string()]),
        Some(vec!["parar".to_string(), "arrêter".to_string(), "stop".to_string()]),
        Some(vec!["estado".to_string(), "statut".to_string(), "status".to_string()]),
        None,
    );
    
    let server = VoiceToTextMcpServer::new_with_config(service, config);
    
    // Test Spanish commands
    let request = create_listen_request(Some("iniciar".to_string()));
    let result = server.listen(Parameters(request)).await;
    assert!(result.contains("Transcribed") || result.contains("Error") || result.contains("No audio data"));
    
    // Test French commands
    let request = create_listen_request(Some("commencer".to_string()));
    let result = server.listen(Parameters(request)).await;
    println!("French command result: '{}'", result);
    assert!(result.contains("Transcribed") || result.contains("Error") || result.contains("Already recording") || result.contains("No audio data"));
    
    // Test status in Spanish
    let request = create_listen_request(Some("estado".to_string()));
    let result = server.listen(Parameters(request)).await;
    assert!(result.contains("Recording:"));
    assert!(result.contains("Voice Commands:"));
}

#[tokio::test]
async fn test_configurable_commands_error_message() {
    let service = VoiceToTextService::new();
    
    // Create custom config with specific commands using from_cli_args
    let config = CommandConfig::from_cli_args(
        Some(vec!["go".to_string()]),
        Some(vec!["halt".to_string()]),
        Some(vec!["check".to_string()]),
        Some(vec!["switch".to_string()]),
    );
    
    let server = VoiceToTextMcpServer::new_with_config(service, config);
    
    // Test invalid command
    let request = create_listen_request(Some("invalid".to_string()));
    let result = server.listen(Parameters(request)).await;
    
    // Should show available commands
    assert!(result.contains("Unknown command"));
    assert!(result.contains("Available commands"));
    assert!(result.contains("go"));
    assert!(result.contains("halt"));
    assert!(result.contains("check"));
    assert!(result.contains("switch"));
}