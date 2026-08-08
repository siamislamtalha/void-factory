use serde::{Deserialize, Serialize};

/// Music service sources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MusicSource {
    Qobuz,
    Tidal,
    Deezer,
    SoundCloud,
    Amazon,
    UnifiedPlayback,
}

impl MusicSource {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "qobuz" => Some(MusicSource::Qobuz),
            "tidal" => Some(MusicSource::Tidal),
            "deezer" => Some(MusicSource::Deezer),
            "soundcloud" | "sound_cloud" => Some(MusicSource::SoundCloud),
            "amazon" => Some(MusicSource::Amazon),
            "unified" | "unifiedplayback" => Some(MusicSource::UnifiedPlayback),
            _ => None,
        }
    }
}

impl MusicSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Qobuz => "Qobuz",
            Self::Tidal => "Tidal",
            Self::Deezer => "Deezer",
            Self::SoundCloud => "SoundCloud",
            Self::Amazon => "Amazon",
            Self::UnifiedPlayback => "UnifiedPlayback",
        }
    }

    pub fn id_prefix(&self) -> &'static str {
        match self {
            Self::Qobuz => "qobuz:",
            Self::Tidal => "tidal:",
            Self::Deezer => "deezer:",
            Self::SoundCloud => "soundcloud:",
            Self::Amazon => "amazon:",
            Self::UnifiedPlayback => "unified:",
        }
    }

    pub fn max_quality(&self) -> u8 {
        match self {
            Self::Qobuz => 4,  // 24-bit, ≤ 192 kHz
            Self::Tidal => 4,   // 24-bit, ≤ 96 kHz (MQA) + Dolby Atmos
            Self::Deezer => 2,  // 16-bit, 44.1 kHz (CD)
            Self::SoundCloud => 0,  // MP3 only
            Self::Amazon => 4,  // UHD up to 24-bit/192kHz
            Self::UnifiedPlayback => 4,  // Depends on provider
        }
    }
}

/// Quality levels for streaming/download
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Quality {
    Low = 0,        // 128 kbps MP3/AAC
    Normal = 1,     // 320 kbps MP3/AAC
    High = 2,       // 16-bit, 44.1 kHz (CD)
    HiRes = 3,      // 24-bit, ≤ 96 kHz
    UltraHiRes = 4, // 24-bit, ≤ 192 kHz
    DolbyAtmos = 5, // Dolby Atmos (EAC3_JOC) - highest quality
}

impl Quality {
    pub fn from_number(n: u8) -> Self {
        match n {
            0 => Quality::Low,
            1 => Quality::Normal,
            2 => Quality::High,
            3 => Quality::HiRes,
            4 => Quality::UltraHiRes,
            5 => Quality::DolbyAtmos,
            _ => Quality::High,
        }
    }

    pub fn as_number(&self) -> u8 {
        *self as u8
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Quality::Low => "LOW",
            Quality::Normal => "NORMAL",
            Quality::High => "LOSSLESS",
            Quality::HiRes => "HI_RES",
            Quality::UltraHiRes => "ULTRA_HI_RES",
            Quality::DolbyAtmos => "DOLBY_ATMOS",
        }
    }

    pub fn bitrate(&self) -> u32 {
        match self {
            Quality::Low => 128,
            Quality::Normal => 320,
            Quality::High => 1411,
            Quality::HiRes => 4704,
            Quality::UltraHiRes => 9408,
            Quality::DolbyAtmos => 4704, // Similar to Hi-Res
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "LOW" => Quality::Low,
            "NORMAL" => Quality::Normal,
            "LOSSLESS" => Quality::High,
            "HI_RES" | "HIRES" => Quality::HiRes,
            "ULTRA_HI_RES" | "ULTRAHIRES" => Quality::UltraHiRes,
            "DOLBY_ATMOS" | "DOLBYATMOS" | "ATMOS" => Quality::DolbyAtmos,
            _ => Quality::High,
        }
    }
}

/// Unified track metadata from all services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedTrack {
    pub id: String,
    pub title: String,
    pub duration: u32,
    pub track_number: Option<u32>,
    pub volume_number: Option<u32>,
    pub replay_gain: Option<ReplayGain>,
    pub peak: Option<f32>,
    pub available: bool,
    pub audio_quality: Option<String>,
    pub audio_modes: Option<Vec<String>>,
    pub artist: Option<UnifiedArtist>,
    pub artists: Option<Vec<UnifiedArtist>>,
    pub album: Option<UnifiedAlbum>,
    pub source: MusicSource,
    pub qualities_available: Vec<Quality>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayGain {
    pub track_gain: Option<f32>,
    pub album_gain: Option<f32>,
    pub track_peak: Option<f32>,
    pub album_peak: Option<f32>,
}

/// Unified artist metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedArtist {
    pub id: String,
    pub name: String,
    pub picture: Option<String>,
    pub url: Option<String>,
    pub source: MusicSource,
}

/// Unified album metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedAlbum {
    pub id: String,
    pub title: String,
    pub cover: Option<String>,
    pub duration: Option<u32>,
    pub track_count: Option<u32>,
    pub release_date: Option<String>,
    pub artist: Option<UnifiedArtist>,
    pub artists: Option<Vec<UnifiedArtist>>,
    pub url: Option<String>,
    pub source: MusicSource,
}

/// Unified playlist metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedPlaylist {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub picture: Option<String>,
    pub duration: Option<u32>,
    pub track_count: Option<u32>,
    pub last_updated: Option<String>,
    pub creator: Option<Creator>,
    pub source: MusicSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Creator {
    pub id: String,
    pub name: String,
}

/// Stream info with quality details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    pub track_id: String,
    pub quality: Quality,
    pub codec: String,
    pub url: String,
    pub encryption_key: Option<String>,
    pub source: MusicSource,
    pub bitrate: u32,
    pub sample_rate: u32,
    pub bit_depth: u8,
}

// Qobuz-specific types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzConfig {
    pub email_or_userid: Option<String>,
    pub password_or_token: Option<String>,
    pub app_id: Option<String>,
    pub secrets: Vec<String>,
    pub use_auth_token: bool,
}

// Tidal-specific types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TidalConfig {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub user_id: Option<String>,
    pub country_code: Option<String>,
    pub token_expiry: Option<f64>,
}

// Deezer-specific types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeezerConfig {
    pub arl: Option<String>,
}

/// Unified API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub qobuz: QobuzConfig,
    pub tidal: TidalConfig,
    pub deezer: DeezerConfig,
    pub proxy_instances: Vec<ApiInstance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInstance {
    pub url: String,
    pub version: Option<String>,
    pub is_user: bool,
}

/// Search response from unified search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub tracks: SearchSection<UnifiedTrack>,
    pub artists: SearchSection<UnifiedArtist>,
    pub albums: SearchSection<UnifiedAlbum>,
    pub playlists: Option<SearchSection<UnifiedPlaylist>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSection<T> {
    pub items: Vec<T>,
    pub limit: u32,
    pub offset: u32,
    pub total_number_of_items: u32,
}

/// Home section data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeSection {
    pub id: String,
    pub title: String,
    pub items: Vec<MediaItemData>,
    pub page_token: Option<String>,
    pub source: MusicSource,
    pub section_type: HomeSectionType,
}

/// Types of home sections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HomeSectionType {
    Featured,
    Recommendations,
    Charts,
    NewReleases,
    TopTracks,
    Trending,
    Editorial,
    ArtistRadio,
    SimilarArtists,
    Custom(String),
}

/// Search suggestion data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSuggestion {
    pub text: String,
    pub suggestion_type: SuggestionType,
    pub metadata: Option<SuggestionMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuggestionType {
    Track,
    Artist,
    Album,
    Playlist,
    Query,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionMetadata {
    pub id: String,
    pub artist: Option<String>,
    pub cover: Option<String>,
    pub source: MusicSource,
}

/// Advanced search filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedSearchFilter {
    pub quality: Option<Quality>,
    pub duration_min: Option<u32>,
    pub duration_max: Option<u32>,
    pub year_min: Option<u32>,
    pub year_max: Option<u32>,
    pub genres: Vec<String>,
}

/// Download request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadRequest {
    pub track_id: String,
    pub quality: Quality,
    pub format: DownloadFormat,
    pub include_metadata: bool,
    pub include_artwork: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DownloadFormat {
    FLAC,
    ALAC,
    MP3,
    AAC,
    OPUS,
}

/// Download progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub track_id: String,
    pub progress: f32, // 0.0 to 1.0
    pub status: DownloadStatus,
    pub speed: Option<u64>, // bytes per second
    pub eta: Option<u64>, // seconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DownloadStatus {
    Queued,
    Downloading,
    Decrypting,
    Converting,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MediaItemData {
    Track(UnifiedTrack),
    Album(UnifiedAlbum),
    Artist(UnifiedArtist),
    Playlist(UnifiedPlaylist),
}

/// Monochrome-compatible API response types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonochromeResponse<T> {
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonochromeTrack {
    pub id: String,
    pub title: String,
    pub version: Option<String>,
    pub duration: i64,
    pub track_number: Option<i32>,
    pub volume_number: Option<i32>,
    pub explicit: Option<bool>,
    pub audio_quality: Option<String>,
    pub audio_modes: Option<Vec<String>>,
    pub stream_url: Option<String>,
    pub preview_url: Option<String>,
    pub copyright: Option<String>,
    pub url: Option<String>,
    pub artists: Vec<MonochromeArtist>,
    pub album: MonochromeAlbum,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonochromeArtist {
    pub id: String,
    pub name: String,
    pub picture: Option<String>,
    pub url: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonochromeAlbum {
    pub id: String,
    pub title: String,
    pub cover: Option<String>,
    pub cover_big: Option<String>,
    pub duration: Option<i64>,
    pub track_count: Option<i32>,
    pub release_date: Option<String>,
    pub copyright: Option<String>,
    pub url: Option<String>,
    pub artist: Option<MonochromeArtist>,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonochromeStreamInfo {
    pub url: String,
    pub stream_url: Option<String>,
    pub quality: String,
    pub codec: String,
    pub bitrate: Option<i32>,
    pub sample_rate: Option<i32>,
    pub bit_depth: Option<i32>,
    pub encryption_key: Option<String>,
}

/// Unified music source enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MusicSource {
    Qobuz,
    Tidal,
    Deezer,
}

impl MusicSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            MusicSource::Qobuz => "qobuz",
            MusicSource::Tidal => "tidal",
            MusicSource::Deezer => "deezer",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "qobuz" => Some(MusicSource::Qobuz),
            "tidal" => Some(MusicSource::Tidal),
            "deezer" => Some(MusicSource::Deezer),
            _ => None,
        }
    }
}

/// Quality enum for audio quality hierarchy
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Quality {
    UltraHiRes,    // 24-bit, ≤192 kHz (Qobuz only)
    HiRes,        // 24-bit, ≤96 kHz (Qobuz, Tidal MQA)
    Lossless,     // 16-bit, 44.1 kHz (All services)
    High,         // 320 kbps (All services)
    Normal,       // 128 kbps (All services)
}

impl Quality {
    pub fn as_str(&self) -> &'static str {
        match self {
            Quality::UltraHiRes => "ULTRA_HI_RES",
            Quality::HiRes => "HI_RES",
            Quality::Lossless => "LOSSLESS",
            Quality::High => "HIGH",
            Quality::Normal => "NORMAL",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "ULTRA_HI_RES" | "ULTRAHIRES" | "24BIT" => Some(Quality::UltraHiRes),
            "HI_RES" | "HIRES" | "MQA" => Some(Quality::HiRes),
            "LOSSLESS" | "FLAC" | "16BIT" => Some(Quality::Lossless),
            "HIGH" | "320" => Some(Quality::High),
            "NORMAL" | "128" => Some(Quality::Normal),
            _ => None,
        }
    }
    
    pub fn to_tidal_quality(&self) -> &'static str {
        match self {
            Quality::UltraHiRes | Quality::HiRes => "HI_RES",
            Quality::Lossless => "LOSSLESS",
            Quality::High => "HIGH",
            Quality::Normal => "LOW",
        }
    }
    
    pub fn to_qobuz_quality(&self) -> i32 {
        match self {
            Quality::UltraHiRes => 4,
            Quality::HiRes => 3,
            Quality::Lossless => 2,
            Quality::High => 1,
            Quality::Normal => 0,
        }
    }
    
    pub fn to_deezer_quality(&self) -> i32 {
        match self {
            Quality::UltraHiRes | Quality::HiRes | Quality::Lossless => 2,
            Quality::High => 1,
            Quality::Normal => 0,
        }
    }
}

/// Unified track structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedTrack {
    pub id: String,
    pub source: MusicSource,
    pub title: String,
    pub version: Option<String>,
    pub artists: Vec<UnifiedArtist>,
    pub album: UnifiedAlbum,
    pub duration: Option<i64>,
    pub track_number: Option<i32>,
    pub explicit: Option<bool>,
    pub quality: Option<Quality>,
    pub audio_quality: Option<String>,
    pub stream_url: Option<String>,
    pub preview_url: Option<String>,
    pub copyright: Option<String>,
    pub url: Option<String>,
}

/// Unified artist structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedArtist {
    pub id: String,
    pub source: MusicSource,
    pub name: String,
    pub picture: Option<String>,
    pub url: Option<String>,
}

/// Unified album structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedAlbum {
    pub id: String,
    pub source: MusicSource,
    pub title: String,
    pub cover: Option<String>,
    pub cover_big: Option<String>,
    pub duration: Option<i64>,
    pub track_count: Option<i32>,
    pub release_date: Option<String>,
    pub copyright: Option<String>,
    pub url: Option<String>,
    pub artist: Option<UnifiedArtist>,
}

/// Unified playlist structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedPlaylist {
    pub id: String,
    pub source: MusicSource,
    pub title: String,
    pub description: Option<String>,
    pub cover: Option<String>,
    pub track_count: Option<i32>,
    pub duration: Option<i64>,
    pub url: Option<String>,
    pub author: Option<String>,
    pub author_id: Option<String>,
}

/// Stream information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    pub url: String,
    pub quality: Quality,
    pub codec: String,
    pub bitrate: Option<i32>,
    pub sample_rate: Option<i32>,
    pub bit_depth: Option<i32>,
    pub encryption_key: Option<String>,
    pub source: MusicSource,
}

/// API configuration structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QobuzConfig {
    pub email_or_userid: Option<String>,
    pub password_or_token: Option<String>,
    pub app_id: Option<String>,
    pub secrets: Vec<String>,
    pub use_auth_token: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TidalConfig {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub user_id: Option<String>,
    pub country_code: Option<String>,
    pub token_expiry: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeezerConfig {
    pub arl: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub qobuz: QobuzConfig,
    pub tidal: TidalConfig,
    pub deezer: DeezerConfig,
    pub proxy_instances: Vec<ApiInstance>,
}

/// Legacy types for backward compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artists: Vec<Artist>,
    pub album: Album,
    pub duration: Option<i64>,
    pub explicit: Option<bool>,
    pub audio_quality: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub picture: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub title: String,
    pub cover: Option<String>,
    pub track_count: Option<i32>,
    pub release_date: Option<String>,
    pub duration: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub cover: Option<String>,
    pub track_count: Option<i32>,
    pub author: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Creator {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfoLegacy {
    pub url: String,
    pub codec: String,
    pub bitrate: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeSection {
    pub id: String,
    pub title: String,
    pub items: Vec<MediaItemData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItemData {
    #[serde(rename = "type")]
    pub item_type: String,
    pub id: String,
    pub title: String,
    pub cover: Option<String>,
    pub artist: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub tracks: SearchSection,
    pub albums: SearchSection,
    pub artists: SearchSection,
    pub playlists: SearchSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSection {
    pub items: Vec<serde_json::Value>,
    pub total: Option<i32>,
}