//! Credential pools for reliable download with automatic rotation
//!
//! This module manages credential pools extracted from APK reference implementations
//! to provide high availability and automatic fallback on failure.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// YouTube API key pool from various music apps
pub struct YouTubeCredentialPool {
    keys: Vec<String>,
    current_index: Arc<AtomicUsize>,
}

impl YouTubeCredentialPool {
    pub fn new() -> Self {
        let keys = vec![
            // WEB_REMIX - InnerTune, Kreate
            "AIzaSyC9XL3ZjWddXya6X74dJoCTL-WEYFDNX30".to_string(),
            // WEB - InnerTune, OuterTune, OpenTune
            "AIzaSyC9XL3ZjWddXya6X74dJoCTL-WEYFDNX3".to_string(),
            // PoToken - RiMusic, Kreate, OuterTune
            "AIzaSyDyT5W0Jh49F30Pqqtyfdf7pDLFKLJoAnw".to_string(),
            // ANDROID_MUSIC - Musify, InnerTune, Kreate
            "AIzaSyAOghZGza2MQSZkY_zfZ370N-PUdXEo8AI".to_string(),
            // ANDROID - InnerTune, Kreate
            "AIzaSyA8eiZmM1FaDVjRy-df2KTyQ_vz_yYM39w".to_string(),
            // IOS - Musify, Kreate
            "AIzaSyB-63vPrdThhKuerbB2N_l7Kwwcxj6yUAc".to_string(),
            // TVHTML5 - InnerTune, Kreate
            "AIzaSyDCU8hByM-4DrUqRUYnGn-3llEO78bcxq8".to_string(),
            // Additional - Musify
            "AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8".to_string(),
        ];
        
        Self {
            keys,
            current_index: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn get_current(&self) -> String {
        let index = self.current_index.load(Ordering::Relaxed);
        self.keys[index % self.keys.len()].clone()
    }

    pub fn rotate(&self) -> String {
        let index = self.current_index.fetch_add(1, Ordering::Relaxed);
        self.keys[index % self.keys.len()].clone()
    }

    pub fn reset(&self) {
        self.current_index.store(0, Ordering::Relaxed);
    }
}

impl Default for YouTubeCredentialPool {
    fn default() -> Self {
        Self::new()
    }
}

/// JioSaavn server pool with automatic rotation
pub struct JioSaavnServerPool {
    servers: Vec<String>,
    current_index: Arc<AtomicUsize>,
}

impl JioSaavnServerPool {
    pub fn new() -> Self {
        let servers = vec![
            "saavn.echomusic.fun".to_string(),
            "saavn1.echomusic.fun".to_string(),
            "saavn2.echomusic.fun".to_string(),
            "www.jiosaavn.com".to_string(),
        ];
        
        Self {
            servers,
            current_index: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn get_current(&self) -> String {
        let index = self.current_index.load(Ordering::Relaxed);
        self.servers[index % self.servers.len()].clone()
    }

    pub fn rotate(&self) -> String {
        let index = self.current_index.fetch_add(1, Ordering::Relaxed);
        self.servers[index % self.servers.len()].clone()
    }

    pub fn reset(&self) {
        self.current_index.store(0, Ordering::Relaxed);
    }
}

impl Default for JioSaavnServerPool {
    fn default() -> Self {
        Self::new()
    }
}

/// JioSaavn DES-ECB decryption key (from Echo-Music APK reference)
pub const JIOSAAVN_DES_KEY: &[u8] = b"38346591";

/// JioSaavn API configuration
pub struct JioSaavnConfig {
    pub api_version: String,
    pub context: String,
    pub base_url: String,
}

impl Default for JioSaavnConfig {
    fn default() -> Self {
        Self {
            api_version: "4".to_string(),
            context: "web6dot0".to_string(),
            base_url: "https://www.jiosaavn.com".to_string(),
        }
    }
}

/// Global credential pools
lazy_static::lazy_static! {
    pub static ref YOUTUBE_CREDENTIALS: YouTubeCredentialPool = YouTubeCredentialPool::new();
    pub static ref JIOSAAVN_SERVERS: JioSaavnServerPool = JioSaavnServerPool::new();
    pub static ref JIOSAAVN_CONFIG: JioSaavnConfig = JioSaavnConfig::default();
}
