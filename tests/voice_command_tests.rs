use voice_to_text_mcp::{VoiceToTextService, VoiceCommandConfig, CommandConfig, CommandType, DebugConfig};
use voice_to_text_mcp::mcp_server::ListenRequest;
use std::time::Duration;

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
async fn test_voice_command_detection() {
    let mut voice_config = VoiceCommandConfig::default();
    voice_config.enabled = true;
    
    let service = VoiceToTextService::new_with_configs(
        DebugConfig::default(),
        voice_config
    );
    
    // Test basic command detection
    assert_eq!(service.detect_voice_commands("stop"), Some(CommandType::Stop));
    assert_eq!(service.detect_voice_commands("start"), Some(CommandType::Start));
    assert_eq!(service.detect_voice_commands("status"), Some(CommandType::Status));
    assert_eq!(service.detect_voice_commands("toggle"), Some(CommandType::Toggle));
    
    // Test case insensitive detection
    assert_eq!(service.detect_voice_commands("STOP"), Some(CommandType::Stop));
    assert_eq!(service.detect_voice_commands("Start"), Some(CommandType::Start));
    
    // Test phrase detection
    assert_eq!(service.detect_voice_commands("stop recording"), Some(CommandType::Stop));
    assert_eq!(service.detect_voice_commands("start recording"), Some(CommandType::Start));
    
    // Test no detection for non-commands
    assert_eq!(service.detect_voice_commands("hello world"), None);
    assert_eq!(service.detect_voice_commands(""), None);
}

#[tokio::test]
async fn test_voice_command_configuration() {
    let service = VoiceToTextService::new();
    
    // Voice commands should be enabled by default
    assert!(service.is_voice_commands_enabled());
    
    // Create service with voice commands enabled
    let mut voice_config = VoiceCommandConfig::default();
    voice_config.enabled = true;
    voice_config.chunk_duration_ms = 2000;
    voice_config.detection_sensitivity = 0.8;
    
    let service = VoiceToTextService::new_with_configs(
        DebugConfig::default(),
        voice_config.clone()
    );
    
    assert!(service.is_voice_commands_enabled());
    assert_eq!(service.get_voice_command_config().chunk_duration_ms, 2000);
    assert_eq!(service.get_voice_command_config().detection_sensitivity, 0.8);
}

#[tokio::test]
async fn test_voice_command_state_management() {
    let mut voice_config = VoiceCommandConfig::default();
    voice_config.enabled = true;
    
    let mut service = VoiceToTextService::new_with_configs(
        DebugConfig::default(),
        voice_config
    );
    
    // Initially no voice command detected
    assert!(!service.voice_command_detected());
    
    // Test enabling/disabling voice commands
    service.set_voice_commands_enabled(false);
    assert!(!service.is_voice_commands_enabled());
    
    service.set_voice_commands_enabled(true);
    assert!(service.is_voice_commands_enabled());
    
    // Test reset functionality
    service.reset_voice_command_detection();
    assert!(!service.voice_command_detected());
}

#[tokio::test]
async fn test_voice_command_transcription_filtering() {
    let mut voice_config = VoiceCommandConfig::default();
    voice_config.enabled = true;
    voice_config.include_in_final_transcription = false;
    
    let service = VoiceToTextService::new_with_configs(
        DebugConfig::default(),
        voice_config
    );
    
    // Test removing voice commands from transcription
    let original = "Hello world stop recording thank you";
    let partial_buffer = "stop recording";
    let cleaned = service.remove_voice_commands_from_transcription(original, partial_buffer);
    
    // The word "stop" should be removed as it's a command
    assert!(cleaned.contains("Hello world"));
    assert!(cleaned.contains("thank you"));
    assert!(!cleaned.contains("stop"));
}

#[tokio::test]
async fn test_voice_command_phrase_detection() {
    let mut voice_config = VoiceCommandConfig::default();
    voice_config.enabled = true;
    
    let service = VoiceToTextService::new_with_configs(
        DebugConfig::default(),
        voice_config
    );
    
    // Test various phrase patterns
    let test_cases = vec![
        ("stop recording now", Some(CommandType::Stop)),
        ("please stop record", Some(CommandType::Stop)),
        ("start recording", Some(CommandType::Start)),
        ("begin recording", Some(CommandType::Start)),
        ("end recording", Some(CommandType::Stop)),
        ("finish recording", Some(CommandType::Stop)),
        ("what is the status", Some(CommandType::Status)),
        ("recording status please", Some(CommandType::Status)),
        ("hello everyone", None),
        ("stopping by the store", None), // Should not trigger false positive
    ];
    
    for (phrase, expected) in test_cases {
        let result = service.detect_voice_commands(phrase);
        assert_eq!(result, expected, "Failed for phrase: '{}'", phrase);
    }
}

#[tokio::test] 
async fn test_voice_command_custom_commands() {
    let mut command_config = CommandConfig::default();
    command_config.start_commands = vec!["go".to_string(), "begin".to_string()];
    command_config.stop_commands = vec!["halt".to_string(), "end".to_string()];
    
    let mut voice_config = VoiceCommandConfig::default();
    voice_config.enabled = true;
    voice_config.command_config = command_config;
    
    let service = VoiceToTextService::new_with_configs(
        DebugConfig::default(),
        voice_config
    );
    
    // Test custom commands
    assert_eq!(service.detect_voice_commands("go"), Some(CommandType::Start));
    assert_eq!(service.detect_voice_commands("halt"), Some(CommandType::Stop));
    assert_eq!(service.detect_voice_commands("begin"), Some(CommandType::Start));
    assert_eq!(service.detect_voice_commands("end"), Some(CommandType::Stop));
    
    // Default commands should not work
    assert_eq!(service.detect_voice_commands("start"), None);
    assert_eq!(service.detect_voice_commands("stop"), None);
}

#[tokio::test]
async fn test_voice_command_recording_integration() {
    let mut voice_config = VoiceCommandConfig::default();
    voice_config.enabled = true;
    
    let service = VoiceToTextService::new_with_configs(
        DebugConfig::default(),
        voice_config
    );
    
    // Initially not recording
    assert!(!service.is_recording());
    
    // Start recording should work normally
    let start_result = service.start_listening().await;
    
    // If audio device is available, should start successfully
    if start_result.is_ok() && start_result.unwrap().contains("Started") {
        assert!(service.is_recording());
        
        // Voice command detection should be reset when starting
        assert!(!service.voice_command_detected());
        
        // Stop recording
        let stop_result = service.stop_listening().await;
        assert!(stop_result.is_ok());
        assert!(!service.is_recording());
    }
}

#[tokio::test]
async fn test_voice_command_sensitivity_settings() {
    let mut voice_config = VoiceCommandConfig::default();
    voice_config.enabled = true;
    voice_config.detection_sensitivity = 0.9; // High sensitivity
    voice_config.chunk_duration_ms = 1000;
    
    let service = VoiceToTextService::new_with_configs(
        DebugConfig::default(),
        voice_config
    );
    
    let config = service.get_voice_command_config();
    assert_eq!(config.detection_sensitivity, 0.9);
    assert_eq!(config.chunk_duration_ms, 1000);
    assert!(config.enabled);
}

#[tokio::test]
async fn test_voice_command_edge_cases() {
    let mut voice_config = VoiceCommandConfig::default();
    voice_config.enabled = true;
    
    let service = VoiceToTextService::new_with_configs(
        DebugConfig::default(),
        voice_config
    );
    
    // Test edge cases
    assert_eq!(service.detect_voice_commands("   stop   "), Some(CommandType::Stop)); // Whitespace
    assert_eq!(service.detect_voice_commands("stop."), Some(CommandType::Stop)); // Punctuation
    assert_eq!(service.detect_voice_commands("stopped"), None); // Word with suffix should not match
    assert_eq!(service.detect_voice_commands("unstoppable"), None); // Word with prefix should not match
    assert_eq!(service.detect_voice_commands("stop stop stop"), Some(CommandType::Stop)); // Repeated
}

#[tokio::test]
async fn test_voice_command_performance() {
    let mut voice_config = VoiceCommandConfig::default();
    voice_config.enabled = true;
    
    let service = VoiceToTextService::new_with_configs(
        DebugConfig::default(),
        voice_config
    );
    
    // Test performance with many rapid command detections
    let start_time = std::time::Instant::now();
    
    for _ in 0..1000 {
        let _ = service.detect_voice_commands("stop recording now");
    }
    
    let duration = start_time.elapsed();
    
    // Should complete 1000 detections in less than 100ms
    assert!(duration < Duration::from_millis(100), 
           "Voice command detection too slow: {:?}", duration);
}

#[tokio::test]
async fn test_voice_command_concurrent_access() {
    let mut voice_config = VoiceCommandConfig::default();
    voice_config.enabled = true;
    
    let service = VoiceToTextService::new_with_configs(
        DebugConfig::default(),
        voice_config
    );
    
    // Test concurrent access to voice command state
    let service_clone1 = service.clone();
    let service_clone2 = service.clone();
    
    let handle1 = tokio::spawn(async move {
        for _ in 0..100 {
            let _ = service_clone1.detect_voice_commands("stop");
            tokio::task::yield_now().await;
        }
    });
    
    let handle2 = tokio::spawn(async move {
        for _ in 0..100 {
            let _ = service_clone2.voice_command_detected();
            service_clone2.reset_voice_command_detection();
            tokio::task::yield_now().await;
        }
    });
    
    // Should complete without deadlocks or panics
    let result1 = handle1.await;
    let result2 = handle2.await;
    
    assert!(result1.is_ok());
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_voice_command_mcp_integration() {
    use voice_to_text_mcp::mcp_server::{VoiceToTextMcpServer, ListenRequest};
    use rmcp::handler::server::tool::Parameters;
    
    let service = VoiceToTextService::new();
    let server = VoiceToTextMcpServer::new(service);
    
    // Test enabling voice commands through MCP interface
    let request = ListenRequest { 
        command: Some("status".to_string()), 
        enable_voice_commands: Some(true),
        timeout_ms: None,
        silence_timeout_ms: None,
        auto_stop: None,
    };
    let result = server.listen(Parameters(request)).await;
    
    // Should show voice commands enabled
    assert!(result.contains("Voice Commands: enabled"));
    
    // Test disabling voice commands
    let request = ListenRequest { 
        command: Some("status".to_string()), 
        enable_voice_commands: Some(false),
        timeout_ms: None,
        silence_timeout_ms: None,
        auto_stop: None,
    };
    let result = server.listen(Parameters(request)).await;
    
    // Should show voice commands disabled  
    assert!(result.contains("Voice Commands: disabled"));
    
    // Test that None doesn't change the setting
    let request = create_listen_request(Some("status".to_string()));
    let result = server.listen(Parameters(request)).await;
    
    // Should still show disabled (last setting)
    assert!(result.contains("Voice Commands: disabled"));
}