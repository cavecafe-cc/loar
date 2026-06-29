use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce, Key
};
use argon2::Argon2;
use rand::RngCore;
use std::fs;
use std::path::Path;

const LOAR_MAGIC: &[u8; 4] = b"LOAR";
const SALT_SIZE: usize = 16;
const IV_SIZE: usize = 12;

/// Retrieve stored password from OS keyring for a repository.
pub fn get_stored_password(repo_name: &str) -> Option<String> {
    let entry = keyring::Entry::new("loar", repo_name).ok()?;
    entry.get_password().ok()
}

/// Store password to OS keyring for a repository.
pub fn store_password(repo_name: &str, password: &str) -> Result<(), String> {
    let entry = keyring::Entry::new("loar", repo_name)
        .map_err(|e| format!("Keyring init failed: {}", e))?;
    entry.set_password(password)
        .map_err(|e| format!("Keyring save failed: {}", e))?;
    Ok(())
}

/// Delete stored password from OS keyring for a repository.
pub fn delete_stored_password(repo_name: &str) -> Result<(), String> {
    if let Ok(entry) = keyring::Entry::new("loar", repo_name) {
        // Delete password from OS keyring. If it does not exist, ignore the error.
        let _ = entry.delete_credential();
    }
    Ok(())
}

/// Derive a 256-bit key from password and salt using Argon2id.
pub fn derive_key(password: &str, salt: &[u8]) -> Result<Vec<u8>, String> {
    let mut key = vec![0u8; 32];
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::default(),
    );
    argon2.hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| format!("Argon2 key derivation failed: {}", e))?;
    Ok(key)
}

/// Encrypts a source file and writes to target path with [LOAR Magic + Salt + IV + Encrypted Data] format.
pub fn encrypt_file(
    src_path: &Path,
    dest_path: &Path,
    password: &str,
) -> Result<(String, String), String> {
    let plaintext = fs::read(src_path)
        .map_err(|e| format!("Failed to read source file: {}", e))?;

    // Generate random Salt and IV
    let mut salt = [0u8; SALT_SIZE];
    let mut iv = [0u8; IV_SIZE];
    let mut rng = rand::thread_rng();
    rng.fill_bytes(&mut salt);
    rng.fill_bytes(&mut iv);

    // Derive Key
    let key_bytes = derive_key(password, &salt)?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&iv);

    // Encrypt
    let ciphertext = cipher.encrypt(nonce, plaintext.as_slice())
        .map_err(|e| format!("AES-256-GCM encryption failed: {}", e))?;

    // Package: Magic + Salt + IV + Ciphertext
    let mut packaged = Vec::with_capacity(LOAR_MAGIC.len() + SALT_SIZE + IV_SIZE + ciphertext.len());
    packaged.extend_from_slice(LOAR_MAGIC);
    packaged.extend_from_slice(&salt);
    packaged.extend_from_slice(&iv);
    packaged.extend_from_slice(&ciphertext);

    // Write to destination
    fs::write(dest_path, packaged)
        .map_err(|e| format!("Failed to write encrypted file: {}", e))?;

    // Return hex encoded IV and Salt for DB logging
    Ok((hex::encode(&salt), hex::encode(&iv)))
}

/// Decrypts a packaged encrypted file and writes to target path.
pub fn decrypt_file(
    src_path: &Path,
    dest_path: &Path,
    password: &str,
) -> Result<(), String> {
    let packaged = fs::read(src_path)
        .map_err(|e| format!("Failed to read encrypted file: {}", e))?;

    let header_len = LOAR_MAGIC.len() + SALT_SIZE + IV_SIZE;
    if packaged.len() < header_len {
        return Err("Encrypted file is corrupted or too small".to_string());
    }

    // Verify magic bytes
    if &packaged[0..4] != LOAR_MAGIC {
        return Err("Invalid file format: not a LoAr encrypted file".to_string());
    }

    // Extract salt, iv and ciphertext
    let salt = &packaged[4..4 + SALT_SIZE];
    let iv = &packaged[4 + SALT_SIZE..header_len];
    let ciphertext = &packaged[header_len..];

    // Derive key
    let key_bytes = derive_key(password, salt)?;
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(iv);

    // Decrypt
    let plaintext = cipher.decrypt(nonce, ciphertext)
        .map_err(|e| format!("AES-256-GCM decryption failed: check your password: {}", e))?;

    // Write decrypted file
    fs::write(dest_path, plaintext)
        .map_err(|e| format!("Failed to write decrypted file: {}", e))?;

    Ok(())
}

/// Helper module to hex-encode metadata
pub mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}
