use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use sha2::{Digest, Sha256};
use crate::db::{DbConnection, RepositoryRow};
use crate::scanner;
use crate::crypto;

/// Calculate SHA-256 hash of a file.
pub fn calculate_sha256(path: &Path) -> Result<String, String> {
    let mut file = File::open(path)
        .map_err(|e| format!("Failed to open file for hashing '{}': {}", path.display(), e))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = file.read(&mut buffer)
            .map_err(|e| format!("Failed to read file for hashing '{}': {}", path.display(), e))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Helper to generate a hash name for encrypted files to obfuscate filenames.
pub fn get_obfuscated_filename(relative_path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(relative_path.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Check if a directory is empty or only contains OS metadata (like .DS_Store, Thumbs.db) or effectively empty directories.
pub fn is_dir_effectively_empty(dir: &Path) -> bool {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(e) = entry {
                let path = e.path();
                if path.is_dir() {
                    if !is_dir_effectively_empty(&path) {
                        return false;
                    }
                } else {
                    let name = e.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str != ".DS_Store" && name_str != "Thumbs.db" {
                        return false;
                    }
                }
            }
        }
        true
    } else {
        false
    }
}

/// Clean up all effectively empty directories inside a given directory recursively (bottom-up), excluding the base directory itself.
pub fn cleanup_empty_directories(dir: &Path, base: &Path) -> Result<(), String> {
    if !dir.exists() || !dir.is_dir() {
        return Ok(());
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(e) = entry {
                let path = e.path();
                if path.is_dir() {
                    cleanup_empty_directories(&path, base)?;
                }
            }
        }
    }

    // If it's not the base directory itself and is effectively empty, delete it
    if dir != base && is_dir_effectively_empty(dir) {
        let _ = fs::remove_dir_all(dir);
    }

    Ok(())
}

/// Perform the backup process for a registered repository.
pub fn run_backup(
    repo: &RepositoryRow,
    target_dir: &str,
    global_exclude: &[String],
    db: &DbConnection,
    password: Option<&str>,
) -> Result<String, String> {
    let repo_path = Path::new(&repo.path);
    if !repo_path.exists() {
        return Err(format!("Repository source path '{}' does not exist", repo.path));
    }

    // Step 1: Scan files to archive
    let relative_files = scanner::scan_folder(repo_path, global_exclude)?;
    let latest_archive = db.get_latest_archive(repo.id)?;
    if relative_files.is_empty() && latest_archive.is_none() {
        return Ok("No new or ignored files found to archive.".to_string());
    }

    // Pre-derive key for performance if encryption is enabled
    // Argon2id is intentionally slow, so we derive the key once here.
    let mut repo_salt = [0u8; 16];
    let derived_key = if repo.encrypt {
        let pwd = password.ok_or_else(|| "Password is required for encrypted repository".to_string())?;
        
        let mut hasher = Sha256::new();
        hasher.update(repo.name.as_bytes());
        let hash_result = hasher.finalize();
        repo_salt.copy_from_slice(&hash_result[0..16]);
        
        let key = crypto::derive_key(pwd, &repo_salt)?;
        Some(key)
    } else {
        None
    };

    // Prepare target paths
    let target_base = Path::new(target_dir).join(&repo.name);
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
    
    // Create a temporary backup directory for Atomicity
    let temp_dir_name = format!(".tmp_loar_backup_{}_{}", timestamp, repo.name);
    let temp_dir = Path::new(target_dir).join(&temp_dir_name);
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to create temporary backup directory: {}", e))?;

    let mut archived_count = 0;
    let mut total_size = 0;
    let mut file_logs = Vec::new();

    // Load previous backup metadata for incremental comparison
    let mut previous_metadata = std::collections::HashMap::new();
    if let Ok(Some(latest_archive)) = db.get_latest_archive(repo.id) {
        if let Ok(records) = db.get_file_records_for_archive(latest_archive.id) {
            for rec in records {
                previous_metadata.insert(rec.relative_path.clone(), rec);
            }
        }
    }

    // Step 2: Copy or Encrypt files to the temporary directory
    for rel_path in &relative_files {
        let src_file_path = repo_path.join(rel_path);
        let metadata = fs::metadata(&src_file_path)
            .map_err(|e| format!("Failed to read file metadata for '{}': {}", src_file_path.display(), e))?;
        let file_size = metadata.len() as i64;
        let mtime = metadata.modified()
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64)
            .unwrap_or(0);

        let sha256 = calculate_sha256(&src_file_path)?;
        let rel_str = rel_path.to_string_lossy().to_string();

        // Check if file is unchanged from the previous backup
        let mut is_unchanged = false;
        let mut iv_hex = None;
        let mut salt_hex = None;

        if let Some(prev) = previous_metadata.get(&rel_str) {
            if prev.file_size == file_size && prev.mtime == mtime && prev.sha256_hash == sha256 {
                // If encrypted, verify if it actually exists in target folder
                let target_exists = if repo.encrypt {
                    let obs_name = get_obfuscated_filename(&rel_str);
                    target_base.join(&obs_name).exists()
                } else {
                    target_base.join(&rel_str).exists()
                };

                if target_exists {
                    is_unchanged = true;
                    iv_hex = prev.iv.clone();
                    salt_hex = prev.salt.clone();
                }
            }
        }

        if is_unchanged {
            // Unchanged file: Skip copying/encrypting, inherit metadata
            file_logs.push((rel_str, file_size, mtime, sha256, iv_hex, salt_hex));
            total_size += file_size;
            archived_count += 1;
            continue;
        }

        // Changed or new file: perform normal copy/encryption
        let (_, final_iv, final_salt) = if repo.encrypt {
            let key_bytes = derived_key.as_ref().ok_or_else(|| "Derived key missing".to_string())?;
            let obs_name = get_obfuscated_filename(&rel_str);
            let dest = temp_dir.join(&obs_name);
            
            // Encrypt and write to temporary folder using pre-derived key & repo salt
            let (salt, iv) = crypto::encrypt_file(&src_file_path, &dest, key_bytes, &repo_salt)?;
            (dest, Some(salt), Some(iv))
        } else {
            let dest = temp_dir.join(rel_path);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directory structure in temp folder: {}", e))?;
            }
            fs::copy(&src_file_path, &dest)
                .map_err(|e| format!("Failed to copy file to temp folder: {}", e))?;
            (dest, None, None)
        };

        archived_count += 1;
        total_size += file_size;
        file_logs.push((rel_str, file_size, mtime, sha256, final_iv, final_salt));
    }

    // Step 3: Transactional metadata logging and final atomic directory merge
    let run_db_operations = || -> Result<i64, String> {
        let archive_id = db.create_archive_session(repo.id, archived_count, total_size, None)?;
        for (rel_str, size, mtime, sha, iv, salt) in &file_logs {
            db.add_file_record(
                archive_id,
                rel_str,
                *size,
                *mtime,
                sha,
                iv.as_deref(),
                salt.as_deref(),
            )?;
        }
        Ok(archive_id)
    };

    let archive_id = match run_db_operations() {
        Ok(id) => id,
        Err(e) => {
            // Cleanup temp folder on failure
            let _ = fs::remove_dir_all(&temp_dir);
            return Err(format!("Database transactional logging failed: {}", e));
        }
    };

    // Step 4: Move/merge files from temporary directory to target base directory
    if !target_base.exists() {
        fs::create_dir_all(&target_base)
            .map_err(|e| format!("Failed to create target directory: {}", e))?;
    }

    // Helper to recursively copy files from temp folder to final destination
    fn merge_directories(src: &Path, dest: &Path) -> Result<(), String> {
        for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let entry_path = entry.path();
            let dest_path = dest.join(entry.file_name());

            if entry_path.is_dir() {
                fs::create_dir_all(&dest_path).map_err(|e| e.to_string())?;
                merge_directories(&entry_path, &dest_path)?;
            } else {
                fs::copy(&entry_path, &dest_path).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    if let Err(e) = merge_directories(&temp_dir, &target_base) {
        let _ = fs::remove_dir_all(&temp_dir);
        return Err(format!("Failed to move files to final destination: {}", e));
    }

    // Cleanup temp directory
    fs::remove_dir_all(&temp_dir)
        .map_err(|e| format!("Failed to cleanup temp directory: {}", e))?;

    // Step 5: One-way sync cleanup (delete files from backup target that were deleted in source)
    if repo.one_way_sync {
        if let Ok(Some(prev_archive)) = db.get_previous_archive(repo.id, archive_id) {
            if let Ok(prev_records) = db.get_file_records_for_archive(prev_archive.id) {
                let current_files_set: std::collections::HashSet<String> = relative_files
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect();

                for prev_rec in prev_records {
                    if !current_files_set.contains(&prev_rec.relative_path) {
                        let target_file = if repo.encrypt {
                            let obs_name = get_obfuscated_filename(&prev_rec.relative_path);
                            target_base.join(&obs_name)
                        } else {
                            target_base.join(&prev_rec.relative_path)
                        };

                        if target_file.exists() {
                            if let Err(e) = fs::remove_file(&target_file) {
                                  eprintln!("Warning: Failed to remove deleted file from backup '{}': {}", target_file.display(), e);
                            }
                        }

                        // Clean up empty parent directories (for plain copy mode)
                        if !repo.encrypt {
                            let mut parent = target_file.parent();
                            while let Some(p) = parent {
                                if p == target_base {
                                    break;
                                }
                                if p.exists() && is_dir_effectively_empty(p) {
                                    let _ = fs::remove_dir_all(p);
                                } else {
                                    break;
                                }
                                parent = p.parent();
                            }
                        }
                    }
                }
            }
        }
        // Delete older archive sessions from database to keep only the latest one
        if let Err(e) = db.delete_older_archives(repo.id, archive_id) {
            eprintln!("Warning: Failed to cleanup older archive histories from DB: {}", e);
        }
    }

    // Clean up all empty subdirectories inside the backup target
    if !repo.encrypt {
        let _ = cleanup_empty_directories(&target_base, &target_base);
    }

    // If the final target directory became empty, remove it to keep storage clean
    if target_base.exists() && is_dir_effectively_empty(&target_base) {
        let _ = fs::remove_dir_all(&target_base);
    }

    Ok(format!("Archive successfully completed. Files backup: {}, total size: {} bytes.", archived_count, total_size))
}

/// Restore the archive session to a target directory.
pub fn run_restore(
    repo: &RepositoryRow,
    archive_id: i64,
    dest_dir: &str,
    target_base_dir: &str,
    db: &DbConnection,
    password: Option<&str>,
) -> Result<String, String> {
    let dest_path = Path::new(dest_dir);
    if !dest_path.exists() {
        fs::create_dir_all(dest_path)
            .map_err(|e| format!("Failed to create restore destination folder: {}", e))?;
    }

    let records = db.get_file_records_for_archive(archive_id)?;
    if records.is_empty() {
        return Err("No files found in the specified archive session".to_string());
    }

    let archive_source_base = Path::new(target_base_dir).join(&repo.name);
    if !archive_source_base.exists() {
        return Err(format!("Backup source folder '{}' does not exist", archive_source_base.display()));
    }

    // Pre-derive key for performance if encryption is enabled
    let mut repo_salt = [0u8; 16];
    let derived_key = if repo.encrypt {
        let pwd = password.ok_or_else(|| "Password is required for encrypted recovery".to_string())?;
        
        let mut hasher = Sha256::new();
        hasher.update(repo.name.as_bytes());
        let hash_result = hasher.finalize();
        repo_salt.copy_from_slice(&hash_result[0..16]);
        
        let key = crypto::derive_key(pwd, &repo_salt)?;
        Some(key)
    } else {
        None
    };

    let mut restored_count = 0;

    for file_record in records {
        let restored_file_path = dest_path.join(&file_record.relative_path);
        
        // Ensure parent directories exist
        if let Some(parent) = restored_file_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create path: {}", e))?;
        }

        if repo.encrypt {
            let pwd = password.ok_or_else(|| "Password is required for encrypted recovery".to_string())?;
            let obs_name = get_obfuscated_filename(&file_record.relative_path);
            let src_file = archive_source_base.join(&obs_name);
            
            if !src_file.exists() {
                return Err(format!("Encrypted backup file '{}' not found", src_file.display()));
            }

            // Attempt decrypt using cached key & salt first, fall back to password computation if needed
            crypto::decrypt_file(
                &src_file,
                &restored_file_path,
                pwd,
                derived_key.as_deref(),
                Some(&repo_salt),
            )?;
        } else {
            let src_file = archive_source_base.join(&file_record.relative_path);
            
            if !src_file.exists() {
                return Err(format!("Backup file '{}' not found", src_file.display()));
            }

            fs::copy(&src_file, &restored_file_path)
                .map_err(|e| format!("Failed to restore file: {}", e))?;
        }

        restored_count += 1;
    }

    Ok(format!("Successfully restored {} files to '{}'.", restored_count, dest_dir))
}
