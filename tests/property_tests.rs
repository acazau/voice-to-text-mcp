use voice_to_text_mcp::VoiceToTextService;
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_audio_data_transcription_properties(
        audio_samples in prop::collection::vec(any::<f32>(), 0..1000)
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let service = VoiceToTextService::new();
        
        rt.block_on(async {
            let result = service.transcribe_audio(audio_samples.clone()).await;
            
            // Property: transcription should always succeed
            assert!(result.is_ok());
            
            let transcription = result.unwrap();
            
            if audio_samples.is_empty() {
                // Property: empty audio should return specific message
                assert_eq!(transcription, "No audio data recorded");
            } else {
                // Property: non-empty audio should mention sample count or model status
                assert!(transcription.contains(&audio_samples.len().to_string()) || 
                        transcription.contains("model not loaded"));
                assert!(transcription.contains("Transcribed") || 
                        transcription.contains("model not loaded"));
            }
        });
    }
}

proptest! {
    #[test]
    fn test_transcription_result_properties(
        audio_size in 0..1000usize
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let service = VoiceToTextService::new();
        
        rt.block_on(async {
            let audio_data = vec![0.5f32; audio_size];
            let result = service.transcribe_audio(audio_data).await;
            
            // Property: transcription should always succeed
            assert!(result.is_ok());
            
            let transcription = result.unwrap();
            
            if audio_size == 0 {
                // Property: empty audio should return specific message
                assert_eq!(transcription, "No audio data recorded");
            } else {
                // Property: non-empty audio should contain size information or model status
                assert!(transcription.contains(&audio_size.to_string()) || 
                        transcription.contains("model not loaded"));
                assert!(transcription.contains("Transcribed") || 
                        transcription.contains("model not loaded"));
                assert!(transcription.contains("samples") || 
                        transcription.contains("model not loaded"));
            }
        });
    }
}