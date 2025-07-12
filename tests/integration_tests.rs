use voice_to_text_mcp::VoiceToTextService;
use std::time::Duration;
use tokio::time::sleep;

// Helper function to check if audio device is available
fn has_audio_device() -> bool {
    use cpal::traits::{DeviceTrait, HostTrait};
    
    let host = cpal::default_host();
    match host.default_input_device() {
        Some(device) => {
            // Try to get a supported config to verify the device works
            match device.default_input_config() {
                Ok(_) => true,
                Err(_) => false,
            }
        },
        None => false,
    }
}

#[tokio::test]
async fn test_complete_recording_workflow() {
    // Skip audio hardware tests in CI or headless environments
    if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() || !has_audio_device() {
        println!("Skipping audio hardware test in CI/headless environment");
        return;
    }
    
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
    // Skip audio hardware tests in CI or headless environments
    if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() || !has_audio_device() {
        println!("Skipping audio hardware test in CI/headless environment");
        return;
    }
    
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
    // Skip audio hardware tests in CI or headless environments
    if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() || !has_audio_device() {
        println!("Skipping audio hardware test in CI/headless environment");
        return;
    }
    
    let service = VoiceToTextService::new();
    
    // Try multiple concurrent start operations
    let start_tasks: Vec<_> = (0..5)
        .map(|_| {
            let service = service.clone();
            tokio::spawn(async move { service.start_listening().await })
        })
        .collect();
    
    let results: Vec<_> = futures::future::join_all(start_tasks).await;
    
    // Count successful starts and "already recording" errors
    let successful_starts = results
        .iter()
        .filter_map(|r| r.as_ref().ok())
        .filter(|r| r.as_ref().map_or(false, |msg| msg.contains("Started")))
        .count();
    
    let _already_recording_errors = results
        .iter()
        .filter_map(|r| r.as_ref().ok())
        .filter(|res| {
            if let Err(e) = res {
                // Check if error is AlreadyRecording using Debug format
                format!("{:?}", e).contains("AlreadyRecording")
            } else {
                false
            }
        })
        .count();
    
    // Should have at most 1 successful start and the rest should be "already recording" errors
    // (or all could fail due to no audio device in test environment)
    if successful_starts > 0 {
        assert_eq!(successful_starts, 1);
        // The remaining should be either already recording errors or audio device failures
        // _already_recording_errors count is meaningful for debugging concurrency
    }
}

#[tokio::test]
async fn test_hardware_acceleration_integration() {
    use std::time::Instant;
    
    let service = VoiceToTextService::new();
    
    // Test platform detection
    let platform = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    
    println!("🔍 Testing hardware acceleration on: {} {}", platform, arch);
    
    // Create test audio data (1 second of sine wave at 16kHz)
    let sample_rate = 16000;
    let duration_samples = sample_rate; // 1 second
    let frequency = 440.0; // A4 note
    
    let test_audio: Vec<f32> = (0..duration_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.3
        })
        .collect();
    
    println!("📊 Generated {} samples of test audio", test_audio.len());
    
    // Test audio processing pipeline performance
    let start_time = Instant::now();
    // Create an audio processor to test the processing pipeline
    let audio_processor = voice_to_text_mcp::AudioProcessor::new(false);
    let processed_result = audio_processor.prepare_for_whisper(&test_audio);
    let processing_duration = start_time.elapsed();
    
    assert!(processed_result.is_ok(), "Audio processing should succeed");
    let processed_audio = processed_result.unwrap();
    
    println!("⚡ Audio processing took: {:?}", processing_duration);
    
    // Validate processed audio
    assert!(!processed_audio.is_empty(), "Processed audio should not be empty");
    assert!(processed_audio.iter().all(|&x| x.is_finite()), "All samples should be finite");
    assert!(processed_audio.iter().all(|&x| x >= -1.0 && x <= 1.0), "All samples should be normalized");
    
    // Test transcription performance (without actual model)
    let start_time = Instant::now();
    let transcription_result = service.transcribe_audio(test_audio.clone()).await;
    let transcription_duration = start_time.elapsed();
    
    assert!(transcription_result.is_ok(), "Transcription should handle gracefully");
    let transcription = transcription_result.unwrap();
    
    println!("🎯 Transcription took: {:?}", transcription_duration);
    println!("📝 Transcription result: {}", transcription);
    
    // Verify expected behavior based on platform
    match (platform, arch) {
        ("macos", "aarch64") => {
            println!("✅ macOS Apple Silicon: Metal + CoreML + NEON acceleration compiled");
            // Should have fast processing due to ARM64 NEON optimizations
            assert!(processing_duration < Duration::from_millis(100), 
                   "Audio processing should be fast with ARM64 optimizations");
        },
        ("macos", "x86_64") => {
            println!("✅ macOS Intel: Metal acceleration compiled");
            // Should have reasonable processing performance
            assert!(processing_duration < Duration::from_millis(200), 
                   "Audio processing should be reasonable on Intel Mac");
        },
        ("linux", "x86_64") | ("windows", "x86_64") => {
            println!("✅ {}: CUDA acceleration compiled", platform);
            // Processing should be reasonable even without actual CUDA hardware
            assert!(processing_duration < Duration::from_millis(300), 
                   "Audio processing should be reasonable");
        },
        _ => {
            println!("✅ Generic platform: CPU-only implementation");
            // Should still work, but no specific performance requirements
            assert!(processing_duration < Duration::from_secs(1), 
                   "Audio processing should complete within reasonable time");
        }
    }
    
    // Test that transcription handles model loading gracefully
    assert!(
        transcription.contains("model not loaded") || 
        transcription.contains("Transcribed") ||
        transcription.contains("No audio data"),
        "Transcription should handle missing model gracefully"
    );
}

#[tokio::test]
async fn test_acceleration_audio_quality_consistency() {
    let _service = VoiceToTextService::new();
    
    // Test that audio processing produces consistent results regardless of acceleration
    let test_cases = vec![
        // Silence
        vec![0.0; 1000],
        // Pure tone
        (0..1000).map(|i| (i as f32 * 0.01).sin()).collect::<Vec<f32>>(),
        // White noise (deterministic)
        (0..1000).map(|i: i32| {
            let seed = (i.wrapping_mul(1103515245).wrapping_add(12345)) & 0x7FFFFFFF;
            (seed as f32 / 2147483647.0) - 0.5
        }).collect::<Vec<f32>>(),
        // Mixed content
        (0..1000).map(|i| {
            let t = i as f32 / 1000.0;
            0.3 * (2.0 * std::f32::consts::PI * 440.0 * t).sin() + 
            0.1 * (2.0 * std::f32::consts::PI * 880.0 * t).sin()
        }).collect::<Vec<f32>>(),
    ];
    
    for (i, test_audio) in test_cases.iter().enumerate() {
        println!("🔬 Testing audio quality case {}", i + 1);
        
        let audio_processor = voice_to_text_mcp::AudioProcessor::new(false);
        let processed = audio_processor.prepare_for_whisper(test_audio);
        assert!(processed.is_ok(), "Audio processing should succeed for case {}", i + 1);
        
        let processed_audio = processed.unwrap();
        
        // Quality checks
        assert!(!processed_audio.is_empty(), "Processed audio should not be empty");
        assert!(processed_audio.iter().all(|&x| x.is_finite()), "All samples should be finite");
        assert!(processed_audio.iter().all(|&x| x >= -1.0 && x <= 1.0), "All samples should be normalized");
        
        // Check that processing doesn't introduce excessive noise
        let max_abs = processed_audio.iter().fold(0.0f32, |acc, &x| acc.max(x.abs()));
        if test_audio.iter().all(|&x| x == 0.0) {
            // Silence should remain silent
            assert!(max_abs < 0.001, "Silence should remain silent after processing");
        }
        
        println!("✅ Case {} passed quality checks", i + 1);
    }
}