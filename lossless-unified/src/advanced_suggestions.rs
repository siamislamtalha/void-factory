use crate::types::*;
use crate::client::{ensure_clients_initialized, QOBUZ_CLIENT, TIDAL_CLIENT, DEEZER_CLIENT, SOUNDCLOUD_CLIENT};
use bex_core::resolver::discovery::Section;
use bex_core::resolver::types::MediaItem;
use lazy_static::lazy_static;
use std::collections::HashMap;
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

    // Clone client references before spawning to avoid holding locks across await
    let qobuz_client = {
        let client = QOBUZ_CLIENT.lock().unwrap();
        client.clone()
    };
    let tidal_client = {
        let client = TIDAL_CLIENT.lock().unwrap();
        client.clone()
    };
    let deezer_client = {
        let client = DEEZER_CLIENT.lock().unwrap();
        client.clone()
    };
    let soundcloud_client = {
        let client = SOUNDCLOUD_CLIENT.lock().unwrap();
        client.clone()
    };

    // Spawn parallel requests to all services
    let qobuz_handle = tokio::spawn(async move {
        if let Some(client) = qobuz_client {
            fetch_qobuz_home_sections(&client).await
        } else {
            Ok(vec![])
        }
    });

    let tidal_handle = tokio::spawn(async move {
        if let Some(client) = tidal_client {
            fetch_tidal_home_sections(&client).await
        } else {
            Ok(vec![])
        }
    });

    let deezer_handle = tokio::spawn(async move {
        if let Some(client) = deezer_client {
            fetch_deezer_home_sections(&client).await
        } else {
            Ok(vec![])
        }
    });

    let soundcloud_handle = tokio::spawn(async move {
        if let Some(client) = soundcloud_client {
            fetch_soundcloud_home_sections(&client).await
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

    // Clone client references before spawning to avoid holding locks across await
    let qobuz_client = {
        let client = QOBUZ_CLIENT.lock().unwrap();
        client.clone()
    };
    let tidal_client = {
        let client = TIDAL_CLIENT.lock().unwrap();
        client.clone()
    };
    let deezer_client = {
        let client = DEEZER_CLIENT.lock().unwrap();
        client.clone()
    };
    let soundcloud_client = {
        let client = SOUNDCLOUD_CLIENT.lock().unwrap();
        client.clone()
    };

    // Spawn parallel requests to all services
    let query_clone = query.to_string();
    let qobuz_handle = tokio::spawn(async move {
        if let Some(client) = qobuz_client {
            fetch_qobuz_search_suggestions(&client, &query_clone).await
        } else {
            Ok(vec![])
        }
    });

    let tidal_handle = tokio::spawn(async move {
        if let Some(client) = tidal_client {
            fetch_tidal_search_suggestions(&client, &query_clone).await
        } else {
            Ok(vec![])
        }
    });

    let deezer_handle = tokio::spawn(async move {
        if let Some(client) = deezer_client {
            fetch_deezer_search_suggestions(&client, &query_clone).await
        } else {
            Ok(vec![])
        }
    });

    let soundcloud_handle = tokio::spawn(async move {
        if let Some(client) = soundcloud_client {
            fetch_soundcloud_search_suggestions(&client, &query_clone).await
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
    use crate::mapper::map_media_item;
    use bex_core::resolver::discovery::SectionType;
    sections.iter().map(|section| {
        Section {
            id: section.id.clone(),
            title: section.title.clone(),
            subtitle: Some(section.section_type.as_str().to_string()),
            card_type: SectionType::Grid,
            items: section.items.iter().filter_map(|item| {
                Some(map_media_item(item.clone()))
            }).collect(),
            more_link: None,
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
            text: format!("{} - {}", track.title, track.artists.first().map(|a| &a.name).unwrap_or(&"Unknown".to_string())),
            suggestion_type: SuggestionType::Track,
            metadata: Some(SuggestionMetadata {
                id: track.id.clone(),
                artist: track.artists.first().map(|a| a.name.clone()),
                cover: track.album.as_ref().and_then(|a| a.cover.clone()),
                source: MusicSource::Qobuz,
            }),
        }
    }).collect())
}

/// Fetch Tidal search suggestions
async fn fetch_tidal_search_suggestions(client: &crate::tidal_client::TidalClient, query: &str) -> Result<Vec<SearchSuggestion>, String> {
    let tracks = client.search_tracks(query, 5).await?;

    Ok(tracks.into_iter().map(|track| {
        SearchSuggestion {
            text: format!("{} - {}", track.title, track.artists.first().map(|a| &a.name).unwrap_or(&"Unknown".to_string())),
            suggestion_type: SuggestionType::Track,
            metadata: Some(SuggestionMetadata {
                id: track.id.clone(),
                artist: track.artists.first().map(|a| a.name.clone()),
                cover: track.album.as_ref().and_then(|a| a.cover.clone()),
                source: MusicSource::Tidal,
            }),
        }
    }).collect())
}

/// Fetch Deezer search suggestions
async fn fetch_deezer_search_suggestions(client: &crate::deezer_client::DeezerClient, query: &str) -> Result<Vec<SearchSuggestion>, String> {
    let tracks = client.search_tracks(query, 5).await?;

    Ok(tracks.into_iter().map(|track| {
        SearchSuggestion {
            text: format!("{} - {}", track.title, track.artists.first().map(|a| &a.name).unwrap_or(&"Unknown".to_string())),
            suggestion_type: SuggestionType::Track,
            metadata: Some(SuggestionMetadata {
                id: track.id.clone(),
                artist: track.artists.first().map(|a| a.name.clone()),
                cover: track.album.as_ref().and_then(|a| a.cover.clone()),
                source: MusicSource::Deezer,
            }),
        }
    }).collect())
}

/// Fetch SoundCloud search suggestions
async fn fetch_soundcloud_search_suggestions(client: &crate::soundcloud_client::SoundCloudClient, query: &str) -> Result<Vec<SearchSuggestion>, String> {
    let tracks = client.search_tracks(query, 5).await?;

    Ok(tracks.into_iter().map(|track| {
        SearchSuggestion {
            text: format!("{} - {}", track.title, track.artists.first().map(|a| &a.name).unwrap_or(&"Unknown".to_string())),
            suggestion_type: SuggestionType::Track,
            metadata: Some(SuggestionMetadata {
                id: track.id.clone(),
                artist: track.artists.first().map(|a| a.name.clone()),
                cover: track.album.as_ref().and_then(|a| a.cover.clone()),
                source: MusicSource::SoundCloud,
            }),
        }
    }).collect())
}

/// Convert sections to home sections with type information
fn convert_to_home_sections(sections: &[Section]) -> Vec<HomeSection> {
    sections.iter().map(|section| {
        let section_type = determine_section_type(&section.id);
        HomeSection {
            id: section.id.clone(),
            title: section.title.clone(),
            items: section.items.iter().map(convert_media_item).collect(),
            page_token: section.more_link.clone(),
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
        MediaItem::Track(track) => {
            let unified_track = UnifiedTrack {
                id: track.id.clone(),
                title: track.title.clone(),
                duration: track.duration_ms.map(|d| (d / 1000) as u32).unwrap_or(0),
                track_number: None,
                volume_number: None,
                replay_gain: None,
                peak: None,
                available: true,
                audio_quality: None,
                audio_modes: None,
                artist: track.artists.first().map(|a| UnifiedArtist {
                    id: a.id.clone(),
                    name: a.name.clone(),
                    picture: a.thumbnail.as_ref().map(|t| t.url.clone()),
                    url: a.url.clone(),
                    source: determine_source_from_id(&track.id),
                }),
                artists: Some(track.artists.iter().map(|a| UnifiedArtist {
                    id: a.id.clone(),
                    name: a.name.clone(),
                    picture: a.thumbnail.as_ref().map(|t| t.url.clone()),
                    url: a.url.clone(),
                    source: determine_source_from_id(&track.id),
                }).collect()),
                album: track.album.as_ref().map(|a| UnifiedAlbum {
                    id: a.id.clone(),
                    title: a.title.clone(),
                    cover: a.thumbnail.as_ref().map(|t| t.url.clone()),
                    duration: None,
                    track_count: None,
                    release_date: a.subtitle.clone(),
                    artist: None,
                    artists: None,
                    url: a.url.clone(),
                    source: determine_source_from_id(&track.id),
                }),
                source: determine_source_from_id(&track.id),
                qualities_available: vec![Quality::LosslessFlac],
            };
            MediaItemData::Track(unified_track)
        }
        MediaItem::Album(_) => {
            // Not supported in BEX types
            MediaItemData::Track(UnifiedTrack {
                id: "placeholder".to_string(),
                title: "Unsupported".to_string(),
                duration: 0,
                track_number: None,
                volume_number: None,
                replay_gain: None,
                peak: None,
                available: true,
                audio_quality: None,
                audio_modes: None,
                artist: None,
                artists: None,
                album: None,
                source: MusicSource::Qobuz,
                qualities_available: vec![],
            })
        }
        MediaItem::Artist(_) => {
            // Not supported in BEX types
            MediaItemData::Artist(UnifiedArtist {
                id: "placeholder".to_string(),
                name: "Unsupported".to_string(),
                picture: None,
                url: None,
                source: MusicSource::Qobuz,
            })
        }
        MediaItem::Playlist(_) => {
            // Not supported in BEX types
            MediaItemData::Playlist(UnifiedPlaylist {
                id: "placeholder".to_string(),
                title: "Unsupported".to_string(),
                description: None,
                picture: None,
                duration: None,
                track_count: None,
                last_updated: None,
                creator: None,
                source: MusicSource::Qobuz,
            })
        }
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