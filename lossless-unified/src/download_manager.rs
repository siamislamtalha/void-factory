use crate::types::*;
use crate::client::{ensure_clients_initialized, QOBUZ_CLIENT, TIDAL_CLIENT, DEEZER_CLIENT, SOUNDCLOUD_CLIENT};
use crate::decryption::{decrypt_tidal_mqa, decrypt_deezer_blowfish};
use reqwest::Client;
use std::sync::Mutex;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::path::Path;
use std::fs;

lazy_static! {
    static ref HTTP_CLIENT: Client = Client::new();
    static ref DOWNLOAD_QUEUE: Mutex<Vec<DownloadRequest>> = Mutex::new(vec![]);
    static ref ACTIVE_DOWNLOADS: Mutex<HashMap<String, DownloadProgress>> = Mutex::new(HashMap::new());
    static ref DOWNLOAD_COUNTER: AtomicU32 = AtomicU32::new(0);
}

/// Download manager for high-quality audio downloads
pub struct DownloadManager {
    max_concurrent: u32,
    download_folder: String,
}

impl DownloadManager {
    pub fn new(max_concurrent: u32, download_folder: String) -> Self {
        Self {
            max_concurrent,
            download_folder,
        }
    }

    /// Add a download request to the queue
    pub fn queue_download(&self, request: DownloadRequest) -> Result<String, String> {
        let download_id = format!("dl_{}", DOWNLOAD_COUNTER.fetch_add(1, Ordering::SeqCst));
        
        // Initialize progress
        let progress = DownloadProgress {
            track_id: request.track_id.clone(),
            progress: 0.0,
            status: DownloadStatus::Queued,
            speed: None,
            eta: None,
        };
        
        ACTIVE_DOWNLOADS.lock().unwrap().insert(download_id.clone(), progress);
        DOWNLOAD_QUEUE.lock().unwrap().push(request);
        
        Ok(download_id)
    }

    /// Start processing the download queue
    pub async fn process_queue(&self) -> Result<(), String> {
        ensure_clients_initialized().await;
        
        let queue = DOWNLOAD_QUEUE.lock().unwrap().clone();
        DOWNLOAD_QUEUE.lock().unwrap().clear();
        
        let mut handles = Vec::new();
        
        for request in queue {
            let manager = self.clone();
            let handle = tokio::spawn(async move {
                manager.process_download(request).await
            });
            handles.push(handle);
        }
        
        // Wait for all downloads to complete
        for handle in handles {
            if let Ok(result) = handle.await {
                if let Err(e) = result {
                    eprintln!("Download failed: {}", e);
                }
            }
        }
        
        Ok(())
    }

    /// Process a single download request
    async fn process_download(&self, request: DownloadRequest) -> Result<(), String> {
        let download_id = format!("dl_{}", request.track_id);
        
        // Update status to downloading
        self.update_progress(&download_id, DownloadStatus::Downloading, 0.0, None, None);
        
        // Parse source from track ID
        let (source, actual_id) = if request.track_id.contains(':') {
            let parts: Vec<&str> = request.track_id.splitn(2, ':').collect();
            (MusicSource::from_str(parts[0]), parts.get(1).map(|s| s.to_string()))
        } else {
            (Some(MusicSource::Tidal), Some(request.track_id.clone()))
        };
        
        let (source, actual_id) = match (source, actual_id) {
            (Some(s), Some(id)) => (s, id),
            _ => return Err("Invalid track ID format".to_string()),
        };
        
        // Get stream URL at requested quality
        let stream_info = match source {
            MusicSource::Qobuz => {
                let client = QOBUZ_CLIENT.lock().unwrap();
                if let Some(client) = client.as_ref() {
                    client.get_stream_url(&actual_id, request.quality).await?
                } else {
                    return Err("Qobuz client not available".to_string());
                }
            }
            MusicSource::Tidal => {
                let client = TIDAL_CLIENT.lock().unwrap();
                if let Some(client) = client.as_ref() {
                    client.get_stream_url(&actual_id, request.quality).await?
                } else {
                    return Err("Tidal client not available".to_string());
                }
            }
            MusicSource::Deezer => {
                let client = DEEZER_CLIENT.lock().unwrap();
                if let Some(client) = client.as_ref() {
                    client.get_stream_url(&actual_id, request.quality).await?
                } else {
                    return Err("Deezer client not available".to_string());
                }
            }
            MusicSource::SoundCloud => {
                let client = SOUNDCLOUD_CLIENT.lock().unwrap();
                if let Some(client) = client.as_ref() {
                    client.get_stream_url(&actual_id, request.quality).await?
                } else {
                    return Err("SoundCloud client not available".to_string());
                }
            }
            _ => return Err("Unsupported source for download".to_string()),
        };
        
        // Update progress to decrypting if needed
        if stream_info.encryption_key.is_some() {
            self.update_progress(&download_id, DownloadStatus::Decrypting, 0.3, None, None);
        }
        
        // Download the audio data
        let audio_data = self.download_audio(&stream_info.url, &download_id).await?;
        
        // Decrypt if necessary
        let final_data = if let Some(encryption_key) = &stream_info.encryption_key {
            self.update_progress(&download_id, DownloadStatus::Decrypting, 0.5, None, None);
            
            match source {
                MusicSource::Tidal => decrypt_tidal_mqa(&audio_data, encryption_key)?,
                MusicSource::Deezer => decrypt_deezer_blowfish(&audio_data, &actual_id)?,
                _ => audio_data,
            }
        } else {
            audio_data
        };
        
        // Convert format if needed
        let final_data = if matches!(request.format, DownloadFormat::MP3 | DownloadFormat::AAC) {
            self.update_progress(&download_id, DownloadStatus::Converting, 0.8, None, None);
            self.convert_audio(&final_data, &request.format).await?
        } else {
            final_data
        };
        
        // Save to file
        let filename = self.generate_filename(&request.track_id, &request.format);
        let filepath = format!("{}/{}", self.download_folder, filename);
        self.save_file(&filepath, &final_data).await?;
        
        // Update to completed
        self.update_progress(&download_id, DownloadStatus::Completed, 1.0, None, None);
        
        Ok(())
    }

    /// Download audio data from URL with progress tracking (streamrip-style fast download)
    async fn download_audio(&self, url: &str, download_id: &str) -> Result<Vec<u8>, String> {
        // Streamrip uses fast_async_download with 131KB chunks and yields every 1MB
        // This prevents CPU-bound issues with async downloads
        let chunk_size: usize = 2usize.pow(17); // 131 KB
        let yield_every: usize = 8; // 1 MB (8 * 131KB)
        
        let response = HTTP_CLIENT
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Download request failed: {}", e))?;
        
        if !response.status().is_success() {
            return Err(format!("Download HTTP error: {}", response.status()));
        }
        
        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded = 0u64;
        let mut data = Vec::new();
        let mut counter = 0usize;
        
        let start_time = std::time::Instant::now();
        
        // Stream using bytes API for better performance
        let mut stream = response.bytes_stream();
        
        use futures_util::StreamExt;
        
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| format!("Stream error: {}", e))?;
            data.extend_from_slice(&chunk);
            downloaded += chunk.len() as u64;
            counter += 1;
            
            // Update progress
            let progress = if total_size > 0 {
                downloaded as f64 / total_size as f64
            } else {
                0.0
            };
            
            // Calculate speed and ETA
            let elapsed = start_time.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 {
                Some(downloaded as f64 / elapsed)
            } else {
                None
            };
            
            let eta = if let Some(s) = speed {
                if total_size > 0 && s > 0.0 {
                    Some((total_size - downloaded) as f64 / s)
                } else {
                    None
                }
            } else {
                None
            };
            
            self.update_progress(download_id, DownloadStatus::Downloading, progress, speed, eta);
            
            // Yield to event loop every 1MB (streamrip optimization)
            if counter % yield_every == 0 {
                tokio::task::yield_now().await;
            }
        }
        
        Ok(data)
    }

    /// Convert audio to different format (placeholder - would need ffmpeg integration)
    async fn convert_audio(&self, data: &[u8], format: &DownloadFormat) -> Result<Vec<u8>, String> {
        // In a real implementation, this would use ffmpeg or similar
        // For now, return the data as-is
        match format {
            DownloadFormat::FLAC => Ok(data.to_vec()),
            DownloadFormat::ALAC => Ok(data.to_vec()), // Would need conversion
            DownloadFormat::MP3 => Ok(data.to_vec()), // Would need conversion
            DownloadFormat::AAC => Ok(data.to_vec()), // Would need conversion
            DownloadFormat::OPUS => Ok(data.to_vec()), // Would need conversion
        }
    }
    
    /// Auto-download best quality version of a track
    pub async fn auto_download_best_quality(track_name: &str, artist_name: &str) -> Result<String, String> {
        // This would integrate with the client to find and download the best quality
        // For now, this is a placeholder
        Ok("download_id_placeholder".to_string())
    }

    /// Save data to file (no-op on wasm)
    async fn save_file(&self, filepath: &str, data: &[u8]) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            tokio::fs::write(filepath, data)
                .await
                .map_err(|e| format!("Failed to save file: {}", e))?;
            Ok(())
        }
        #[cfg(target_arch = "wasm32")]
        {
            // File operations not supported in wasm/browser environment
            Err("File operations not supported in wasm environment".to_string())
        }
    }

    /// Generate filename for download
    fn generate_filename(&self, track_id: &str, format: &DownloadFormat) -> String {
        let extension = match format {
            DownloadFormat::FLAC => "flac",
            DownloadFormat::ALAC => "m4a",
            DownloadFormat::MP3 => "mp3",
            DownloadFormat::AAC => "m4a",
            DownloadFormat::OPUS => "opus",
        };
        
        format!("{}.{}", track_id.replace(':', "_"), extension)
    }

    /// Update download progress
    fn update_progress(&self, download_id: &str, status: DownloadStatus, progress: f32, speed: Option<u64>, eta: Option<u64>) {
        let mut downloads = ACTIVE_DOWNLOADS.lock().unwrap();
        if let Some(dl) = downloads.get_mut(download_id) {
            dl.status = status;
            dl.progress = progress;
            dl.speed = speed;
            dl.eta = eta;
        }
    }

    /// Get download progress
    pub fn get_progress(&self, download_id: &str) -> Option<DownloadProgress> {
        ACTIVE_DOWNLOADS.lock().unwrap().get(download_id).cloned()
    }

    /// Get all active downloads
    pub fn get_all_downloads(&self) -> Vec<DownloadProgress> {
        ACTIVE_DOWNLOADS.lock().unwrap().values().cloned().collect()
    }

    /// Cancel a download
    pub fn cancel_download(&self, download_id: &str) -> Result<(), String> {
        let mut downloads = ACTIVE_DOWNLOADS.lock().unwrap();
        if let Some(dl) = downloads.get_mut(download_id) {
            dl.status = DownloadStatus::Failed("Cancelled".to_string());
            Ok(())
        } else {
            Err("Download not found".to_string())
        }
    }
}

impl Clone for DownloadManager {
    fn clone(&self) -> Self {
        Self {
            max_concurrent: self.max_concurrent,
            download_folder: self.download_folder.clone(),
        }
    }
}

/// Auto-download best quality version of a track (Lossless FLAC style)
pub async fn auto_download_best_quality(track_id: &str, download_folder: &str) -> Result<String, String> {
    ensure_clients_initialized().await;
    
    // Try to get the best available quality for this track (Lossless FLAC hierarchy)
    let qualities = vec![
        Quality::DolbyAtmos,      // Highest priority - Dolby Atmos
        Quality::UltraHiRes,     // 24-bit, ≤192 kHz
        Quality::HiRes,          // 24-bit, ≤96 kHz  
        Quality::High,           // 16-bit, 44.1 kHz (CD quality)
        Quality::Normal,         // 320 kbps
        Quality::Low,            // 128 kbps
    ];
    
    // Parse source from track ID
    let (source, actual_id) = if track_id.contains(':') {
        let parts: Vec<&str> = track_id.splitn(2, ':').collect();
        (MusicSource::from_str(parts[0]), parts.get(1).map(|s| s.to_string()))
    } else {
        (Some(MusicSource::Tidal), Some(track_id.to_string()))
    };
    
    let (source, actual_id) = match (source, actual_id) {
        (Some(s), Some(id)) => (s, id),
        _ => return Err("Invalid track ID format".to_string()),
    };
    
    // Try each quality level until we find one that works (streamrip approach)
    for quality in qualities {
        // First check if this quality is available from the source
        let stream_info = match source {
            MusicSource::Qobuz => {
                let client = QOBUZ_CLIENT.lock().unwrap();
                if let Some(client) = client.as_ref() {
                    client.get_stream_url(&actual_id, quality.as_number()).await.ok()
                } else {
                    None
                }
            }
            MusicSource::Tidal => {
                let client = TIDAL_CLIENT.lock().unwrap();
                if let Some(client) = client.as_ref() {
                    client.get_stream_url(&actual_id, quality.as_number()).await.ok()
                } else {
                    None
                }
            }
            MusicSource::Deezer => {
                let client = DEEZER_CLIENT.lock().unwrap();
                if let Some(client) = client.as_ref() {
                    client.get_stream_url(&actual_id, quality.as_number()).await.ok()
                } else {
                    None
                }
            }
            MusicSource::SoundCloud => {
                let client = SOUNDCLOUD_CLIENT.lock().unwrap();
                if let Some(client) = client.as_ref() {
                    client.get_stream_url(&actual_id, quality.as_number()).await.ok()
                } else {
                    None
                }
            }
            _ => None,
        };
        
        if stream_info.is_some() {
            // This quality is available, proceed with download
            let request = DownloadRequest {
                track_id: track_id.to_string(),
                quality,
                format: DownloadFormat::FLAC, // Always prefer FLAC for highest quality
                include_metadata: true,
                include_artwork: true,
            };
            
            let manager = DownloadManager::new(3, download_folder.to_string());
            if let Ok(download_id) = manager.queue_download(request) {
                manager.process_queue().await?;
                
                // Check if download succeeded
                if let Some(progress) = manager.get_progress(&download_id) {
                    if matches!(progress.status, DownloadStatus::Completed) {
                        return Ok(download_id);
                    }
                }
            }
        }
    }
    
    Err("Failed to download track at any quality".to_string())
}