use crate::types::{ApiConfig, ApiInstance, QobuzConfig, TidalConfig, DeezerConfig};
use lazy_static::lazy_static;
use std::sync::RwLock;

lazy_static! {
    static ref API_CONFIG: RwLock<ApiConfig> = RwLock::new(ApiConfig {
        // Qobuz credentials - app_id and secrets are fetched dynamically from play.qobuz.com
        qobuz: QobuzConfig {
            email_or_userid: None,
            password_or_token: None,
            app_id: Some("639242930".to_string()), // Will be updated dynamically
            secrets: vec![
                // These are fetched dynamically from Qobuz's bundle.js
                // The client will attempt to fetch them if not provided
            ],
            use_auth_token: false,
        },
        // Tidal credentials - using hardcoded client credentials from streamrip and Monochrome
        tidal: TidalConfig {
            access_token: None, // Will be obtained via OAuth
            refresh_token: None,
            user_id: None,
            country_code: Some("US".to_string()),
            token_expiry: None,
        },
        // Deezer credentials - ARL token needed for full FLAC quality
        deezer: DeezerConfig {
            arl: None, // User needs to provide ARL for full quality
        },
        // Monochrome API instances from instances.json
        proxy_instances: vec![
            ApiInstance {
                url: "https://eu-central.monochrome.tf".to_string(),
                version: Some("2.10".to_string()),
                is_user: false,
            },
            ApiInstance {
                url: "https://us-west.monochrome.tf".to_string(),
                version: Some("2.10".to_string()),
                is_user: false,
            },
            ApiInstance {
                url: "https://arran.monochrome.tf".to_string(),
                version: Some("2.10".to_string()),
                is_user: false,
            },
            ApiInstance {
                url: "https://api.monochrome.tf/".to_string(),
                version: Some("2.10".to_string()),
                is_user: false,
            },
            ApiInstance {
                url: "https://monochrome-api.samidy.com".to_string(),
                version: Some("2.10".to_string()),
                is_user: false,
            },
            ApiInstance {
                url: "https://triton.squid.wtf".to_string(),
                version: Some("2.10".to_string()),
                is_user: false,
            },
            ApiInstance {
                url: "https://wolf.qqdl.site".to_string(),
                version: Some("2.10".to_string()),
                is_user: false,
            },
            ApiInstance {
                url: "https://maus.qqdl.site".to_string(),
                version: Some("2.10".to_string()),
                is_user: false,
            },
            ApiInstance {
                url: "https://vogel.qqdl.site".to_string(),
                version: Some("2.10".to_string()),
                is_user: false,
            },
            ApiInstance {
                url: "https://hund.qqdl.site".to_string(),
                version: Some("2.10".to_string()),
                is_user: false,
            },
            ApiInstance {
                url: "https://tidal.kinoplus.online".to_string(),
                version: Some("2.10".to_string()),
                is_user: false,
            },
        ],
    });
}

/// Initialize API configuration with credentials
pub fn initialize_api_config() {
    let mut config = API_CONFIG.write().unwrap();
    
    // Proxy instances are already hardcoded in the lazy_static initializer
    // No additional initialization needed
}

/// Get the full API configuration
pub fn get_api_config() -> ApiConfig {
    API_CONFIG.read().unwrap().clone()
}

/// Set Qobuz credentials
pub fn set_qobuz_credentials(email: Option<String>, password: Option<String>, app_id: Option<String>, secrets: Vec<String>) {
    let mut config = API_CONFIG.write().unwrap();
    config.qobuz.email_or_userid = email;
    config.qobuz.password_or_token = password;
    config.qobuz.app_id = app_id;
    config.qobuz.secrets = secrets;
}

/// Set Tidal credentials
pub fn set_tidal_credentials(access_token: String, refresh_token: String, user_id: String, country_code: String) {
    let mut config = API_CONFIG.write().unwrap();
    config.tidal.access_token = Some(access_token);
    config.tidal.refresh_token = Some(refresh_token);
    config.tidal.user_id = Some(user_id);
    config.tidal.country_code = Some(country_code);
}

/// Set Deezer credentials
pub fn set_deezer_credentials(arl: String) {
    let mut config = API_CONFIG.write().unwrap();
    config.deezer.arl = Some(arl);
}

/// Get Tidal client credentials (hardcoded from streamrip and Monochrome)
pub fn get_tidal_client_credentials() -> (String, String) {
    // From streamrip tidal.py (base64 decoded)
    // CLIENT_ID = base64.b64decode("ZlgySnhkbW50WldLMGl4VA==").decode("iso-8859-1")
    // CLIENT_SECRET = base64.b64decode("MU5tNUFmREFqeHJnSkZKYktOV0xlQXlLR1ZHbUlOdVhQUExIVlhBdnhBZz0=").decode("iso-8859-1")
    let client_id = "fLxJxmntWZK0ixT".to_string();
    let client_secret = "M5nMAfDAjxrgFJbKNWLeAyKGVGmINuXPLIXAvxAg=".to_string();
    
    (client_id, client_secret)
}

/// Get Unified Playback API credentials (from Monochrome)
pub fn get_unified_api_credentials() -> (String, String, String) {
    let api_token = "amp_29b2lIr4mze4tK-P8QDOxfMZ9anCgJ9_uGTUks3nIyo".to_string();
    let api_base_url = "https://music-api.geeked.wtf".to_string();
    let turnstile_site_key = "0x4AAAAAADgxqF6QVMm0GLHH".to_string();
    
    (api_token, api_base_url, turnstile_site_key)
}

/// Get podcast API credentials (from Monochrome)
pub fn get_podcast_api_credentials() -> (String, String) {
    let api_key = "YU5HMSDYBQQVYDF6QN4P".to_string();
    let api_secret = "p8s7v9x2k5m4n1q3w6e9r0t2y5u8i1o4".to_string();
    
    (api_key, api_secret)
}

/// Get Last.fm API credentials (from Monochrome)
pub fn get_lastfm_api_credentials() -> String {
    "85214f5abbc730e78770f27784b9bdf7".to_string()
}

/// Get Monochrome Tidal client credentials (alternative)
pub fn get_monochrome_tidal_credentials() -> (String, String) {
    let client_id = "txNoH4kkV41MfH25".to_string();
    let client_secret = "dQjy0MinCEvxi1O4UmxvxWnDjt4cgHBPw8ll6nYBk98=".to_string();
    
    (client_id, client_secret)
}

/// Get streamrip Tidal client credentials (alternative)
pub fn get_streamrip_tidal_credentials() -> (String, String) {
    let client_id = "fX2mnDmntWLzMixT".to_string();
    let client_secret = "5Nt5AfDjxrgJFJKNOWLeAyKGVGmINuXPLXAdvAgg=".to_string();
    
    (client_id, client_secret)
}

/// Get API instances for a specific type (legacy compatibility)
pub fn get_instances(instance_type: &str) -> Vec<ApiInstance> {
    let config = API_CONFIG.read().unwrap();
    config.proxy_instances.clone()
}

/// Add user-defined proxy instance
pub fn add_user_instance(url: String, version: Option<String>) {
    let mut config = API_CONFIG.write().unwrap();
    let instance = ApiInstance {
        url,
        version,
        is_user: true,
    };
    config.proxy_instances.push(instance);
}

/// Apply proxy transformation to URLs if needed
pub fn apply_proxy_transform(url: &str) -> String {
    // For now, return URL as-is. Proxy transformation can be added if needed
    url.to_string()
}

/// Get next available proxy instance with round-robin
pub fn get_next_instance(instance_type: &str) -> Option<ApiInstance> {
    let instances = get_instances(instance_type);
    if instances.is_empty() {
        return None;
    }
    
    // Simple round-robin using a counter
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let index = COUNTER.fetch_add(1, Ordering::Relaxed) % instances.len();
    
    Some(instances[index].clone())
}