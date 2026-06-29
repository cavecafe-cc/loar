use rusqlite::{params, Connection, Result};
use std::path::Path;

pub struct DbConnection {
    conn: Connection,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct RepositoryRow {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub encrypt: bool,
    pub one_way_sync: bool,
    pub created_at: String,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct ArchiveRow {
    pub id: i64,
    pub repo_id: i64,
    pub timestamp: String,
    pub file_count: i64,
    pub total_size: i64,
    pub master_hash: Option<String>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct FileRecordRow {
    pub id: i64,
    pub archive_id: i64,
    pub relative_path: String,
    pub file_size: i64,
    pub mtime: i64,
    pub sha256_hash: String,
    pub iv: Option<String>,
    pub salt: Option<String>,
}

impl DbConnection {
    pub fn open(target_dir: &str) -> Result<Self, String> {
        let target_path = Path::new(target_dir);
        if !target_path.exists() {
            std::fs::create_dir_all(&target_path)
                .map_err(|e| format!("Failed to create database directory: {}", e))?;
        }
        let db_path = target_path.join("loar.db");
        let conn = Connection::open(db_path)
            .map_err(|e| format!("Failed to open SQLite database: {}", e))?;

        // Enable foreign key constraints for cascade delete
        conn.execute("PRAGMA foreign_keys = ON;", [])
            .map_err(|e| format!("Failed to enable foreign key support: {}", e))?;

        let db = Self { conn };
        db.init_schema()?;
        db.migrate_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<(), String> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS repositories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                path TEXT NOT NULL UNIQUE,
                encrypt INTEGER NOT NULL,
                one_way_sync INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL
            )",
            [],
        ).map_err(|e| format!("Failed to create repositories table: {}", e))?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS archives (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_id INTEGER NOT NULL,
                timestamp TEXT NOT NULL,
                file_count INTEGER NOT NULL,
                total_size INTEGER NOT NULL,
                master_hash TEXT,
                FOREIGN KEY(repo_id) REFERENCES repositories(id) ON DELETE CASCADE
            )",
            [],
        ).map_err(|e| format!("Failed to create archives table: {}", e))?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS file_records (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                archive_id INTEGER NOT NULL,
                relative_path TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                mtime INTEGER NOT NULL,
                sha256_hash TEXT NOT NULL,
                iv TEXT,
                salt TEXT,
                FOREIGN KEY(archive_id) REFERENCES archives(id) ON DELETE CASCADE
            )",
            [],
        ).map_err(|e| format!("Failed to create file_records table: {}", e))?;

        Ok(())
    }

    fn migrate_schema(&self) -> Result<(), String> {
        let has_col: i32 = self.conn.query_row(
            "SELECT count(*) FROM pragma_table_info('repositories') WHERE name='one_way_sync'",
            [],
            |row| row.get(0),
        ).map_err(|e| format!("Failed to check table info: {}", e))?;

        if has_col == 0 {
            self.conn.execute(
                "ALTER TABLE repositories ADD COLUMN one_way_sync INTEGER DEFAULT 1",
                [],
            ).map_err(|e| format!("Failed to migrate database (adding one_way_sync column): {}", e))?;
            println!("Database migrated: Added one_way_sync column to repositories.");
        }
        Ok(())
    }

    // Repository operations
    pub fn add_repository(&self, name: &str, path: &str, encrypt: bool, one_way_sync: bool) -> Result<i64, String> {
        let created_at = chrono::Local::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO repositories (name, path, encrypt, one_way_sync, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![name, path, if encrypt { 1 } else { 0 }, if one_way_sync { 1 } else { 0 }, created_at],
        ).map_err(|e| format!("Failed to add repository: {}", e))?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_repositories(&self) -> Result<Vec<RepositoryRow>, String> {
        let mut stmt = self.conn.prepare("SELECT id, name, path, encrypt, one_way_sync, created_at FROM repositories")
            .map_err(|e| format!("Failed to prepare select repositories: {}", e))?;

        let rows = stmt.query_map([], |row| {
            let encrypt_val: i32 = row.get(3)?;
            let sync_val: i32 = row.get(4)?;
            Ok(RepositoryRow {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                encrypt: encrypt_val != 0,
                one_way_sync: sync_val != 0,
                created_at: row.get(5)?,
            })
        }).map_err(|e| format!("Failed to query repositories: {}", e))?;

        let mut repos = Vec::new();
        for r in rows {
            repos.push(r.map_err(|e| format!("Error mapping repository row: {}", e))?);
        }
        Ok(repos)
    }

    pub fn get_repository_by_path(&self, path: &str) -> Result<Option<RepositoryRow>, String> {
        let mut stmt = self.conn.prepare("SELECT id, name, path, encrypt, one_way_sync, created_at FROM repositories WHERE path = ?1")
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let mut rows = stmt.query_map(params![path], |row| {
            let encrypt_val: i32 = row.get(3)?;
            let sync_val: i32 = row.get(4)?;
            Ok(RepositoryRow {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                encrypt: encrypt_val != 0,
                one_way_sync: sync_val != 0,
                created_at: row.get(5)?,
            })
        }).map_err(|e| format!("Failed to query repository by path: {}", e))?;

        if let Some(row_result) = rows.next() {
            let row = row_result.map_err(|e| format!("Error mapping repository row: {}", e))?;
            Ok(Some(row))
        } else {
            Ok(None)
        }
    }

    pub fn delete_older_archives(&self, repo_id: i64, keep_archive_id: i64) -> Result<(), String> {
        self.conn.execute(
            "DELETE FROM archives WHERE repo_id = ?1 AND id != ?2",
            params![repo_id, keep_archive_id],
        ).map_err(|e| format!("Failed to delete older archives: {}", e))?;

        // Reclaim unused space to shrink the physical db file size
        let _ = self.conn.execute("VACUUM", []);
        Ok(())
    }

    pub fn delete_repository(&self, id: i64) -> Result<(), String> {
        self.conn.execute("DELETE FROM repositories WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to delete repository: {}", e))?;
        Ok(())
    }

    // Archive operations
    pub fn create_archive_session(&self, repo_id: i64, file_count: i64, total_size: i64, master_hash: Option<&str>) -> Result<i64, String> {
        let timestamp = chrono::Local::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO archives (repo_id, timestamp, file_count, total_size, master_hash) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![repo_id, timestamp, file_count, total_size, master_hash],
        ).map_err(|e| format!("Failed to create archive session: {}", e))?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn add_file_record(&self, archive_id: i64, relative_path: &str, file_size: i64, mtime: i64, sha256_hash: &str, iv: Option<&str>, salt: Option<&str>) -> Result<(), String> {
        self.conn.execute(
            "INSERT INTO file_records (archive_id, relative_path, file_size, mtime, sha256_hash, iv, salt) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![archive_id, relative_path, file_size, mtime, sha256_hash, iv, salt],
        ).map_err(|e| format!("Failed to insert file record: {}", e))?;
        Ok(())
    }

    pub fn get_latest_archive(&self, repo_id: i64) -> Result<Option<ArchiveRow>, String> {
        let mut stmt = self.conn.prepare("SELECT id, repo_id, timestamp, file_count, total_size, master_hash FROM archives WHERE repo_id = ?1 ORDER BY timestamp DESC LIMIT 1")
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let mut rows = stmt.query_map(params![repo_id], |row| {
            Ok(ArchiveRow {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                timestamp: row.get(2)?,
                file_count: row.get(3)?,
                total_size: row.get(4)?,
                master_hash: row.get(5)?,
            })
        }).map_err(|e| format!("Failed to query latest archive: {}", e))?;

        if let Some(row_result) = rows.next() {
            let row = row_result.map_err(|e| format!("Error mapping archive row: {}", e))?;
            Ok(Some(row))
        } else {
            Ok(None)
        }
    }

    pub fn get_previous_archive(&self, repo_id: i64, current_archive_id: i64) -> Result<Option<ArchiveRow>, String> {
        let mut stmt = self.conn.prepare("SELECT id, repo_id, timestamp, file_count, total_size, master_hash FROM archives WHERE repo_id = ?1 AND id != ?2 ORDER BY timestamp DESC LIMIT 1")
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let mut rows = stmt.query_map(params![repo_id, current_archive_id], |row| {
            Ok(ArchiveRow {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                timestamp: row.get(2)?,
                file_count: row.get(3)?,
                total_size: row.get(4)?,
                master_hash: row.get(5)?,
            })
        }).map_err(|e| format!("Failed to query previous archive: {}", e))?;

        if let Some(row_result) = rows.next() {
            let row = row_result.map_err(|e| format!("Error mapping archive row: {}", e))?;
            Ok(Some(row))
        } else {
            Ok(None)
        }
    }

    pub fn get_file_records_for_archive(&self, archive_id: i64) -> Result<Vec<FileRecordRow>, String> {
        let mut stmt = self.conn.prepare("SELECT id, archive_id, relative_path, file_size, mtime, sha256_hash, iv, salt FROM file_records WHERE archive_id = ?1")
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let rows = stmt.query_map(params![archive_id], |row| {
            Ok(FileRecordRow {
                id: row.get(0)?,
                archive_id: row.get(1)?,
                relative_path: row.get(2)?,
                file_size: row.get(3)?,
                mtime: row.get(4)?,
                sha256_hash: row.get(5)?,
                iv: row.get(6)?,
                salt: row.get(7)?,
            })
        }).map_err(|e| format!("Failed to query file records: {}", e))?;

        let mut records = Vec::new();
        for r in rows {
            records.push(r.map_err(|e| format!("Error mapping file record row: {}", e))?);
        }
        Ok(records)
    }
}
