//! Credential pool for multi-source plugin.
//! Contains all API keys and credentials extracted from APK reference folder.
//! Implements credential rotation for high availability.

// YouTube/Innertube API keys extracted from APK reference
// These are used across multiple music apps (InnerTune, RiMusic, Kreate, OuterTune, Musify, etc.)
pub const YOUTUBE_API_KEYS: &[&str] = &[
    // Primary keys from various apps
    "AIzaSyC9XL3ZjWddXya6X74dJoCTL-WEYFDNX30",  // WEB_REMIX (InnerTune, Kreate)
    "AIzaSyC9XL3ZjWddXya6X74dJoCTL-WEYFDNX3",   // WEB (InnerTune, OuterTune, OpenTune)
    "AIzaSyDyT5W0Jh49F30Pqqtyfdf7pDLFKLJoAnw",  // PoToken (RiMusic, Kreate, OuterTune)
    
    // ANDROID_MUSIC keys
    "AIzaSyAOghZGza2MQSZkY_zfZ370N-PUdXEo8AI",  // ANDROID_MUSIC (Musify, InnerTune, Kreate)
    
    // ANDROID keys
    "AIzaSyA8eiZmM1FaDVjRy-df2KTyQ_vz_yYM39w",  // ANDROID (InnerTune, Kreate)
    
    // IOS keys
    "AIzaSyB-63vPrdThhKuerbB2N_l7Kwwcxj6yUAc",  // IOS (Musify, Kreate)
    
    // TVHTML5 keys
    "AIzaSyDCU8hByM-4DrUqRUYnGn-3llEO78bcxq8",  // TVHTML5 (InnerTune, Kreate)
    
    // Additional keys from Musify
    "AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8",  // Additional (Musify)
];

// JioSaavn credentials extracted from Echo-Music APK reference
pub const JIOSAAVN_DES_KEY: &[u8; 8] = b"38346591";
pub const JIOSAAVN_REMOTE_CONFIG_URL: &str = "https://echomusic.fun/saavn.json";
pub const JIOSAAVN_BASE_URL: &str = "www.jiosaavn.com";
pub const JIOSAAVN_API_VERSION: &str = "4";
pub const JIOSAAVN_CONTEXT_WEB: &str = "web6dot0";
pub const JIOSAAVN_CONTEXT_WAP: &str = "wap6dot0";

// Default JioSaavn servers (fallback if remote config fails)
pub const JIOSAAVN_DEFAULT_SERVERS: &[&str] = &[
    "saavn.echomusic.fun",
    "saavn1.echomusic.fun",
    "saavn2.echomusic.fun",
];

// Spotify credentials extracted from BlackHole APK reference
// Note: Spotify supports both cookie-based authentication (sp_dc) and OAuth client credentials
pub const SPOTIFY_TOKEN_URL: &str = "https://open.spotify.com/api/token";
pub const SPOTIFY_SERVER_TIME_URL: &str = "https://open.spotify.com/api/server-time";
pub const SPOTIFY_NUANCE_GIST_URL: &str = "https://api.github.com/gists/22ed9c6ba463899e933427f7de1f0eef";
pub const SPOTIFY_LOGIN_URL: &str = "https://accounts.spotify.com/login?continue=https%3A%2F%2Fopen.spotify.com%2F";
pub const SPOTIFY_API_BASE_URL: &str = "https://api.spotify.com/v1";
pub const SPOTIFY_ACCOUNTS_API_URL: &str = "https://accounts.spotify.com/api";

// Spotify OAuth client credentials (from BlackHole APK)
pub const SPOTIFY_CLIENT_IDS: &[&str] = &[
    "08de4eaf71904d1b95254fab3015d711",  // Primary (BlackHole)
];

pub const SPOTIFY_CLIENT_SECRETS: &[&str] = &[
    "622b4fbad33947c59b95a6ae607de11d",  // Primary (BlackHole)
];

pub const SPOTIFY_REDIRECT_URL: &str = "blackhole://spotify/auth";

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Credential rotator for API keys.
/// Automatically rotates through available keys on failure.
pub struct CredentialRotator {
    keys: Vec<String>,
    current_index: Arc<AtomicUsize>,
}

impl CredentialRotator {
    /// Create a new rotator from a slice of API keys.
    pub fn new(keys: &[&str]) -> Self {
        Self {
            keys: keys.iter().map(|s| s.to_string()).collect(),
            current_index: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Get the current API key.
    pub fn current(&self) -> &str {
        let index = self.current_index.load(Ordering::Relaxed);
        &self.keys[index % self.keys.len()]
    }

    /// Rotate to the next API key (call on failure).
    pub fn rotate(&self) -> &str {
        let index = self.current_index.fetch_add(1, Ordering::Relaxed);
        &self.keys[index % self.keys.len()]
    }

    /// Reset to the first API key.
    pub fn reset(&self) {
        self.current_index.store(0, Ordering::Relaxed);
    }

    /// Get the total number of keys.
    pub fn count(&self) -> usize {
        self.keys.len()
    }
}

/// Server rotator for JioSaavn servers.
/// Automatically rotates through available servers on failure.
pub struct ServerRotator {
    servers: Vec<String>,
    current_index: Arc<AtomicUsize>,
}

impl ServerRotator {
    /// Create a new rotator from a slice of server URLs.
    pub fn new(servers: &[&str]) -> Self {
        Self {
            servers: servers.iter().map(|s| s.to_string()).collect(),
            current_index: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Get the current server URL.
    pub fn current(&self) -> &str {
        let index = self.current_index.load(Ordering::Relaxed);
        &self.servers[index % self.servers.len()]
    }

    /// Rotate to the next server (call on failure).
    pub fn rotate(&self) -> &str {
        let index = self.current_index.fetch_add(1, Ordering::Relaxed);
        &self.servers[index % self.servers.len()]
    }

    /// Reset to the first server.
    pub fn reset(&self) {
        self.current_index.store(0, Ordering::Relaxed);
    }

    /// Get the total number of servers.
    pub fn count(&self) -> usize {
        self.servers.len()
    }
}

/// Global YouTube API key rotator.
/// Shared between YouTube Music and YouTube Video clients.
pub fn youtube_rotator() -> &'static CredentialRotator {
    static ROTATOR: CredentialRotator = CredentialRotator::new(YOUTUBE_API_KEYS);
    &ROTATOR
}

/// Global JioSaavn server rotator.
/// Used for automatic server fallback.
pub fn jiosaavn_server_rotator() -> &'static ServerRotator {
    static ROTATOR: ServerRotator = ServerRotator::new(JIOSAAVN_DEFAULT_SERVERS);
    &ROTATOR
}

/// Global Spotify client ID rotator.
/// Used for automatic credential rotation.
pub fn spotify_client_id_rotator() -> &'static CredentialRotator {
    static ROTATOR: CredentialRotator = CredentialRotator::new(SPOTIFY_CLIENT_IDS);
    &ROTATOR
}

/// Global Spotify client secret rotator.
/// Used for automatic credential rotation.
pub fn spotify_client_secret_rotator() -> &'static CredentialRotator {
    static ROTATOR: CredentialRotator = CredentialRotator::new(SPOTIFY_CLIENT_SECRETS);
    &ROTATOR
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_rotator() {
        let keys = vec!["key1", "key2", "key3"];
        let rotator = CredentialRotator::new(&keys);
        
        assert_eq!(rotator.current(), "key1");
        assert_eq!(rotator.rotate(), "key2");
        assert_eq!(rotator.rotate(), "key3");
        assert_eq!(rotator.rotate(), "key1"); // wraps around
        
        rotator.reset();
        assert_eq!(rotator.current(), "key1");
    }

    #[test]
    fn test_youtube_key_count() {
        assert!(YOUTUBE_API_KEYS.len() > 1, "Should have multiple YouTube API keys");
    }
}
