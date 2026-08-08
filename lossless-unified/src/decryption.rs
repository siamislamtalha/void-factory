use crate::types::*;
use aes::Aes256;
use aes::cipher::{
    BlockEncrypt, BlockDecrypt, KeyInit,
    generic_array::GenericArray,
};
use blowfish::Blowfish;
use blowfish::cipher::{
    BlockEncrypt as BlowfishEncrypt, BlockDecrypt as BlowfishDecrypt,
    BlockCipher as BlowfishCipher,
};
use md5::Md5;
use std::iter::zip;

/// Decrypt Tidal MQA encrypted streams
pub fn decrypt_tidal_mqa(data: &[u8], encryption_key: &str) -> Result<Vec<u8>, String> {
    // Master key for Tidal MQA decryption (from streamrip)
    let master_key_b64 = "UIlTTEMmmLfGowo/UC60x2H45W6MdGgTRfo/umg4754=";
    let master_key = base64::decode(master_key_b64)
        .map_err(|e| format!("Failed to decode master key: {}", e))?;
    
    let security_token = base64::decode(encryption_key)
        .map_err(|e| format!("Failed to decode security token: {}", e))?;
    
    if security_token.len() < 16 {
        return Err("Security token too short".to_string());
    }
    
    // Get IV from first 16 bytes of security token
    let iv = &security_token[..16];
    let encrypted_st = &security_token[16..];
    
    // Initialize decryptor with master key
    let key = GenericArray::from_slice(&master_key);
    let iv_array = GenericArray::from_slice(iv);
    let decryptor = Aes256::new(key);
    
    // Decrypt the security token
    let mut decrypted_st = encrypted_st.to_vec();
    for chunk in decrypted_st.chunks_mut(16) {
        let block = GenericArray::from_mut_slice(chunk);
        decryptor.decrypt_block_b2b(block, block);
    }
    
    // Remove PKCS7 padding
    if let Some(&pad_len) = decrypted_st.last() {
        if pad_len <= 16 && pad_len as usize <= decrypted_st.len() {
            decrypted_st.truncate(decrypted_st.len() - pad_len as usize);
        }
    }
    
    if decrypted_st.len() < 24 {
        return Err("Decrypted security token too short".to_string());
    }
    
    // Get the audio stream decryption key and nonce
    let audio_key = &decrypted_st[..16];
    let nonce = &decrypted_st[16..24];
    
    // For CTR mode, we need to implement the counter mode
    // Since AES crate doesn't have CTR directly, we'll use a simpler approach
    // In production, you'd use a proper CTR implementation
    
    // For now, return the data as-is (placeholder for proper decryption)
    Ok(data.to_vec())
}

/// Decrypt Deezer Blowfish encrypted streams
pub fn decrypt_deezer_blowfish(data: &[u8], track_id: &str) -> Result<Vec<u8>, String> {
    let blowfish_secret = "g4el58wc0zvf9na1";
    
    // Generate Blowfish key from track ID
    let blowfish_key = generate_deezer_blowfish_key(track_id);
    
    let cipher = Blowfish::new_from_slice(&blowfish_key)
        .map_err(|e| format!("Failed to create Blowfish cipher: {}", e))?;
    
    let chunk_size = 2048;
    let mut result = Vec::with_capacity(data.len());
    
    for (i, chunk) in data.chunks(chunk_size * 3).enumerate() {
        if chunk.len() >= 2048 {
            let mut encrypted_chunk = [0u8; 2048];
            encrypted_chunk.copy_from_slice(&chunk[..2048]);
            
            let mut block = GenericArray::from_mut_slice(&mut encrypted_chunk);
            cipher.decrypt_block(block);
            
            result.extend_from_slice(&encrypted_chunk);
            result.extend_from_slice(&chunk[2048..]);
        } else {
            result.extend_from_slice(chunk);
        }
    }
    
    Ok(result)
}

/// Generate Deezer Blowfish key from track ID
fn generate_deezer_blowfish_key(track_id: &str) -> Vec<u8> {
    let md5_hash = md5_hash(track_id);
    
    let mut key = Vec::new();
    for (a, b, c) in zip(
        md5_hash[..16].bytes(),
        md5_hash[16..].bytes(),
        blowfish_secret.bytes()
    ) {
        let xor_val = (a as u8) ^ (b as u8) ^ (c as u8);
        key.push(xor_val);
    }
    
    key
}

/// Calculate MD5 hash of a string
fn md5_hash(input: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Decrypt Deezer AES encrypted file URL
pub fn decrypt_deezer_file_url(track_id: &str, track_hash: &str, media_version: &str) -> Result<String, String> {
    let url_bytes = [
        track_hash.as_bytes(),
        b"\xa4",
        b"1",
        b"\xa4",
        track_id.as_bytes(),
        b"\xa4",
        media_version.as_bytes(),
        b"\xa4",
    ].concat();
    
    let url_hash = md5_hash_vec(&url_bytes);
    let mut info_bytes = url_hash.into_bytes();
    info_bytes.extend_from_slice(b"\xa4");
    info_bytes.extend_from_slice(&url_bytes);
    info_bytes.extend_from_slice(b"\xa4");
    
    // Pad to multiple of 16
    let padding_len = (16 - (info_bytes.len() % 16)) % 16;
    info_bytes.extend(vec![b'.'; padding_len]);
    
    // Encrypt with AES ECB
    let key = b"jo6aey6haid2Teih";
    let cipher = Aes256::new(GenericArray::from_slice(key));
    
    let mut encrypted = Vec::new();
    for chunk in info_bytes.chunks(16) {
        let mut block = GenericArray::clone_from_slice(chunk);
        cipher.encrypt_block(&mut block);
        encrypted.extend_from_slice(block.as_slice());
    }
    
    let path = hex::encode(encrypted);
    let first_char = track_hash.chars().next().unwrap_or('0');
    
    Ok(format!("https://e-cdns-proxy-{}.dzcdn.net/mobile/1/{}", first_char, path))
}

/// Calculate MD5 hash of bytes
fn md5_hash_vec(input: &[u8]) -> String {
    let mut hasher = Md5::new();
    hasher.update(input);
    format!("{:x}", hasher.finalize())
}

/// Generate Blowfish key for Deezer decryption (public version)
pub fn generate_blowfish_key(track_id: &str) -> Vec<u8> {
    generate_deezer_blowfish_key(track_id)
}

/// Decrypt Blowfish (public wrapper)
pub fn decrypt_blowfish(data: &[u8], track_id: &str) -> Result<Vec<u8>, String> {
    decrypt_deezer_blowfish(data, track_id)
}

/// Decrypt AES (public wrapper)
pub fn decrypt_aes(data: &[u8], _key: &[u8]) -> Result<Vec<u8>, String> {
    // Placeholder for AES decryption
    Ok(data.to_vec())
}
