use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce, Key
};
use argon2::Argon2;
use rand::RngCore;
use std::fs;
use std::path::Path;
use console::style;

const LOAR_MAGIC: &[u8; 4] = b"LOAR";
const SALT_SIZE: usize = 16;
const IV_SIZE: usize = 12;

/// Check if the keyring error is caused by Snap confinement restrictions and print guide.
fn check_keyring_snap_error(err: &keyring::Error) {
    if std::env::var("SNAP").is_ok() {
        // NoEntry is a normal condition indicating that the password isn't set yet.
        // Other errors (like D-Bus errors, communication failures) in Snap mean permission denied.
        if !matches!(err, keyring::Error::NoEntry) {
            eprintln!("\n{}", style("snap error:").red().bold());
            eprintln!("To store encryption passwords securely in the OS Keyring under Snap's sandbox restriction,");
            eprintln!("please grant permission by running the following command:");
            eprintln!("\n    {}\n", style("sudo snap connect loar:password-manager-service").yellow().bold());
            eprintln!("Once connected, you can run loar again.");
            std::process::exit(1);
        }
    }
}

/// Retrieve stored password from OS keyring for a repository.
pub fn get_stored_password(repo_name: &str) -> Option<String> {
    let entry = match keyring::Entry::new("loar", repo_name) {
        Ok(e) => e,
        Err(e) => {
            check_keyring_snap_error(&e);
            return None;
        }
    };
    match entry.get_password() {
        Ok(p) => Some(p),
        Err(e) => {
            check_keyring_snap_error(&e);
            None
        }
    }
}

/// Store password to OS keyring for a repository.
pub fn store_password(repo_name: &str, password: &str) -> Result<(), String> {
    let entry = match keyring::Entry::new("loar", repo_name) {
        Ok(e) => e,
        Err(e) => {
            check_keyring_snap_error(&e);
            return Err(format!("Keyring init failed: {}", e));
        }
    };
    if let Err(e) = entry.set_password(password) {
        check_keyring_snap_error(&e);
        return Err(format!("Keyring save failed: {}", e));
    }
    Ok(())
}

/// Delete stored password from OS keyring for a repository.
pub fn delete_stored_password(repo_name: &str) -> Result<(), String> {
    if let Ok(entry) = keyring::Entry::new("loar", repo_name) {
        // Delete password from OS keyring. If it does not exist, ignore the error.
        if let Err(e) = entry.delete_credential() {
            check_keyring_snap_error(&e);
        }
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
/// Receives derived key and salt directly to bypass repetitive Argon2id key derivation for performance.
pub fn encrypt_file(
    src_path: &Path,
    dest_path: &Path,
    key_bytes: &[u8],
    salt: &[u8],
) -> Result<(String, String), String> {
    let plaintext = fs::read(src_path)
        .map_err(|e| format!("Failed to read source file: {}", e))?;

    // Generate random IV
    let mut iv = [0u8; IV_SIZE];
    let mut rng = rand::thread_rng();
    rng.fill_bytes(&mut iv);

    // Use pre-derived Key
    let key = Key::<Aes256Gcm>::from_slice(key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&iv);

    // Encrypt
    let ciphertext = cipher.encrypt(nonce, plaintext.as_slice())
        .map_err(|e| format!("AES-256-GCM encryption failed: {}", e))?;

    // Package: Magic + Salt + IV + Ciphertext
    let mut packaged = Vec::with_capacity(LOAR_MAGIC.len() + SALT_SIZE + IV_SIZE + ciphertext.len());
    packaged.extend_from_slice(LOAR_MAGIC);
    packaged.extend_from_slice(salt);
    packaged.extend_from_slice(&iv);
    packaged.extend_from_slice(&ciphertext);

    // Write to destination
    fs::write(dest_path, packaged)
        .map_err(|e| format!("Failed to write encrypted file: {}", e))?;

    // Return hex encoded IV and Salt for DB logging
    Ok((hex::encode(salt), hex::encode(&iv)))
}

/// Decrypts a packaged encrypted file and writes to target path.
/// Leverages cached_key and cached_salt to bypass repetitive Argon2id calculations when salt matches.
/// Falls back to dynamic key derivation if salt mismatch is detected to preserve backward compatibility.
pub fn decrypt_file(
    src_path: &Path,
    dest_path: &Path,
    password: &str,
    cached_key: Option<&[u8]>,
    cached_salt: Option<&[u8]>,
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
    let file_salt = &packaged[4..4 + SALT_SIZE];
    let iv = &packaged[4 + SALT_SIZE..header_len];
    let ciphertext = &packaged[header_len..];

    // Determine which key to use (Cached key vs fallback calculation)
    let key_bytes = if let (Some(ckey), Some(csalt)) = (cached_key, cached_salt) {
        if csalt == file_salt {
            // Salt matches cached salt: reuse pre-derived key instantly for speed!
            ckey.to_vec()
        } else {
            // Salt mismatch: fallback to derive key dynamically
            derive_key(password, file_salt)?
        }
    } else {
        derive_key(password, file_salt)?
    };

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
