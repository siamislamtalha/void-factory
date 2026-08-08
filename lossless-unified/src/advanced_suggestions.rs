use crate::types::*;
use crate::client::{ensure_clients_initialized, QOBUZ_CLIENT, TIDAL_CLIENT, DEEZER_CLIENT, SOUNDCLOUD_CLIENT};
use bex_core::resolver::discovery::Section;
use bex_core::resolver::types::MediaItem;
use lazy_static::lazy_static;
use std::sync::Mutex;
use std::time::{Duration, Instant};

lazy_static! {
    static ref HOME_SECTIONS_CACHE: Mutex<Option<(Vec<HomeSection>, Instant)>> = Mutex::new(None);
    static ref SEARCH_SUGGESTIONS_CACHE: Mutex<HashMap<String, (Vec<SearchSuggestion>, Instant)>> = Mutex::new(HashMap::new());
}

const HOME_CACHE_TTL: Duration = Duration::from_secs(1800); // 30 minutes
const SEARCH_CACHE_TTL: Duration = Duration::from_secs(3600); // 1 hour

/// Fetch home screen sections from all services simultaneously (Lossless FLAC style)
pub async fn fetch_home_sections() -> Result<Vec<Section>, String> {
    ensure_clients_initialized().await;
    
    // Check cache first
    {
        let cache = HOME_SECTIONS_CACHE.lock().unwrap();
        if let Some((sections, timestamp)) = cache.as_ref() {
            if timestamp.elapsed() < HOME_CACHE_TTL {
                return Ok(convert_to_bex_sections(sections));
            }
        }
    }
    
    let mut all_sections = Vec::new();
    
    // Spawn parallel requests to all services
    let qobuz_handle = tokio::spawn(async move {
        let client = QOBUZ_CLIENT.lock().unwrap();
        if let Some(client) = client.as_ref() {
            fetch_qobuz_home_sections(client).await
        } else {
            Ok(vec![])
        }
    });
    
    let tidal_handle = tokio::spawn(async move {
        let client = TIDAL_CLIENT.lock().unwrap();
        if let Some(client) = client.as_ref() {
            fetch_tidal_home_sections(client).await
        } else {
            Ok(vec![])
        }
    });
    
    let deezer_handle = tokio::spawn(async move {
        let client = DEEZER_CLIENT.lock().unwrap();
        if let Some(client) = client.as_ref() {
            fetch_deezer_home_sections(client).await
        } else {
            Ok(vec![])
        }
    });
    
    let soundcloud_handle = tokio::spawn(async move {
        let client = SOUNDCLOUD_CLIENT.lock().unwrap();
        if let Some(client) = client.as_ref() {
            fetch_soundcloud_home_sections(client).await
        } else {
            Ok(vec![])
        }
    });
    
    // Collect results from all services (parallel execution like Lossless FLAC)
    let (qobuz_result, tidal_result, deezer_result, soundcloud_result) = tokio::join!(
        qobuz_handle,
        tidal_handle,
        deezer_handle,
        soundcloud_handle
    );
    
    if let Ok(Ok(sections)) = qobuz_result {
        all_sections.extend(sections);
    }
    
    if let Ok(Ok(sections)) = tidal_result {
        all_sections.extend(sections);
    }
    
    if let Ok(Ok(sections)) = deezer_result {
        all_sections.extend(sections);
    }
    
    if let Ok(Ok(sections)) = soundcloud_result {
        all_sections.extend(sections);
    }
    
    // Cache the results
    {
        let mut cache = HOME_SECTIONS_CACHE.lock().unwrap();
        *cache = Some((all_sections.clone(), Instant::now()));
    }
    
    Ok(convert_to_bex_sections(&all_sections))
}

/// Get cached home sections if available
pub fn get_cached_home_sections() -> Option<Vec<HomeSection>> {
    let cache = HOME_SECTIONS_CACHE.lock().unwrap();
    cache.as_ref().and_then(|(sections, timestamp)| {
        if timestamp.elapsed() < HOME_CACHE_TTL {
            Some(sections.clone())
        } else {
            None
        }
    })
}

/// Fetch search suggestions from all services (Lossless FLAC style)
pub async fn fetch_search_suggestions(query: &str) -> Result<Vec<SearchSuggestion>, String> {
    ensure_clients_initialized().await;
    
    if query.is_empty() {
        return Ok(vec![]);
    }
    
    // Check cache first
    {
        let cache = SEARCH_SUGGESTIONS_CACHE.lock().unwrap();
        if let Some((suggestions, timestamp)) = cache.get(query) {
            if timestamp.elapsed() < SEARCH_CACHE_TTL {
                return Ok(suggestions.clone());
            }
        }
    }
    
    let mut all_suggestions = Vec::new();
    
    // Spawn parallel requests to all services
    let query_clone = query.to_string();
    let qobuz_handle = tokio::spawn(async move {
        let client = QOBUZ_CLIENT.lock().unwrap();
        if let Some(client) = client.as_ref() {
            fetch_qobuz_search_suggestions(client, &query_clone).await
        } else {
            Ok(vec![])
        }
    });
    
    let query_clone = query.to_string();
    let tidal_handle = tokio::spawn(async move {
        let client = TIDAL_CLIENT.lock().unwrap();
        if let Some(client) = client.as_ref() {
            fetch_tidal_search_suggestions(client, &query_clone).await
        } else {
            Ok(vec![])
        }
    });
    
    let query_clone = query.to_string();
    let deezer_handle = tokio::spawn(async move {
        let client = DEEZER_CLIENT.lock().unwrap();
        if let Some(client) = client.as_ref() {
            fetch_deezer_search_suggestions(client, &query_clone).await
        } else {
            Ok(vec![])
        }
    });
    
    let query_clone = query.to_string();
    let soundcloud_handle = tokio::spawn(async move {
        let client = SOUNDCLOUD_CLIENT.lock().unwrap();
        if let Some(client) = client.as_ref() {
            fetch_soundcloud_search_suggestions(client, &query_clone).await
        } else {
            Ok(vec![])
        }
    });
    
    // Collect results from all services (parallel execution)
    let (qobuz_result, tidal_result, deezer_result, soundcloud_result) = tokio::join!(
        qobuz_handle,
        tidal_handle,
        deezer_handle,
        soundcloud_handle
    );
    
    if let Ok(Ok(suggestions)) = qobuz_result {
        all_suggestions.extend(suggestions);
    }
    
    if let Ok(Ok(suggestions)) = tidal_result {
        all_suggestions.extend(suggestions);
    }
    
    if let Ok(Ok(suggestions)) = deezer_result {
        all_suggestions.extend(suggestions);
    }
    
    if let Ok(Ok(suggestions)) = soundcloud_result {
        all_suggestions.extend(suggestions);
    }
    
    // Deduplicate suggestions
    all_suggestions.sort_by(|a, b| a.text.cmp(&b.text));
    all_suggestions.dedup_by(|a, b| a.text == b.text);
    
    // Cache the results
    {
        let mut cache = SEARCH_SUGGESTIONS_CACHE.lock().unwrap();
        cache.insert(query.to_string(), (all_suggestions.clone(), Instant::now()));
    }
    
    Ok(all_suggestions)
}

/// Get cached search suggestions if available
pub fn get_cached_search_suggestions(query: &str) -> Option<Vec<SearchSuggestion>> {
    let cache = SEARCH_SUGGESTIONS_CACHE.lock().unwrap();
    cache.get(query).and_then(|(suggestions, timestamp)| {
        if timestamp.elapsed() < SEARCH_CACHE_TTL {
            Some(suggestions.clone())
        } else {
            None
        }
    })
}

/// Convert home sections to BEX format
fn convert_to_bex_sections(sections: &[HomeSection]) -> Vec<Section> {
    sections.iter().map(|section| {
        Section {
            id: section.id.clone(),
            title: section.title.clone(),
            items: section.items.iter().map(|item| {
                match item {
                    MediaItemData::Track(track) => MediaItem::Track(track.clone()),
                    MediaItemData::Album(album) => MediaItem::Album(album.clone()),
                    MediaItemData::Artist(artist) => MediaItem::Artist(artist.clone()),
                    MediaItemData::Playlist(playlist) => MediaItem::Playlist(playlist.clone()),
                }
            }).collect(),
        }
    }).collect()
}

/// Fetch Qobuz home sections
async fn fetch_qobuz_home_sections(client: &crate::qobuz_client::QobuzClient) -> Result<Vec<HomeSection>, String> {
    let mut sections = Vec::new();
    
    // Fetch different featured content types from Qobuz (streamrip style)
    let featured_types = vec!["new-releases", "best-sellers", "press-awards", "editor-picks"];
    
    for feature_type in featured_types {
        match client.get_featured(feature_type, 10).await {
            Ok(albums) => {
                let title = format!("Qobuz {}", feature_type.replace("-", " "));
                let section = HomeSection {
                    id: format!("qobuz_{}", feature_type),
                    title,
                    items: albums.into_iter().map(MediaItemData::Album).collect(),
                    page_token: None,
                    source: MusicSource::Qobuz,
                    section_type: HomeSectionType::Featured,
                };
                sections.push(section);
            }
            Err(e) => {
                eprintln!("Failed to fetch Qobuz {}: {}", feature_type, e);
            }
        }
    }
    
    Ok(sections)
}

/// Fetch Tidal home sections
async fn fetch_tidal_home_sections(client: &crate::tidal_client::TidalClient) -> Result<Vec<HomeSection>, String> {
    let mut sections = Vec::new();
    
    // Fetch featured playlists from Tidal
    match client.get_featured_playlists(10).await {
        Ok(playlists) => {
            let section = HomeSection {
                id: "tidal_featured".to_string(),
                title: "Tidal Featured Playlists".to_string(),
                items: playlists.into_iter().map(MediaItemData::Playlist).collect(),
                page_token: None,
                source: MusicSource::Tidal,
                section_type: HomeSectionType::Featured,
            };
            sections.push(section);
        }
        Err(e) => {
            eprintln!("Failed to fetch Tidal featured playlists: {}", e);
        }
    }
    
    // Fetch new releases from Tidal
    match client.get_featured(10).await {
        Ok(albums) => {
            let section = HomeSection {
                id: "tidal_new".to_string(),
                title: "Tidal New Releases".to_string(),
                items: albums.into_iter().map(MediaItemData::Album).collect(),
                page_token: None,
                source: MusicSource::Tidal,
                section_type: HomeSectionType::Featured,
            };
            sections.push(section);
        }
        Err(e) => {
            eprintln!("Failed to fetch Tidal new releases: {}", e);
        }
    }
    
    Ok(sections)
}

/// Fetch Deezer home sections
async fn fetch_deezer_home_sections(client: &crate::deezer_client::DeezerClient) -> Result<Vec<HomeSection>, String> {
    let mut sections = Vec::new();
    
    // Fetch featured/new releases from Deezer
    match client.get_featured(10).await {
        Ok(albums) => {
            let section = HomeSection {
                id: "deezer_featured".to_string(),
                title: "Deezer Featured".to_string(),
                items: albums.into_iter().map(MediaItemData::Album).collect(),
                page_token: None,
                source: MusicSource::Deezer,
                section_type: HomeSectionType::Featured,
            };
            sections.push(section);
        }
        Err(e) => {
            eprintln!("Failed to fetch Deezer featured: {}", e);
        }
    }
    
    Ok(sections)
}

/// Fetch SoundCloud home sections
async fn fetch_soundcloud_home_sections(client: &crate::soundcloud_client::SoundCloudClient) -> Result<Vec<HomeSection>, String> {
    let mut sections = Vec::new();
    
    // Fetch trending tracks from SoundCloud
    match client.get_trending_tracks(10).await {
        Ok(tracks) => {
            let section = HomeSection {
                id: "soundcloud_trending".to_string(),
                title: "SoundCloud Trending".to_string(),
                items: tracks.into_iter().map(MediaItemData::Track).collect(),
                page_token: None,
                source: MusicSource::SoundCloud,
                section_type: HomeSectionType::Trending,
            };
            sections.push(section);
        }
        Err(e) => {
            eprintln!("Failed to fetch SoundCloud trending: {}", e);
        }
    }
    
    Ok(sections)
}

/// Fetch Qobuz search suggestions
async fn fetch_qobuz_search_suggestions(client: &crate::qobuz_client::QobuzClient, query: &str) -> Result<Vec<SearchSuggestion>, String> {
    let tracks = client.search_tracks(query, 5).await?;
    
    Ok(tracks.into_iter().map(|track| {
        SearchSuggestion {
            text: format!("{} - {}", track.title, track.artist.as_ref().map(|a| &a.name).unwrap_or(&"Unknown".to_string())),
            subtitle: track.album.as_ref().map(|a| a.title.clone()),
            item_type: SuggestionItemType::Track,
            source: MusicSource::Qobuz,
        }
    }).collect())
}

/// Fetch Tidal search suggestions
async fn fetch_tidal_search_suggestions(client: &crate::tidal_client::TidalClient, query: &str) -> Result<Vec<SearchSuggestion>, String> {
    let tracks = client.search_tracks(query, 5).await?;
    
    Ok(tracks.into_iter().map(|track| {
        SearchSuggestion {
            text: format!("{} - {}", track.title, track.artist.as_ref().map(|a| &a.name).unwrap_or(&"Unknown".to_string())),
            subtitle: track.album.as_ref().map(|a| a.title.clone()),
            item_type: SuggestionItemType::Track,
            source: MusicSource::Tidal,
        }
    }).collect())
}

/// Fetch Deezer search suggestions
async fn fetch_deezer_search_suggestions(client: &crate::deezer_client::DeezerClient, query: &str) -> Result<Vec<SearchSuggestion>, String> {
    let tracks = client.search_tracks(query, 5).await?;
    
    Ok(tracks.into_iter().map(|track| {
        SearchSuggestion {
            text: format!("{} - {}", track.title, track.artist.as_ref().map(|a| &a.name).unwrap_or(&"Unknown".to_string())),
            subtitle: track.album.as_ref().map(|a| a.title.clone()),
            item_type: SuggestionItemType::Track,
            source: MusicSource::Deezer,
        }
    }).collect())
}

/// Fetch SoundCloud search suggestions
async fn fetch_soundcloud_search_suggestions(client: &crate::soundcloud_client::SoundCloudClient, query: &str) -> Result<Vec<SearchSuggestion>, String> {
    let tracks = client.search_tracks(query, 5).await?;
    
    Ok(tracks.into_iter().map(|track| {
        SearchSuggestion {
            text: format!("{} - {}", track.title, track.artist.as_ref().map(|a| &a.name).unwrap_or(&"Unknown".to_string())),
            subtitle: track.album.as_ref().map(|a| a.title.clone()),
            item_type: SuggestionItemType::Track,
            source: MusicSource::SoundCloud,
        }
    }).collect())
}

/// Get cached home sections
pub fn get_cached_home_sections() -> Vec<HomeSection> {
    HOME_SECTIONS_CACHE.lock().unwrap().clone()
}

/// Get cached search suggestions
pub fn get_cached_search_suggestions() -> Vec<SearchSuggestion> {
    SEARCH_SUGGESTIONS_CACHE.lock().unwrap().clone()
}

// Qobuz home sections
async fn fetch_qobuz_home_sections(client: &crate::qobuz_client::QobuzClient) -> Result<Vec<Section>, String> {
    let mut sections = Vec::new();
    
    // Featured albums
    if let Ok(featured) = client.get_featured_albums("best-sellers", 20).await {
        let items: Vec<MediaItem> = featured.into_iter().map(MediaItem::Album).collect();
        sections.push(Section {
            id: "qobuz-best-sellers".to_string(),
            title: "Qobuz Best Sellers".to_string(),
            items,
            page_token: None,
        });
    }
    
    // New releases
    if let Ok(new_releases) = client.get_featured_albums("new-releases", 20).await {
        let items: Vec<MediaItem> = new_releases.into_iter().map(MediaItem::Album).collect();
        sections.push(Section {
            id: "qobuz-new-releases".to_string(),
            title: "Qobuz New Releases".to_string(),
            items,
            page_token: None,
        });
    }
    
    // Press awards
    if let Ok(press_awards) = client.get_featured_albums("press-awards", 20).await {
        let items: Vec<MediaItem> = press_awards.into_iter().map(MediaItem::Album).collect();
        sections.push(Section {
            id: "qobuz-press-awards".to_string(),
            title: "Qobuz Press Awards".to_string(),
            items,
            page_token: None,
        });
    }
    
    Ok(sections)
}

// Tidal home sections
async fn fetch_tidal_home_sections(client: &crate::tidal_client::TidalClient) -> Result<Vec<Section>, String> {
    let mut sections = Vec::new();
    
    // Featured playlists
    if let Ok(featured) = client.get_featured_playlists(20).await {
        let items: Vec<MediaItem> = featured.into_iter().map(MediaItem::Playlist).collect();
        sections.push(Section {
            id: "tidal-featured-playlists".to_string(),
            title: "Tidal Featured Playlists".to_string(),
            items,
            page_token: None,
        });
    }
    
    // Top tracks
    if let Ok(top_tracks) = client.get_top_tracks(20).await {
        let items: Vec<MediaItem> = top_tracks.into_iter().map(MediaItem::Track).collect();
        sections.push(Section {
            id: "tidal-top-tracks".to_string(),
            title: "Tidal Top Tracks".to_string(),
            items,
            page_token: None,
        });
    }
    
    Ok(sections)
}

// Deezer home sections
async fn fetch_deezer_home_sections(client: &crate::deezer_client::DeezerClient) -> Result<Vec<Section>, String> {
    let mut sections = Vec::new();
    
    // Charts
    if let Ok(charts) = client.get_charts(20).await {
        let items: Vec<MediaItem> = charts.into_iter().map(MediaItem::Track).collect();
        sections.push(Section {
            id: "deezer-charts".to_string(),
            title: "Deezer Charts".to_string(),
            items,
            page_token: None,
        });
    }
    
    // New releases
    if let Ok(new_releases) = client.get_new_releases(20).await {
        let items: Vec<MediaItem> = new_releases.into_iter().map(MediaItem::Album).collect();
        sections.push(Section {
            id: "deezer-new-releases".to_string(),
            title: "Deezer New Releases".to_string(),
            items,
            page_token: None,
        });
    }
    
    Ok(sections)
}

// SoundCloud home sections
async fn fetch_soundcloud_home_sections(client: &crate::soundcloud_client::SoundCloudClient) -> Result<Vec<Section>, String> {
    let mut sections = Vec::new();
    
    // Trending tracks
    if let Ok(trending) = client.get_trending_tracks(20).await {
        let items: Vec<MediaItem> = trending.into_iter().map(MediaItem::Track).collect();
        sections.push(Section {
            id: "soundcloud-trending".to_string(),
            title: "SoundCloud Trending".to_string(),
            items,
            page_token: None,
        });
    }
    
    Ok(sections)
}

// Search suggestions for each service
async fn fetch_qobuz_search_suggestions(client: &crate::qobuz_client::QobuzClient, query: &str) -> Result<Vec<SearchSuggestion>, String> {
    let mut suggestions = Vec::new();
    
    // Search for tracks
    if let Ok(tracks) = client.search_tracks(query, 5).await {
        for track in tracks {
            suggestions.push(SearchSuggestion {
                text: format!("{} - {}", track.artist.as_ref().map(|a| &a.name).unwrap_or("Unknown"), track.title),
                suggestion_type: SuggestionType::Track,
                metadata: Some(SuggestionMetadata {
                    id: format!("qobuz:{}", track.id),
                    artist: track.artist.as_ref().map(|a| a.name.clone()),
                    cover: track.album.as_ref().and_then(|a| a.cover.clone()),
                    source: MusicSource::Qobuz,
                }),
            });
        }
    }
    
    Ok(suggestions)
}

async fn fetch_tidal_search_suggestions(client: &crate::tidal_client::TidalClient, query: &str) -> Result<Vec<SearchSuggestion>, String> {
    let mut suggestions = Vec::new();
    
    if let Ok(tracks) = client.search_tracks(query, 5).await {
        for track in tracks {
            suggestions.push(SearchSuggestion {
                text: format!("{} - {}", track.artist.as_ref().map(|a| &a.name).unwrap_or("Unknown"), track.title),
                suggestion_type: SuggestionType::Track,
                metadata: Some(SuggestionMetadata {
                    id: format!("tidal:{}", track.id),
                    artist: track.artist.as_ref().map(|a| a.name.clone()),
                    cover: track.album.as_ref().and_then(|a| a.cover.clone()),
                    source: MusicSource::Tidal,
                }),
            });
        }
    }
    
    Ok(suggestions)
}

async fn fetch_deezer_search_suggestions(client: &crate::deezer_client::DeezerClient, query: &str) -> Result<Vec<SearchSuggestion>, String> {
    let mut suggestions = Vec::new();
    
    if let Ok(tracks) = client.search_tracks(query, 5).await {
        for track in tracks {
            suggestions.push(SearchSuggestion {
                text: format!("{} - {}", track.artist.as_ref().map(|a| &a.name).unwrap_or("Unknown"), track.title),
                suggestion_type: SuggestionType::Track,
                metadata: Some(SuggestionMetadata {
                    id: format!("deezer:{}", track.id),
                    artist: track.artist.as_ref().map(|a| a.name.clone()),
                    cover: track.album.as_ref().and_then(|a| a.cover.clone()),
                    source: MusicSource::Deezer,
                }),
            });
        }
    }
    
    Ok(suggestions)
}

/// Convert sections to home sections with type information
fn convert_to_home_sections(sections: &[Section]) -> Vec<HomeSection> {
    sections.iter().map(|section| {
        let section_type = determine_section_type(&section.id);
        HomeSection {
            id: section.id.clone(),
            title: section.title.clone(),
            items: section.items.iter().map(convert_media_item).collect(),
            page_token: section.page_token.clone(),
            source: determine_source_from_id(&section.id),
            section_type,
        }
    }).collect()
}

fn determine_section_type(section_id: &str) -> HomeSectionType {
    if section_id.contains("best-sellers") || section_id.contains("charts") {
        HomeSectionType::Charts
    } else if section_id.contains("new-releases") {
        HomeSectionType::NewReleases
    } else if section_id.contains("featured") {
        HomeSectionType::Featured
    } else if section_id.contains("top") {
        HomeSectionType::TopTracks
    } else if section_id.contains("trending") {
        HomeSectionType::Trending
    } else {
        HomeSectionType::Editorial
    }
}

fn determine_source_from_id(section_id: &str) -> MusicSource {
    if section_id.starts_with("qobuz") {
        MusicSource::Qobuz
    } else if section_id.starts_with("tidal") {
        MusicSource::Tidal
    } else if section_id.starts_with("deezer") {
        MusicSource::Deezer
    } else if section_id.starts_with("soundcloud") {
        MusicSource::SoundCloud
    } else {
        MusicSource::UnifiedPlayback
    }
}

fn convert_media_item(item: &MediaItem) -> MediaItemData {
    match item {
        MediaItem::Track(track) => MediaItemData {
            id: track.id.clone(),
            title: track.title.clone(),
            artist: track.artists.first().map(|a| a.name.clone()),
            cover: track.album.as_ref().and_then(|a| a.cover.clone()),
            duration: track.duration,
            item_type: MediaType::Track,
        },
        MediaItem::Album(album) => MediaItemData {
            id: album.id.clone(),
            title: album.title.clone(),
            artist: album.artist.as_ref().map(|a| a.name.clone()),
            cover: album.cover.clone(),
            duration: album.duration,
            item_type: MediaType::Album,
        },
        MediaItem::Artist(artist) => MediaItemData {
            id: artist.id.clone(),
            title: artist.name.clone(),
            artist: Some(artist.name.clone()),
            cover: artist.picture.clone(),
            duration: None,
            item_type: MediaType::Artist,
        },
        MediaItem::Playlist(playlist) => MediaItemData {
            id: playlist.id.clone(),
            title: playlist.title.clone(),
            artist: playlist.creator.as_ref().map(|c| c.name.clone()),
            cover: playlist.picture.clone(),
            duration: playlist.duration,
            item_type: MediaType::Playlist,
        },
    }
}

/// Deduplicate suggestions based on text
fn deduplicate_suggestions(suggestions: Vec<SearchSuggestion>) -> Vec<SearchSuggestion> {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    
    for suggestion in suggestions {
        let key = suggestion.text.to_lowercase();
        if !seen.contains(&key) {
            seen.insert(key);
            unique.push(suggestion);
        }
    }
    
    unique
}