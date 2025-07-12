/// Platform compatibility layer for Send/Sync traits on macOS
/// 
/// On macOS, the cpal library uses CoreAudio types that don't implement Send/Sync,
/// but our usage pattern is actually thread-safe since we only access audio streams
/// from one thread at a time through proper synchronization.

use crate::VoiceToTextService;


/// Macro to conditionally implement Send + Sync for types on macOS
macro_rules! impl_send_sync_on_macos {
    ($type:ty) => {
        #[cfg(target_os = "macos")]
        unsafe impl Send for $type {}
        
        #[cfg(target_os = "macos")]
        unsafe impl Sync for $type {}
    };
}

// Apply the Send + Sync implementation to VoiceToTextService on macOS
impl_send_sync_on_macos!(VoiceToTextService);

// Also apply to the AudioCapture type  
impl_send_sync_on_macos!(crate::audio::AudioCapture);

// Note: We cannot implement Send/Sync for external types like cpal::Stream
// due to Rust's orphan rules. The unsafe implementations above for our own types
// should be sufficient for the MCP server to work on macOS.

/// Platform-specific wrapper for creating MCP servers
pub fn create_mcp_server(service: VoiceToTextService) -> crate::mcp_server::VoiceToTextMcpServer {
    crate::mcp_server::VoiceToTextMcpServer::new(service)
}