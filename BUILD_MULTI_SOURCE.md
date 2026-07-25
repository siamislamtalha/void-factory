# Build Instructions for Multi-Source Plugin

## Quick Build (Recommended)

The plugin uses the existing GitHub Actions workflow. To build:

### Option 1: GitHub Actions (Easiest)
1. Push your changes to the `plugins` repository
2. The `.github/workflows/bex-factory.yml` will automatically build all plugins
3. Download the `.bex` file from the GitHub release
4. Install it in your app

### Option 2: Manual Build with bex CLI
If you have the `bex` CLI tool installed:

```bash
cd c:\Users\siami\OneDrive\Desktop\NEW VOID\plugins
bex pack multi-source
```

This will create `multi-source.bex` in the current directory.

### Option 3: Manual Build with cargo-component (Required for Windows)
**Note**: Standard `cargo build` will fail on Windows due to missing MSVC linker. Use cargo-component instead.

If you have Rust and cargo-component installed:

```bash
cd c:\Users\siami\OneDrive\Desktop\NEW VOID\plugins\multi-source
cargo component build --release
```

The compiled `.wasm` file will be in `target/wasm32-unknown-unknown/release/`.

To install cargo-component:
```bash
cargo install cargo-component --locked
```

## What This Plugin Does

The multi-source plugin aggregates search results from multiple music services using **actual API implementations** from existing plugins:

1. **YouTube Music** - Full Innertube API implementation with:
   - Primary API key + 6 backup API keys from APK reference
   - ANDROID_VR, IOS, and TVHTML5 client fallbacks for streaming
   - Visitor data extraction and caching
   - Signature cipher decoding
   - Full search, streaming, and playback support

2. **YouTube Video** - Full Innertube API implementation with:
   - Same multi-client streaming strategy as YouTube Music
   - Signature cipher decoding
   - Full search, streaming, and playback support

3. **JioSaavn** - Full API implementation with:
   - DES-ECB decryption for stream URLs (key: "38346591")
   - Public API for search and metadata
   - Full search, streaming, and playback support

All results are tagged with source prefixes:
- `ytm:` for YouTube Music
- `ytv:` for YouTube Video
- `jio:` for JioSaavn

## App Compatibility

✅ **No app code changes needed** - The plugin uses the standard content-resolver interface:
- Plugin ID: `content-resolver.bloomfactory.multisource`
- Standard function signatures: `search()`, `get_track_details()`, `get_streams()`, etc.
- Standard data structures returned
- Publisher: Void Music

## Installation

Once you have the `.bex` file:

1. Open your app
2. Go to Plugin Manager
3. Install the new `multi-source.bex` file
4. The app will automatically recognize the new plugin alongside existing 12 plugins

## Usage

After installation:
1. Use the search function in your app
2. Select the "multi-source" plugin as the search source
3. Results will be aggregated from YouTube Music, YouTube Video, and JioSaavn
4. Each result will show its source (ytm:, ytv:, jio:)
5. Click on any result to play/download with full streaming support

## Streaming Capabilities

Full streaming support for all included services:
- **YouTube Music**: Full streaming with ANDROID_VR/IOS/TVHTML5 client fallbacks
- **YouTube Video**: Full streaming with signature cipher decoding
- **JioSaavn**: Full streaming with DES-ECB decryption


## Troubleshooting

If the plugin fails to load:
- Ensure the `.bex` file was built successfully
- Check that the plugin ID matches: `content-resolver.bloomfactory.multisource`
- Verify the manifest version is compatible with your app
- Confirm publisher is set to: Void Music

## API Notes

All APIs use credential pools extracted from APK reference folder for high availability:

### YouTube/Innertube API Keys (8 keys total)
The plugin uses a credential rotator that automatically cycles through 8 API keys from various music apps:
- `AIzaSyC9XL3ZjWddXya6X74dJoCTL-WEYFDNX30` (WEB_REMIX - InnerTune, Kreate)
- `AIzaSyC9XL3ZjWddXya6X74dJoCTL-WEYFDNX3` (WEB - InnerTune, OuterTune, OpenTune)
- `AIzaSyDyT5W0Jh49F30Pqqtyfdf7pDLFKLJoAnw` (PoToken - RiMusic, Kreate, OuterTune)
- `AIzaSyAOghZGza2MQSZkY_zfZ370N-PUdXEo8AI` (ANDROID_MUSIC - Musify, InnerTune, Kreate)
- `AIzaSyA8eiZmM1FaDVjRy-df2KTyQ_vz_yYM39w` (ANDROID - InnerTune, Kreate)
- `AIzaSyB-63vPrdThhKuerbB2N_l7Kwwcxj6yUAc` (IOS - Musify, Kreate)
- `AIzaSyDCU8hByM-4DrUqRUYnGn-3llEO78bcxq8` (TVHTML5 - InnerTune, Kreate)
- `AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8` (Additional - Musify)

**Credential Rotation**: If one key fails, the plugin automatically rotates to the next available key. This ensures high availability even if some keys are rate-limited or deprecated.

### JioSaavn Credentials (from Echo-Music APK reference)
- DES-ECB decryption key: `38346591`
- Remote config URL: `https://echomusic.fun/saavn.json`
- Base URL: `www.jiosaavn.com`
- API version: `4`
- Context strings: `web6dot0`, `wap6dot0`
- Default servers with automatic rotation:
  - `saavn.echomusic.fun`
  - `saavn1.echomusic.fun`
  - `saavn2.echomusic.fun`

**Server Rotation**: JioSaavn uses a server rotator that automatically cycles through available servers on failure, similar to the YouTube API key rotation.

### Spotify Credentials (from OpenTune and BlackHole APK reference)
- Token URL: `https://open.spotify.com/api/token`
- Server time URL: `https://open.spotify.com/api/server-time`
- Nuance GIST URL: `https://api.github.com/gists/22ed9c6ba463899e933427f7de1f0eef`
- API base URL: `https://api.spotify.com/v1`
- Accounts API URL: `https://accounts.spotify.com/api`
- OAuth Client ID: `08de4eaf71904d1b95254fab3015d711` (from BlackHole)
- OAuth Client Secret: `622b4fbad33947c59b95a6ae607de11d` (from BlackHole)
- Redirect URL: `blackhole://spotify/auth`

**Authentication Methods:**
1. TOTP-based authentication with sp_dc cookies (from OpenTune implementation)
2. OAuth client credentials (from BlackHole implementation)

**Credential Rotation**: Spotify OAuth credentials use credential rotators for automatic failover.

## Implementation Details

This plugin contains the complete implementations copied from existing plugins:
- **credentials.rs** - Credential pool with 8 YouTube API keys and rotator for automatic failover
- **ytmusic_client.rs** - Full YouTube Music client with visitor data, multi-client streaming, credential rotation
- **ytmusic_cipher.rs** - Signature cipher decoding for YouTube streams
- **ytmusic_parser.rs** - JSON response parsing for YouTube Music API
- **ytmusic_mapper.rs** - Maps parsed data to bex-core types
- **ytvideo_client.rs** - Full YouTube Video client with same capabilities, credential rotation
- **ytvideo_cipher.rs** - Signature cipher for YouTube Video
- **ytvideo_parser.rs** - JSON parsing for YouTube Video API
- **ytvideo_mapper.rs** - Maps YouTube Video data to bex-core types
- **jiosaavn_client.rs** - Full JioSaavn client with search and streaming
- **jiosaavn_crypto.rs** - DES-ECB decryption for JioSaavn streams (uses credentials module)
- **jiosaavn_mapper.rs** - Maps JioSaavn data to bex-core types
- **jiosaavn_types.rs** - JioSaavn API response types
