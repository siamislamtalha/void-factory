//! Universal Downloader Plugin — Multi-method music download with fallback support
//!
//! This plugin provides reliable music downloading by implementing multiple download methods
//! from various APK reference implementations with automatic fallback:
//!
//! **YouTube Download Methods:**
//! - Innertube API with 8 backup API keys from various music apps
//! - Multiple client fallbacks: ANDROID_VR, IOS, TVHTML5, WEB_REMIX
//! - Signature cipher decoding for encrypted streams
//! - Range request support to avoid YouTube throttling
//! - Visitor data extraction and caching
//!
//! **JioSaavn Download Methods:**
//! - DES-ECB decryption for encrypted stream URLs
//! - Multiple server endpoints with automatic rotation
//! - Quality selection (96kbps, 128kbps, 160kbps, 320kbps)
//! - Remote config support for dynamic server updates
//!
//! **Direct HTTP Methods:**
//! - Fallback to direct HTTP streaming for unsupported sources
//! - Support for various audio formats (m4a, mp3, webm)
//! - Progressive download with resume capability
//!
//! **Credential Pools:**
//! - YouTube API keys from InnerTune, Kreate, Musify, OuterTune, OpenTune, RiMusic
//! - JioSaavn servers from Echo-Music implementation
//! - Automatic rotation on failure

mod credentials;
mod youtube_downloader;
mod jiosaavn_downloader;
mod http_downloader;
mod stream_resolver;

use bex_core::resolver::{
    data_source::{
        AlbumDetails, ArtistDetails, Guest as DataSourceGuest,
        PagedAlbums, PagedMediaItems, PagedTracks, PlaylistDetails,
        SearchFilter, StreamSource,
    },
    discovery::{Guest as DiscoveryGuest, Section},
    types::MediaItem,
};

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DownloadMethod {
    YouTubeInnertube,
    YouTubeAndroidVr,
    YouTubeIos,
    YouTubeTvHtml5,
    JioSaavnDes,
    DirectHttp,
}

#[allow(dead_code)]
impl DownloadMethod {
    fn as_str(&self) -> &'static str {
        match self {
            Self::YouTubeInnertube => "YouTube Innertube",
            Self::YouTubeAndroidVr => "YouTube Android VR",
            Self::YouTubeIos => "YouTube iOS",
            Self::YouTubeTvHtml5 => "YouTube TVHTML5",
            Self::JioSaavnDes => "JioSaavn DES",
            Self::DirectHttp => "Direct HTTP",
        }
    }

    fn priority(&self) -> u8 {
        match self {
            Self::YouTubeInnertube => 10,
            Self::YouTubeAndroidVr => 9,
            Self::YouTubeIos => 8,
            Self::YouTubeTvHtml5 => 7,
            Self::JioSaavnDes => 6,
            Self::DirectHttp => 1,
        }
    }
}

struct Component;

impl DiscoveryGuest for Component {
    fn get_home_sections() -> Result<Vec<Section>, String> {
        // This is a downloader plugin, not a discovery plugin
        Ok(vec![])
    }

    fn load_more(_section_id: String, _page_token: String) -> Result<Vec<MediaItem>, String> {
        Ok(vec![])
    }
}

impl DataSourceGuest for Component {
    fn get_track_details(id: String) -> Result<bex_core::resolver::types::Track, String> {
        // Delegate to stream resolver for track details
        stream_resolver::get_track_details(&id).map_err(|e| e.to_string())
    }

    fn get_album_details(_id: String) -> Result<AlbumDetails, String> {
        Err("Album details not supported by downloader plugin".to_string())
    }

    fn more_album_tracks(_id: String, _page_token: String) -> Result<PagedTracks, String> {
        Err("Album tracks not supported by downloader plugin".to_string())
    }

    fn get_artist_details(_id: String) -> Result<ArtistDetails, String> {
        Err("Artist details not supported by downloader plugin".to_string())
    }

    fn more_artist_albums(_id: String, _page_token: String) -> Result<PagedAlbums, String> {
        Err("Artist albums not supported by downloader plugin".to_string())
    }

    fn get_playlist_details(_id: String) -> Result<PlaylistDetails, String> {
        Err("Playlist details not supported by downloader plugin".to_string())
    }

    fn more_playlist_tracks(_id: String, _page_token: String) -> Result<PagedTracks, String> {
        Err("Playlist tracks not supported by downloader plugin".to_string())
    }

    fn get_streams(track_id: String) -> Result<Vec<StreamSource>, String> {
        // Main function: try all download methods with fallback
        stream_resolver::get_streams_with_fallback(&track_id).map_err(|e| e.to_string())
    }

    fn get_segments(_track_id: String) -> Result<Vec<bex_core::resolver::types::MediaSegment>, String> {
        Ok(vec![])
    }

    fn get_radio_tracks(
        _reference_id: String,
        _page_token: Option<String>,
    ) -> Result<PagedTracks, String> {
        Err("Radio tracks not supported by downloader plugin".to_string())
    }

    fn search(_query: String, _filter: SearchFilter, _page_token: Option<String>) -> Result<PagedMediaItems, String> {
        // This is a downloader plugin, not a search plugin
        Err("Search not supported by downloader plugin".to_string())
    }
}

bex_core::export_resolver!(Component);
