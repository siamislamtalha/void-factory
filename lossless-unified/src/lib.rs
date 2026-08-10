//! Unified Lossless FLAC Music Plugin
//!
//! This plugin combines Qobuz, Tidal, Deezer, and SoundCloud into a single unified interface:
//! - Parallel search across all services for maximum results (Lossless FLAC style)
//! - Automatic quality selection (prioritizes Dolby Atmos, 24-bit FLAC, MQA, etc.)
//! - Home screen suggestions from all services
//! - High-quality song streaming and downloads
//! - Decryption support for Tidal MQA and Deezer encrypted streams
//! - Source tagging for result identification
//! - Monochrome API compatibility
//!
//! # Configuration
//!
//! Set credentials using the proxy module functions:
//! - Qobuz: `set_qobuz_credentials(email, password, app_id, secrets)`
//! - Tidal: `set_tidal_credentials(access_token, refresh_token, user_id, country_code)`
//! - Deezer: `set_deezer_credentials(arl)`
//!
//! # Quality Hierarchy (Lossless FLAC + Monochrome)
//!
//! 1. Dolby Atmos (EAC3_JOC) - Highest quality, spatial audio
//! 2. Ultra Hi-Res (24-bit, ≤192 kHz) - Qobuz only
//! 3. Hi-Res (24-bit, ≤96 kHz) - Qobuz, Tidal MQA
//! 4. Lossless (16-bit, 44.1 kHz) - All services
//! 5. High (320 kbps) - All services
//! 6. Normal (128 kbps) - All services
//!
//! # Features
//!
//! - Simultaneous API calls to all services (parallel execution)
//! - Intelligent quality ranking and selection
//! - Automatic fallback to lower quality if highest unavailable
//! - Streamrip-style fast downloads with chunk optimization
//! - Monochrome API compatibility for existing integrations
//! - Caching for reduced API calls (30-minute TTL)
//! - Support for ISRC-based cross-service lookup

mod client;
mod types;
mod mapper;
mod proxy;
mod suggestions;
mod qobuz_client;
mod tidal_client;
mod deezer_client;
mod soundcloud_client;
mod unified_client;
mod decryption;
mod monochrome_api;
mod advanced_suggestions;
mod download_manager;

// Re-export Monochrome API functions for external use
pub use monochrome_api::{
    get_track_info,
    get_album_info,
    get_artist_info,
    get_stream_url,
    search as monochrome_search,
    get_home_data,
    get_search_suggestions,
};

// Re-export advanced suggestions and download manager
pub use advanced_suggestions::{
    fetch_home_sections,
    fetch_search_suggestions,
    get_cached_home_sections,
    get_cached_search_suggestions,
};

pub use download_manager::{
    DownloadManager,
};

// Re-export advanced client functions
pub use client::{
    find_best_quality_track,
};

use bex_core::resolver::{
    data_source::{
        AlbumDetails, ArtistDetails, Guest as DataSourceGuest, PagedAlbums, PagedMediaItems,
        PagedTracks, PlaylistDetails, SearchFilter, StreamSource,
    },
    discovery::{Guest as DiscoveryGuest, Section},
    types::MediaItem,
};

struct Component;

impl DiscoveryGuest for Component {
    fn get_home_sections() -> Result<Vec<Section>, String> {
        // Use a blocking runtime for async calls
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            suggestions::fetch_home_sections().await.map_err(|e| e.to_string())
        })
    }

    fn load_more(section_id: String, page_token: String) -> Result<Vec<MediaItem>, String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            suggestions::load_more_section(&section_id, &page_token).await.map_err(|e| e.to_string())
        })
    }
}

impl DataSourceGuest for Component {
    fn get_track_details(id: String) -> Result<bex_core::resolver::types::Track, String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            client::get_track_details(&id).await.map_err(|e| e.to_string())
        })
    }

    fn get_album_details(id: String) -> Result<AlbumDetails, String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            client::get_album_details(&id).await.map_err(|e| e.to_string())
        })
    }

    fn get_artist_details(id: String) -> Result<ArtistDetails, String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            client::get_artist_details(&id).await.map_err(|e| e.to_string())
        })
    }

    fn more_artist_albums(id: String, page_token: String) -> Result<PagedAlbums, String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            client::more_artist_albums(&id, &page_token).await.map_err(|e| e.to_string())
        })
    }

    fn get_playlist_details(id: String) -> Result<PlaylistDetails, String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            client::get_playlist_details(&id).await.map_err(|e| e.to_string())
        })
    }

    fn more_album_tracks(id: String, page_token: String) -> Result<PagedTracks, String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            client::more_album_tracks(&id, &page_token).await.map_err(|e| e.to_string())
        })
    }

    fn more_playlist_tracks(id: String, page_token: String) -> Result<PagedTracks, String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            client::more_playlist_tracks(&id, &page_token).await.map_err(|e| e.to_string())
        })
    }

    fn get_radio_tracks(
        reference_id: String,
        page_token: Option<String>,
    ) -> Result<PagedTracks, String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            client::get_radio_tracks(&reference_id, page_token.as_deref()).await.map_err(|e| e.to_string())
        })
    }

    fn get_streams(track_id: String) -> Result<Vec<StreamSource>, String> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            client::get_stream_source(&track_id, "LOSSLESS").await.map_err(|e| e.to_string())
        })
    }

    fn get_segments(_track_id: String) -> Result<Vec<bex_core::resolver::types::MediaSegment>, String> {
        Ok(vec![])
    }

    fn search(
        query: String,
        filter: SearchFilter,
        page_token: Option<String>,
    ) -> Result<PagedMediaItems, String> {
        let page = page_token.and_then(|p| p.parse().ok()).unwrap_or(1);
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        rt.block_on(async {
            client::search(&query, filter, page).await.map_err(|e| e.to_string())
        })
    }
}

bex_core::export_resolver!(Component);