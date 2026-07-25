# Build Instructions for Universal Downloader Plugin

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
bex pack universal-downloader
```

This will create `universal-downloader.bex` in the current directory.

### Option 3: Manual Build with cargo-component (Required for Windows)
**Note**: Standard `cargo build` will fail on Windows due to missing MSVC linker. Use cargo-component instead.

If you have Rust and cargo-component installed:

```bash
cd c:\Users\siami\OneDrive\Desktop\NEW VOID\plugins\universal-downloader
cargo component build --release
```

The compiled `.wasm` file will be in `target/wasm32-unknown-unknown/release/`.

To install cargo-component:
```bash
cargo install cargo-component --locked
```

## What This Plugin Does

The universal downloader plugin provides reliable music downloading by implementing multiple download methods from various APK reference implementations with automatic fallback:

1. **YouTube Download Methods** - Full Innertube API implementation with:
   - Primary API key + 7 backup API keys from APK reference
   - ANDROID_VR, IOS, TVHTML5, and WEB_REMIX client fallbacks for streaming
   - Signature cipher decoding for encrypted streams
   - Range request support to avoid YouTube throttling
   - Visitor data extraction and caching

2. **JioSaavn Download Methods** - Full API implementation with:
   - DES-ECB decryption for stream URLs (key: "38346591")
   - Multiple server endpoints with automatic rotation
   - Quality selection (96kbps, 128kbps, 160kbps, 320kbps)
   - Full streaming support

3. **Direct HTTP Fallback** - For unsupported sources:
   - Direct URL downloads
   - Progressive download with resume capability
   - Support for various audio formats

## App Compatibility

✅ **No app code changes needed** - The plugin uses the standard content-resolver interface:
- Plugin ID: `content-resolver.bloomfactory.universal-downloader`
- Standard function signatures: `get_streams()`, etc.
- Standard data structures returned
- Publisher: Void Music

## Installation

Once you have the `.bex` file:

1. Open your app
2. Go to Plugin Manager
3. Install the new `universal-downloader.bex` file
4. The app will automatically recognize the new plugin

## Usage

After installation:
1. Use the download function in your app
2. The plugin will automatically try multiple download methods in priority order
3. If one method fails, it automatically falls back to the next method
4. Results are returned with the best available stream

## Download Method Priority

The plugin implements a priority-based fallback system:

1. **YouTube Innertube API** (highest priority)
   - Tries ANDROID_VR client first
   - Falls back to IOS client
   - Falls back to TVHTML5 client
   - Falls back to WEB_REMIX client
   - Rotates through 8 API keys on failure

2. **JioSaavn DES-ECB** (medium priority)
   - Tries primary server first
   - Rotates through 4 backup servers on failure
   - Applies DES-ECB decryption to encrypted URLs

3. **Direct HTTP** (fallback priority)
   - Used when all other methods fail
   - Supports direct URL downloads
   - Progressive download with resume capability

## Troubleshooting

If the plugin fails to load:
- Ensure the `.bex` file was built successfully
- Check that the plugin ID matches: `content-resolver.bloomfactory.universal-downloader`
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
- Default servers with automatic rotation:
  - `saavn.echomusic.fun`
  - `saavn1.echomusic.fun`
  - `saavn2.echomusic.fun`
  - `www.jiosaavn.com`

**Server Rotation**: JioSaavn uses a server rotator that automatically cycles through available servers on failure, similar to the YouTube API key rotation.

## Implementation Details

This plugin contains the complete implementations:
- **credentials.rs** - Credential pool with 8 YouTube API keys and 4 JioSaavn servers with rotators
- **youtube_downloader.rs** - Full YouTube client with multi-client streaming, credential rotation, cipher decoding
- **jiosaavn_downloader.rs** - Full JioSaavn client with DES-ECB decryption, server rotation, quality selection
- **http_downloader.rs** - Direct HTTP fallback with resume capability, format detection
- **stream_resolver.rs** - Stream resolver with automatic fallback coordination between all methods
- **lib.rs** - Main plugin entry point implementing the bex-core content-resolver interface

## Performance

- **Automatic credential rotation** prevents rate limiting
- **Multi-client fallback** ensures high availability for YouTube
- **Server rotation** for JioSaavn prevents single-point failures
- **Progressive download** with resume capability for large files
- **Optimized WASM build** for fast loading
- **Priority-based fallback** ensures best quality streams are tried first
