use voice_to_text_mcp::VoiceToTextService;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_complete_recording_workflow() {
    let service = VoiceToTextService::new();
    
    // Initially not recording
    assert!(!service.is_recording());
    assert_eq!(service.get_audio_sample_count(), 0);
    
    // Try to start recording (may fail in CI environment without audio)
    let start_result = service.start_listening().await;
    
    if start_result.is_ok() {
        // If we successfully started, should be recording
        assert!(service.is_recording());
        
        // Wait a brief moment to potentially capture some samples
        sleep(Duration::from_millis(100)).await;
        
        // Stop recording
        let stop_result = service.stop_listening().await;
        assert!(stop_result.is_ok());
        
        // Should no longer be recording
        assert!(!service.is_recording());
        
        let transcription = stop_result.unwrap();
        assert!(transcription.contains("Transcribed") || transcription == "No audio data recorded");
    }
}

#[tokio::test]
async fn test_multiple_start_stop_cycles() {
    let service = VoiceToTextService::new();
    
    for _i in 0..3 {
        // Each cycle should work independently
        let start_result = service.start_listening().await;
        
        if start_result.is_ok() {
            assert!(service.is_recording());
            
            let stop_result = service.stop_listening().await;
            assert!(stop_result.is_ok());
            assert!(!service.is_recording());
        }
        
        // Small delay between cycles
        sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test]
async fn test_concurrent_operations() {
    let service = VoiceToTextService::new();
    
    // Try multiple concurrent start operations
    let start_tasks: Vec<_> = (0..5)
        .map(|_| {
            let service = service.clone();
            tokio::spawn(async move { service.start_listening().await })
        })
        .collect();
    
    let results: Vec<_> = futures::future::join_all(start_tasks).await;
    
    // At most one should succeed, others should get "Already recording"
    let successful_starts = results
        .iter()
        .filter_map(|r| r.as_ref().ok())
        .filter(|r| r.as_ref().map_or(false, |msg| msg.contains("Started")))
        .count();
    
    let already_recording_responses = results
        .iter()
        .filter_map(|r| r.as_ref().ok())
        .filter(|r| r.as_ref().map_or(false, |msg| msg.contains("Already recording")))
        .count();
    
    // Should have at most 1 successful start and the rest should be "already recording"
    // (or all could fail due to no audio device in test environment)
    if successful_starts > 0 {
        assert_eq!(successful_starts, 1);
        assert_eq!(already_recording_responses, 4);
    }
}