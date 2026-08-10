use crate::types::*;
use crate::mapper::parse_source_id;
use crate::proxy::{get_next_instance, apply_proxy_transform, initialize_api_config};
use crate::mapper::*;
use crate::qobuz_client::QobuzClient;
use crate::tidal_client::TidalClient;
use crate::deezer_client::DeezerClient;
use crate::soundcloud_client::SoundCloudClient;
use crate::unified_client::UnifiedClient;
use bex_core::resolver::{
    data_source::{
        AlbumDetails, ArtistDetails, PagedAlbums, PagedMediaItems, PagedTracks,
        PlaylistDetails, SearchFilter, StreamSource,
    },
    types::{MediaItem, Track as BexTrack, Artwork as BexArtwork, ImageLayout},
};
use reqwest::Client;
use std::collections::HashMap;
use lazy_static::lazy_static;
use std::sync::Mutex;

/// Helper function to convert a URL string to Artwork struct
fn url_to_artwork(url: String) -> BexArtwork {
    BexArtwork {
        url: url.clone(),
        url_low: Some(url.clone()),
        url_high: Some(url),
        layout: ImageLayout::Square,
    }
}

/// Helper function to convert optional URL to optional Artwork
fn opt_url_to_artwork(url: Option<String>) -> Option<BexArtwork> {
    url.map(url_to_artwork)
}

lazy_static! {
    static ref HTTP_CLIENT: Client = Client::new();
    static ref INITIALIZED: std::sync::Once = std::sync::Once::new();
    pub static ref QOBUZ_CLIENT: Mutex<Option<QobuzClient>> = Mutex::new(None);
    pub static ref TIDAL_CLIENT: Mutex<Option<TidalClient>> = Mutex::new(None);
    pub static ref DEEZER_CLIENT: Mutex<Option<DeezerClient>> = Mutex::new(None);
    pub static ref SOUNDCLOUD_CLIENT: Mutex<Option<SoundCloudClient>> = Mutex::new(None);
    pub static ref UNIFIED_CLIENT: Mutex<Option<UnifiedClient>> = Mutex::new(None);
    static ref CLIENTS_INITIALIZED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
}

fn ensure_initialized() {
    INITIALIZED.call_once(|| {
        initialize_api_config();
    });
}

/// Initialize clients asynchronously - call this before using clients
pub async fn ensure_clients_initialized() {
    if CLIENTS_INITIALIZED.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    
    let config = crate::proxy::get_api_config();
    
    // Initialize Qobuz client with default app_id
    // Secrets are fetched dynamically if needed
    if let Some(app_id) = &config.qobuz.app_id {
        let client = QobuzClient::new(app_id.clone(), "default_secret".to_string());
        if let Some(token) = &config.qobuz.password_or_token {
            *QOBUZ_CLIENT.lock().unwrap() = Some(client.with_auth(token.clone()));
        } else {
            *QOBUZ_CLIENT.lock().unwrap() = Some(client);
        }
    }
    
    // Initialize Tidal client with automatic OAuth token fetch
    // This uses hardcoded client credentials from streamrip
    match TidalClient::new_auto().await {
        Ok(client) => {
            *TIDAL_CLIENT.lock().unwrap() = Some(client);
        }
        Err(e) => {
            eprintln!("Failed to initialize Tidal client: {}", e);
        }
    }
    
    // Initialize Deezer client if ARL is provided
    // Without ARL, Deezer will use public API (lower quality)
    if let Some(arl) = &config.deezer.arl {
        let client = DeezerClient::new(arl.clone());
        *DEEZER_CLIENT.lock().unwrap() = Some(client);
    } else {
        // Create client without ARL (will use public API)
        *DEEZER_CLIENT.lock().unwrap() = Some(DeezerClient::new("".to_string()));
    }
    
    // Initialize SoundCloud client with auto credential fetch
    match SoundCloudClient::new_auto().await {
        Ok(client) => {
            *SOUNDCLOUD_CLIENT.lock().unwrap() = Some(client);
        }
        Err(e) => {
            eprintln!("Failed to initialize SoundCloud client: {}", e);
        }
    }
    
    // Initialize Unified Playback API client
    let unified_client = UnifiedClient::new(None, None);
    *UNIFIED_CLIENT.lock().unwrap() = Some(unified_client);
    
    CLIENTS_INITIALIZED.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Fetch with retry across multiple API instances
async fn fetch_with_retry(path: &str, instance_type: &str) -> Result<reqwest::Response, String> {
    ensure_initialized();
    
    let instances = crate::proxy::get_instances(instance_type);
    if instances.is_empty() {
        return Err("No API instances configured".to_string());
    }
    
    let mut last_error = String::new();
    
    for instance in instances {
        let base_url = instance.url.trim_end_matches('/');
        let url = format!("{}{}", base_url, path);
        let proxied_url = apply_proxy_transform(&url);
        
        match HTTP_CLIENT.get(&proxied_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    return Ok(response);
                }
                last_error = format!("HTTP {} from {}", response.status(), base_url);
            }
            Err(e) => {
                last_error = format!("Request failed to {}: {}", base_url, e);
            }
        }
    }
    
    Err(format!("All API instances failed. Last error: {}", last_error))
}

/// Parallel search across all Lossless FLAC APIs with quality ranking
pub async fn search(query: &str, filter: SearchFilter, page: u32) -> Result<PagedMediaItems, String> {
    ensure_initialized();
    ensure_clients_initialized().await;
    
    let limit = 50;
    let mut all_items = Vec::new();
    
    // Spawn parallel searches across all available services
    let qobuz_handle = tokio::spawn(async move {
        let client = QOBUZ_CLIENT.lock().unwrap();
        if let Some(client) = client.as_ref() {
            match filter {
                SearchFilter::Track => client.search_tracks(query, limit).await,
                SearchFilter::Album => client.search_albums(query, limit).await.map(|a| {
                    a.into_iter().map(|album| bex_core::resolver::types::MediaItem::Album(bex_core::resolver::types::AlbumSummary {
                        id: format!("{}:{}", MusicSource::Qobuz.as_str(), album.id),
                        title: album.title,
                        artists: album.artist.iter().map(|a| bex_core::resolver::types::ArtistSummary {
                            id: format!("{}:{}", MusicSource::Qobuz.as_str(), a.id),
                            name: a.name.clone(),
                            thumbnail: opt_url_to_artwork(a.picture.clone()),
                            subtitle: None,
                            url: a.url,
                        }).collect(),
                        thumbnail: opt_url_to_artwork(album.cover.clone()),
                        subtitle: album.release_date.clone(),
                        year: album.release_date.as_ref().and_then(|d| d.split('-').next()).and_then(|y| y.parse().ok()),
                        url: album.url,
                    })).collect()
                }),
                SearchFilter::Artist => client.search_artists(query, limit).await.map(|a| {
                    a.into_iter().map(|artist| bex_core::resolver::types::MediaItem::Artist(bex_core::resolver::types::ArtistSummary {
                        id: format!("{}:{}", MusicSource::Qobuz.as_str(), artist.id),
                        name: artist.name,
                        thumbnail: opt_url_to_artwork(artist.picture.clone()),
                        subtitle: None,
                        url: artist.url,
                    })).collect()
                }),
                _ => Ok(vec![]),
            }
        } else {
            Ok(vec![])
        }
    });
    
    let query_clone = query.to_string();
    let tidal_handle = tokio::spawn(async move {
        let client = TIDAL_CLIENT.lock().unwrap();
        if let Some(client) = client.as_ref() {
            match filter {
                SearchFilter::Track => client.search_tracks(&query_clone, limit).await,
                SearchFilter::Album => client.search_albums(&query_clone, limit).await.map(|a| {
                    a.into_iter().map(|album| bex_core::resolver::types::MediaItem::Album(bex_core::resolver::types::AlbumSummary {
                        id: format!("{}:{}", MusicSource::Tidal.as_str(), album.id),
                        title: album.title,
                        artists: album.artist.iter().map(|a| bex_core::resolver::types::ArtistSummary {
                            id: format!("{}:{}", MusicSource::Tidal.as_str(), a.id),
                            name: a.name.clone(),
                            thumbnail: opt_url_to_artwork(a.picture.clone()),
                            subtitle: None,
                            url: a.url,
                        }).collect(),
                        thumbnail: opt_url_to_artwork(album.cover.clone()),
                        subtitle: album.release_date.clone(),
                        year: album.release_date.as_ref().and_then(|d| d.split('-').next()).and_then(|y| y.parse().ok()),
                        url: album.url,
                    })).collect()
                }),
                SearchFilter::Artist => client.search_artists(&query_clone, limit).await.map(|a| {
                    a.into_iter().map(|artist| bex_core::resolver::types::MediaItem::Artist(bex_core::resolver::types::ArtistSummary {
                        id: format!("{}:{}", MusicSource::Tidal.as_str(), artist.id),
                        name: artist.name,
                        thumbnail: opt_url_to_artwork(artist.picture.clone()),
                        subtitle: None,
                        url: artist.url,
                    })).collect()
                }),
                _ => Ok(vec![]),
            }
        } else {
            Ok(vec![])
        }
    });
    
    let query_clone = query.to_string();
    let deezer_handle = tokio::spawn(async move {
        let client = DEEZER_CLIENT.lock().unwrap();
        if let Some(client) = client.as_ref() {
            match filter {
                SearchFilter::Track => client.search_tracks(&query_clone, limit).await,
                SearchFilter::Album => client.search_albums(&query_clone, limit).await.map(|a| {
                    a.into_iter().map(|album| bex_core::resolver::types::MediaItem::Album(bex_core::resolver::types::AlbumSummary {
                        id: format!("{}:{}", MusicSource::Deezer.as_str(), album.id),
                        title: album.title,
                        artists: album.artist.iter().map(|a| bex_core::resolver::types::ArtistSummary {
                            id: format!("{}:{}", MusicSource::Deezer.as_str(), a.id),
                            name: a.name.clone(),
                            thumbnail: a.picture.clone().map(|p| bex_core::resolver::types::Artwork { url: p }),
                            subtitle: None,
                            url: a.url,
                        }).collect(),
                        thumbnail: opt_url_to_artwork(album.cover.clone()),
                        subtitle: album.release_date.clone(),
                        year: album.release_date.as_ref().and_then(|d| d.split('-').next()).and_then(|y| y.parse().ok()),
                        url: album.url,
                    })).collect()
                }),
                SearchFilter::Artist => client.search_artists(&query_clone, limit).await.map(|a| {
                    a.into_iter().map(|artist| bex_core::resolver::types::MediaItem::Artist(bex_core::resolver::types::ArtistSummary {
                        id: format!("{}:{}", MusicSource::Tidal.as_str(), artist.id),
                        name: artist.name,
                        thumbnail: opt_url_to_artwork(artist.picture.clone()),
                        subtitle: None,
                        url: artist.url,
                    })).collect()
                }),
                _ => Ok(vec![]),
            }
        } else {
            Ok(vec![])
        }
    });
    
    let query_clone = query.to_string();
    let soundcloud_handle = tokio::spawn(async move {
        let client = SOUNDCLOUD_CLIENT.lock().unwrap();
        if let Some(client) = client.as_ref() {
            match filter {
                SearchFilter::Track => client.search_tracks(&query_clone, limit).await,
                SearchFilter::Playlist => client.search_playlists(&query_clone, limit).await.map(|p| {
                    p.into_iter().map(|playlist| bex_core::resolver::types::MediaItem::Playlist(bex_core::resolver::types::PlaylistSummary {
                        id: format!("{}:{}", MusicSource::SoundCloud.as_str(), playlist.id),
                        title: playlist.title,
                        thumbnail: opt_url_to_artwork(playlist.picture.clone()),
                        subtitle: playlist.description,
                        url: playlist.url,
                    })).collect()
                }),
                _ => Ok(vec![]),
            }
        } else {
            Ok(vec![])
        }
    });
    
    // Collect results from all services
    if let Ok(Ok(items)) = qobuz_handle.await {
        all_items.extend(items);
    }

    if let Ok(Ok(items)) = tidal_handle.await {
        all_items.extend(items);
    }

    if let Ok(Ok(items)) = deezer_handle.await {
        all_items.extend(items);
    }

    if let Ok(Ok(items)) = soundcloud_handle.await {
        all_items.extend(items);
    }

    // Deduplicate by ISRC if available (or title+artist combination)
    let deduplicated = deduplicate_items(all_items);

    // Sort by quality priority (highest quality first)
    deduplicated.sort_by(|a, b| {
        let quality_a = get_item_quality_priority(a);
        let quality_b = get_item_quality_priority(b);
        quality_b.cmp(&quality_a) // Descending order
    });

    let total = deduplicated.len() as u32;
    let offset = (page - 1) * limit;

    // Apply pagination
    let items: Vec<MediaItem> = deduplicated.into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect();

    Ok(PagedMediaItems {
        items,
        next_page_token: if offset + limit < total { Some((page + 1).to_string()) } else { None },
    })
}

/// Advanced search with quality filtering
pub async fn advanced_search(
    query: &str,
    filter: SearchFilter,
    page: u32,
    _min_quality: Option<Quality>,
    _max_quality: Option<Quality>,
) -> Result<PagedMediaItems, String> {
    // For now, just call regular search - quality filtering can be added later
    search(query, filter, page).await
}

/// Find best quality version of a track across all services
/// Implements Lossless FLAC quality hierarchy with Dolby Atmos priority
pub async fn find_best_quality_track(query: &str, track_name: &str, artist_name: &str) -> Result<MediaItem, String> {
    ensure_clients_initialized().await;
    
    let search_query = format!("{} {}", track_name, artist_name);
    let results = search(&search_query, SearchFilter::Track, 1).await?;
    
    // Find the best quality match with enhanced quality hierarchy
    results.items.into_iter()
        .filter(|item| {
            if let MediaItem::Track(track) = item {
                // Check if title and artist match
                let title_match = track.title.to_lowercase().contains(&track_name.to_lowercase());
                let artist_match = track.artists.first()
                    .map(|a| a.name.to_lowercase().contains(&artist_name.to_lowercase()))
                    .unwrap_or(false);
                title_match && artist_match
            } else {
                false
            }
        })
        .max_by(|a, b| {
            let priority_a = get_item_quality_priority(a);
            let priority_b = get_item_quality_priority(b);
            priority_a.cmp(&priority_b)
        })
        .ok_or_else(|| "No matching track found".to_string())
}

/// Get quality priority for an item (higher = better quality)
/// Implements Lossless FLAC hierarchy with Dolby Atmos priority
fn get_item_quality_priority(item: &MediaItem) -> u8 {
    match item {
        MediaItem::Track(track) => {
            // Extract source from ID prefix for priority
            let source = track.id.split(':').next()
                .and_then(|s| MusicSource::from_str(s))
                .unwrap_or(MusicSource::Tidal);

            // Default priority based on source
            match source {
                MusicSource::Qobuz => 6, // Qobuz has best quality
                MusicSource::Tidal => 5,
                MusicSource::Deezer => 4,
                MusicSource::SoundCloud => 3,
                _ => 4,
            }
        }
        MediaItem::Album(album) => {
            // Extract source from ID prefix for priority
            let source = album.id.split(':').next()
                .and_then(|s| MusicSource::from_str(s))
                .unwrap_or(MusicSource::Tidal);

            // Albums sorted by source priority
            match source {
                MusicSource::Qobuz => 6,
                MusicSource::Tidal => 5,
                MusicSource::Deezer => 4,
                MusicSource::SoundCloud => 3,
                _ => 4,
            }
        }
        _ => 0,
    }
}

/// Deduplicate items by ISRC or title+artist combination
fn deduplicate_items(items: Vec<MediaItem>) -> Vec<MediaItem> {
    use std::collections::HashMap;

    let mut seen: HashMap<String, MediaItem> = HashMap::new();

    for item in items {
        let key = match &item {
            MediaItem::Track(track) => {
                // Use title+first artist name for deduplication
                let artist_name = track.artists.first()
                    .map(|a| a.name.clone())
                    .unwrap_or_default();
                format!("track_{}_{}", track.title, artist_name)
            }
            MediaItem::Album(album) => {
                // Use title+first artist name for deduplication
                let artist_name = album.artists.first()
                    .map(|a| a.name.clone())
                    .unwrap_or_default();
                format!("album_{}_{}", album.title, artist_name)
            }
            MediaItem::Artist(artist) => {
                format!("artist_{}", artist.name)
            }
            MediaItem::Playlist(playlist) => {
                format!("playlist_{}", playlist.title)
            }
        };
        
        // Only keep if not seen, or if new item has higher quality
        if !seen.contains_key(&key) {
            seen.insert(key, item);
        } else {
            // Compare quality and keep the better one
            let existing_quality = get_item_quality_priority(seen.get(&key).unwrap());
            let new_quality = get_item_quality_priority(&item);
            
            if new_quality > existing_quality {
                seen.insert(key, item);
            }
        }
    }
    
    seen.into_values().collect()
}

/// Get detailed track information from appropriate service
pub async fn get_track_details(id: &str) -> Result<BexTrack, String> {
    ensure_initialized();
    ensure_clients_initialized().await;
    
    let (source, actual_id) = parse_source_id(id)?;
    
    let unified_track = match source {
        MusicSource::Qobuz => {
            let client = QOBUZ_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                client.get_track(&actual_id).await?
            } else {
                return Err("Qobuz client not available".to_string());
            }
        }
        MusicSource::Tidal => {
            let client = TIDAL_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                client.get_track(&actual_id).await?
            } else {
                return Err("Tidal client not available".to_string());
            }
        }
        MusicSource::Deezer => {
            let client = DEEZER_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                client.get_track(&actual_id).await?
            } else {
                return Err("Deezer client not available".to_string());
            }
        }
        MusicSource::SoundCloud | MusicSource::Amazon | MusicSource::UnifiedPlayback => {
            return Err("Track fetching not supported for this source".to_string());
        }
    };
    
    Ok(map_track(unified_track))
}

/// Get detailed album information from appropriate service
pub async fn get_album_details(id: &str) -> Result<AlbumDetails, String> {
    ensure_initialized();
    ensure_clients_initialized().await;
    
    let (source, actual_id) = parse_source_id(id)?;
    
    let unified_album = match source {
        MusicSource::Qobuz => {
            let client = QOBUZ_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                client.get_album(&actual_id).await?
            } else {
                return Err("Qobuz client not available".to_string());
            }
        }
        MusicSource::Tidal => {
            let client = TIDAL_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                client.get_album(&actual_id).await?
            } else {
                return Err("Tidal client not available".to_string());
            }
        }
        MusicSource::Deezer => {
            let client = DEEZER_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                client.get_album(&actual_id).await?
            } else {
                return Err("Deezer client not available".to_string());
            }
        }
        MusicSource::SoundCloud | MusicSource::Amazon | MusicSource::UnifiedPlayback => {
            return Err("Album fetching not supported for this source".to_string());
        }
    };
    
    Ok(AlbumDetails {
        album: map_album(unified_album.clone()),
        description: Some(format!("Source: {}", source.as_str())),
        tracks: None, // Will be loaded separately
        total_tracks: unified_album.track_count.unwrap_or(0) as i32,
        release_date: unified_album.release_date,
        duration: unified_album.duration.map(|d| d as i64),
    })
}

/// Get detailed artist information from appropriate service
pub async fn get_artist_details(id: &str) -> Result<ArtistDetails, String> {
    ensure_initialized();
    ensure_clients_initialized().await;
    
    let (source, actual_id) = parse_source_id(id)?;
    
    // For now, return a basic artist details since individual artist fetching
    // varies significantly between services
    let unified_artist = match source {
        MusicSource::Qobuz => {
            let client = QOBUZ_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                // Search for the artist by ID to get details
                let results = client.search_artists(&actual_id, 1).await?;
                results.into_iter().next().ok_or("Artist not found")?
            } else {
                return Err("Qobuz client not available".to_string());
            }
        }
        MusicSource::Tidal => {
            let client = TIDAL_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                let results = client.search_artists(&actual_id, 1).await?;
                results.into_iter().next().ok_or("Artist not found")?
            } else {
                return Err("Tidal client not available".to_string());
            }
        }
        MusicSource::Deezer => {
            let client = DEEZER_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                let results = client.search_artists(&actual_id, 1).await?;
                results.into_iter().next().ok_or("Artist not found")?
            } else {
                return Err("Deezer client not available".to_string());
            }
        }
        MusicSource::SoundCloud | MusicSource::Amazon | MusicSource::UnifiedPlayback => {
            return Err("Artist fetching not supported for this source".to_string());
        }
    };
    
    Ok(ArtistDetails {
        summary: bex_core::resolver::types::ArtistSummary {
            id: format!("{}:{}", source.as_str(), unified_artist.id),
            name: unified_artist.name,
            thumbnail: opt_url_to_artwork(unified_artist.picture),
            subtitle: None,
            url: unified_artist.url,
        },
        description: Some(format!("Source: {}", source.as_str())),
        top_tracks: vec![],
        albums: PagedAlbums {
            items: vec![],
            next_page_token: None,
        },
        related_artists: vec![],
    })
}

/// Get more albums from an artist
pub async fn more_artist_albums(id: &str, page_token: &str) -> Result<PagedAlbums, String> {
    ensure_initialized();
    ensure_clients_initialized().await;
    
    let (source, actual_id) = parse_source_id(id)?;
    let limit = 50;
    let offset: u32 = page_token.parse().unwrap_or(0);
    
    // For now, search for albums by artist name
    let albums = match source {
        MusicSource::Qobuz => {
            let client = QOBUZ_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                client.search_albums(&actual_id, limit).await?
            } else {
                return Err("Qobuz client not available".to_string());
            }
        }
        MusicSource::Tidal => {
            let client = TIDAL_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                client.search_albums(&actual_id, limit).await?
            } else {
                return Err("Tidal client not available".to_string());
            }
        }
        MusicSource::Deezer => {
            let client = DEEZER_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                client.search_albums(&actual_id, limit).await?
            } else {
                return Err("Deezer client not available".to_string());
            }
        }
        MusicSource::SoundCloud | MusicSource::Amazon | MusicSource::UnifiedPlayback => {
            return Err("Album search not supported for this source".to_string());
        }
    };
    
    let total = albums.len() as u32;

    Ok(PagedAlbums {
        items: albums.into_iter().map(|album| bex_core::resolver::types::AlbumSummary {
            id: format!("{}:{}", source.as_str(), album.id),
            title: album.title,
            artists: album.artist.iter().map(|a| bex_core::resolver::types::ArtistSummary {
                id: format!("{}:{}", source.as_str(), a.id),
                name: a.name.clone(),
                thumbnail: opt_url_to_artwork(a.picture.clone()),
                subtitle: None,
                url: a.url,
            }).collect(),
            thumbnail: opt_url_to_artwork(album.cover.clone()),
            subtitle: album.release_date.clone(),
            year: album.release_date.as_ref().and_then(|d| d.split('-').next()).and_then(|y| y.parse().ok()),
            url: album.url,
        }).collect(),
        next_page_token: if offset + limit < total { Some((offset + limit).to_string()) } else { None },
    })
}

/// Get playlist details (not fully implemented for all services)
pub async fn get_playlist_details(id: &str) -> Result<PlaylistDetails, String> {
    // Playlist support varies significantly between services
    // For now, return an error
    Err("Playlist support not yet implemented".to_string())
}

/// Get more tracks from an album
pub async fn more_album_tracks(id: &str, page_token: &str) -> Result<PagedTracks, String> {
    ensure_initialized();
    ensure_clients_initialized().await;
    
    let (source, actual_id) = parse_source_id(id)?;
    let limit = 50;
    let offset: u32 = page_token.parse().unwrap_or(0);
    
    // For now, search for tracks by album name
    let tracks = match source {
        MusicSource::Qobuz => {
            let client = QOBUZ_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                client.search_tracks(&actual_id, limit).await?
            } else {
                return Err("Qobuz client not available".to_string());
            }
        }
        MusicSource::Tidal => {
            let client = TIDAL_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                client.search_tracks(&actual_id, limit).await?
            } else {
                return Err("Tidal client not available".to_string());
            }
        }
        MusicSource::Deezer => {
            let client = DEEZER_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                client.search_tracks(&actual_id, limit).await?
            } else {
                return Err("Deezer client not available".to_string());
            }
        }
        _ => return Err("Unsupported source for album tracks".to_string()),
    };

    let total = tracks.len() as u32;

    Ok(PagedTracks {
        items: tracks.into_iter().map(map_track).collect(),
        next_page_token: if offset + limit < total { Some((offset + limit).to_string()) } else { None },
    })
}

/// Get more tracks from a playlist
pub async fn more_playlist_tracks(id: &str, page_token: &str) -> Result<PagedTracks, String> {
    // Playlist support varies significantly between services
    Err("Playlist support not yet implemented".to_string())
}

/// Get radio tracks based on a reference
pub async fn get_radio_tracks(reference_id: &str, page_token: Option<&str>) -> Result<PagedTracks, String> {
    // Radio not yet implemented
    Ok(PagedTracks {
        items: vec![],
        next_page_token: None,
    })
}

/// Get stream source for a track
pub async fn get_stream_source(track_id: &str, quality: &str) -> Result<Vec<StreamSource>, String> {
    ensure_initialized();
    ensure_clients_initialized().await;

    let (source, actual_id) = parse_source_id(track_id)?;

    let stream_info = match source {
        MusicSource::Qobuz => {
            let client = QOBUZ_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                // Try different quality levels
                let quality_num = match quality {
                    "LOSSLESS" => 3,
                    "HI_RES" => 4,
                    "ULTRA_HI_RES" => 5,
                    _ => 3,
                };
                if let Ok(stream) = client.get_stream_url(&actual_id, quality_num).await {
                    stream
                } else {
                    return Err("Failed to get stream from Qobuz".to_string());
                }
            } else {
                return Err("Qobuz client not available".to_string());
            }
        }
        MusicSource::Tidal => {
            let client = TIDAL_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                let quality_num = match quality {
                    "LOSSLESS" => 3,
                    "HI_RES" => 4,
                    "ULTRA_HI_RES" => 5,
                    _ => 3,
                };
                if let Ok(stream) = client.get_stream_url(&actual_id, quality_num).await {
                    stream
                } else {
                    return Err("Failed to get stream from Tidal".to_string());
                }
            } else {
                return Err("Tidal client not available".to_string());
            }
        }
        MusicSource::Deezer => {
            let client = DEEZER_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                let quality_num = match quality {
                    "LOSSLESS" => 3,
                    "HI_RES" => 4,
                    _ => 3,
                };
                if let Ok(stream) = client.get_stream_url(&actual_id, quality_num).await {
                    stream
                } else {
                    return Err("Failed to get stream from Deezer".to_string());
                }
            } else {
                return Err("Deezer client not available".to_string());
            }
        }
        MusicSource::SoundCloud => {
            let client = SOUNDCLOUD_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                if let Ok(stream) = client.get_stream_url(&actual_id, 0).await {
                    stream
                } else {
                    return Err("Failed to get stream from SoundCloud".to_string());
                }
            } else {
                return Err("SoundCloud client not available".to_string());
            }
        }
        MusicSource::Amazon | MusicSource::UnifiedPlayback => {
            return Err("Stream not supported for this source".to_string());
        }
    };

    Ok(vec![convert_to_stream_source(stream_info)])
}

/// Convert StreamInfo to bex_core StreamSource
fn convert_to_stream_source(info: StreamInfo) -> StreamSource {
    use bex_core::resolver::data_source::Quality as BexQuality;

    let quality = match info.quality {
        Quality::Low => BexQuality::Low,
        Quality::Normal => BexQuality::Medium,
        Quality::High => BexQuality::High,
        Quality::LosslessFlac => BexQuality::Lossless,
        Quality::HiRes => BexQuality::Lossless,
        Quality::UltraHiRes => BexQuality::Lossless,
        Quality::DolbyAtmos => BexQuality::Lossless,
    };

    StreamSource {
        url: info.url,
        quality,
        format: info.codec,
        headers: None,
        expires_at: None,
    }
}

/// Get album details
pub async fn get_album_details(id: &str) -> Result<AlbumDetails, String> {
    ensure_initialized();
    ensure_clients_initialized().await;

    let (source, actual_id) = parse_source_id(id)?;

    let unified_album = match source {
        MusicSource::Qobuz => {
            let client = QOBUZ_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                client.get_album(&actual_id).await?
            } else {
                return Err("Qobuz client not available".to_string());
            }
        }
        MusicSource::Tidal => {
            let client = TIDAL_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                client.get_album(&actual_id).await?
            } else {
                return Err("Tidal client not available".to_string());
            }
        }
        MusicSource::Deezer => {
            let client = DEEZER_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                client.get_album(&actual_id).await?
            } else {
                return Err("Deezer client not available".to_string());
            }
        }
        _ => return Err("Unsupported source".to_string()),
    };

    Ok(AlbumDetails {
        summary: bex_core::resolver::types::AlbumSummary {
            id: format!("{}:{}", source.as_str(), unified_album.id),
            title: unified_album.title,
            artists: unified_album.artist.iter().map(|a| bex_core::resolver::types::ArtistSummary {
                id: format!("{}:{}", source.as_str(), a.id),
                name: a.name.clone(),
                thumbnail: opt_url_to_artwork(a.picture.clone()),
                subtitle: None,
                url: a.url,
            }).collect(),
            thumbnail: opt_url_to_artwork(unified_album.cover.clone()),
            subtitle: unified_album.release_date.clone(),
            year: unified_album.release_date.as_ref().and_then(|d| d.split('-').next()).and_then(|y| y.parse().ok()),
            url: unified_album.url,
        },
        tracks: PagedTracks {
            items: vec![],
            next_page_token: None,
        },
        description: None,
    })
}

/// Get track details
pub async fn get_track_details(id: &str) -> Result<bex_core::resolver::types::Track, String> {
    ensure_initialized();
    ensure_clients_initialized().await;

    let (source, actual_id) = parse_source_id(id)?;

    // For now, return a basic track structure
    // Full implementation would fetch from the appropriate service
    Ok(bex_core::resolver::types::Track {
        id: id.to_string(),
        title: "Track".to_string(),
        artists: vec![],
        album: None,
        duration_ms: Some(0),
        thumbnail: url_to_artwork(String::new()),
        url: None,
        lyrics: None,
        is_explicit: false,
    })
}

/// Find best quality track across all services
pub async fn find_best_quality_track(query: &str, artist: &str) -> Result<UnifiedTrack, String> {
    ensure_initialized();
    ensure_clients_initialized().await;

    let search_query = format!("{} {}", query, artist);
    let results = search(&search_query, SearchFilter::Track, 1).await?;

    // Return the first track (highest quality due to sorting)
    results.items.into_iter()
        .find_map(|item| {
            if let MediaItem::Track(track) = item {
                // Convert back to UnifiedTrack
                let (source, actual_id) = parse_source_id(&track.id).ok()?;
                Some(UnifiedTrack {
                    id: actual_id,
                    title: track.title,
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
                        source,
                    }),
                    artists: Some(track.artists.iter().map(|a| UnifiedArtist {
                        id: a.id.clone(),
                        name: a.name.clone(),
                        picture: a.thumbnail.as_ref().map(|t| t.url.clone()),
                        url: a.url.clone(),
                        source,
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
                        source,
                    }),
                    source,
                    qualities_available: vec![Quality::LosslessFlac, Quality::HiRes],
                })
            } else {
                None
            }
        })
        .ok_or_else(|| "No tracks found".to_string())
}
        limit: 50,
        offset: page_token.parse().unwrap_or(0),
        total: 0,
        next_page_token: None,
    })
}

/// Get radio/recommendation tracks
pub async fn get_radio_tracks(reference_id: &str, page_token: Option<&str>) -> Result<PagedTracks, String> {
    ensure_initialized();
    ensure_clients_initialized().await;
    
    let (source, actual_id) = parse_source_id(reference_id)?;
    let limit = 50;
    let offset: u32 = page_token.and_then(|p| p.parse().ok()).unwrap_or(0);
    
    // Use search as a fallback for radio recommendations
    let tracks = match source {
        MusicSource::Qobuz => {
            let client = QOBUZ_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                client.search_tracks(actual_id, limit).await?
            } else {
                return Err("Qobuz client not available".to_string());
            }
        }
        MusicSource::Tidal => {
            let client = TIDAL_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                client.search_tracks(actual_id, limit).await?
            } else {
                return Err("Tidal client not available".to_string());
            }
        }
        MusicSource::Deezer => {
            let client = DEEZER_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                client.search_tracks(actual_id, limit).await?
            } else {
                return Err("Deezer client not available".to_string());
            }
        }
    };
    
    let total = tracks.len() as u32;
    
    Ok(PagedTracks {
        items: tracks.into_iter().map(map_track).collect(),
        limit,
        offset,
        total,
        next_page_token: if offset + limit < total { Some((offset + limit).to_string()) } else { None },
    })
}

/// Get stream source for a track at highest available quality
/// Implements Lossless FLAC quality hierarchy: Dolby Atmos > Ultra Hi-Res > Hi-Res > Lossless > High > Low
pub async fn get_stream_source(track_id: &str, _quality: &str) -> Result<Vec<StreamSource>, String> {
    ensure_initialized();
    ensure_clients_initialized().await;
    
    // Parse source from track ID
    let (source, actual_id) = parse_source_id(track_id)?;
    
    // Try to get the highest quality stream from the appropriate service
    // Quality hierarchy: 5 (Dolby Atmos) > 4 (Ultra Hi-Res) > 3 (Hi-Res) > 2 (Lossless) > 1 (High) > 0 (Low)
    let stream_info = match source {
        MusicSource::Qobuz => {
            let client = QOBUZ_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                // Try highest quality first (4 for Qobuz Ultra Hi-Res), then fallback
                for quality in (0..=4).rev() {
                    if let Ok(stream) = client.get_stream_url(actual_id, quality).await {
                        return Ok(vec![convert_to_stream_source(stream)]);
                    }
                }
            }
            return Err("Qobuz client not available".to_string());
        }
        MusicSource::Tidal => {
            let client = TIDAL_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                // Try Dolby Atmos first (5), then Ultra Hi-Res (4), then fallback
                for quality in (0..=5).rev() {
                    if let Ok(stream) = client.get_stream_url(actual_id, quality).await {
                        return Ok(vec![convert_to_stream_source(stream)]);
                    }
                }
            }
            return Err("Tidal client not available".to_string());
        }
        MusicSource::Deezer => {
            let client = DEEZER_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                // Try highest quality first (2 for Deezer FLAC), then fallback
                for quality in (0..=2).rev() {
                    if let Ok(stream) = client.get_stream_url(actual_id, quality).await {
                        return Ok(vec![convert_to_stream_source(stream)]);
                    }
                }
            }
            return Err("Deezer client not available".to_string());
        }
        MusicSource::SoundCloud => {
            let client = SOUNDCLOUD_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                // SoundCloud only has quality 0
                if let Ok(stream) = client.get_stream_url(actual_id, 0).await {
                    return Ok(vec![convert_to_stream_source(stream)]);
                }
            }
            return Err("SoundCloud client not available".to_string());
        }
        MusicSource::UnifiedPlayback => {
            let client = UNIFIED_CLIENT.lock().unwrap();
            if let Some(client) = client.as_ref() {
                // Try different quality levels with Lossless FLAC hierarchy
                for quality in [Quality::DolbyAtmos, Quality::UltraHiRes, Quality::HiRes, Quality::High, Quality::Normal].iter() {
                    if let Ok(stream) = client.get_stream_url(actual_id, *quality).await {
                        return Ok(vec![convert_to_stream_source(stream)]);
                    }
                }
            }
            return Err("Unified client not available".to_string());
        }
        _ => return Err("Unknown music source".to_string()),
    };
    
    Ok(vec![convert_to_stream_source(stream_info)])
}

/// Convert StreamInfo to StreamSource
fn convert_to_stream_source(info: StreamInfo) -> StreamSource {
    StreamSource {
        source_id: info.track_id.clone(),
        url: info.url,
        mime_type: format!("audio/{}", info.codec),
        bitrate: info.bitrate as i32,
        format: info.codec,
        is_encrypted: info.encryption_key.is_some(),
    }
}