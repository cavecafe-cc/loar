mod config;
mod db;
mod scanner;
mod crypto;
mod archive;

use clap::{Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, Input, Select, MultiSelect, Confirm};
use console::{style, Style};
use std::path::Path;
use db::DbConnection;
use config::{AppConfig, RepositoryConfig};

fn custom_theme() -> ColorfulTheme {
    ColorfulTheme {
        active_item_prefix: style("  ●".to_string()).cyan(),
        inactive_item_prefix: style("  ○".to_string()).dim(),
        checked_item_prefix: style("  ●".to_string()).green(),
        unchecked_item_prefix: style("  ○".to_string()).dim(),
        inactive_item_style: Style::new().dim(),
        ..ColorfulTheme::default()
    }
}

fn print_banner() {
    let banner = r#"
██╗      ██████╗  █████╗ ██████╗ 
██║     ██╔═══██╗██╔══██╗██╔══██╗
██║     ██║   ██║███████║██████╔╝
██║     ██║   ██║██╔══██║██╔══██╗
███████╗╚██████╔╝██║  ██║██║  ██║
╚══════╝ ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝
"#;
    println!();
    println!("{}", style(banner.trim_matches('\n')).cyan().bold());
}


#[derive(Parser)]
#[command(name = "loar")]
#[command(version)]
#[command(about = "Local Archive (LoAr) Utility - Backup local-only files securely", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run archiving/backup for registered repositories
    Run {
        /// Name of the specific repository to archive (omitting runs all)
        #[arg(short, long)]
        repo: Option<String>,

        /// Encryption password for backup (if not stored in keyring)
        #[arg(short, long)]
        password: Option<String>,
    },
    /// Register a new folder or repository for archiving
    Register {
        /// Unique identification name for the repository
        #[arg(short, long)]
        name: String,

        /// Absolute path of the folder to archive
        #[arg(short, long)]
        path: String,

        /// Enable AES-256 encryption for the archived files
        #[arg(short, long)]
        encrypt: bool,

        /// Store backups in snapshot history mode (keeps deleted files) instead of default one-way sync
        #[arg(short, long)]
        snapshot: bool,

        /// Encryption password (if encrypt is true; will be stored in OS keyring if provided)
        #[arg(long)]
        password: Option<String>,
    },
    /// Restore archived files of a repository to a target location
    Restore {
        /// Name of the repository to restore
        #[arg(short, long)]
        repo: String,

        /// Destination path to restore the files
        #[arg(short, long)]
        dest: String,

        /// Specific archive session ID (omitting restores the latest session)
        #[arg(short, long)]
        archive_id: Option<i64>,

        /// Decryption password (if the archive is encrypted)
        #[arg(short, long)]
        password: Option<String>,
    },
    /// Display status of registered repositories and latest backups
    Status,
    /// Unregister a repository, delete its DB records and backup files
    Unregister {
        /// Name of the repository to unregister
        #[arg(short, long)]
        repo: String,
    },
}

fn main() {
    // Print banner if run with no arguments or help/version flags
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 || args.iter().any(|arg| arg == "--help" || arg == "-h" || arg == "--version" || arg == "-V") {
        print_banner();
    }

    // Step 0: Parse command-line args immediately (handles --version and --help before config/db load)
    let cli = Cli::parse();

    // Step 1: Load configurations
    let mut config = match config::load_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error loading configuration: {}", e);
            std::process::exit(1);
        }
    };

    // Step 2: If target_dir is not set, prompt user in interactive setup
    if config.target_dir.is_empty() {
        println!("Welcome to LoAr (Local Archive)!");
        println!("Let's perform the initial setup first.\n");
        let target: String = Input::with_theme(&custom_theme())
            .with_prompt("Enter the absolute path for your central archive folder (somewhere safe)")
            .validate_with(|input: &String| {
                let cleaned = input.trim_matches(|c| c == '\'' || c == '"');
                let path = Path::new(cleaned);
                if path.is_absolute() {
                    Ok(())
                } else {
                    Err("Please enter a valid absolute path.")
                }
            })
            .interact_text()
            .unwrap();

        config.target_dir = target.trim_matches(|c| c == '\'' || c == '"').to_string();
        if let Err(e) = config::save_config(&config) {
            eprintln!("Failed to save initial configuration: {}", e);
            std::process::exit(1);
        }
        println!("Initial setup completed! Settings stored.\n");
    }

    // Step 3: Open SQLite Database
    let db = match DbConnection::open(&config.target_dir) {
        Ok(connection) => connection,
        Err(e) => {
            eprintln!("Database initialization failed: {}", e);
            std::process::exit(1);
        }
    };

    // 1. Sync missing repositories from SQLite DB back to config TOML (For portability)
    let mut config_updated = false;
    if let Ok(db_repos) = db.list_repositories() {
        for db_repo in db_repos {
            if !config.repositories.iter().any(|r| r.path == db_repo.path) {
                config.repositories.push(RepositoryConfig {
                    name: db_repo.name.clone(),
                    path: db_repo.path.clone(),
                    encrypt: db_repo.encrypt,
                    one_way_sync: db_repo.one_way_sync,
                });
                config_updated = true;
            }
        }
    }

    // 2. Auto-sync missing repositories from config TOML to SQLite DB
    for repo_cfg in &config.repositories {
        if let Ok(None) = db.get_repository_by_path(&repo_cfg.path) {
            if let Err(e) = db.add_repository(&repo_cfg.name, &repo_cfg.path, repo_cfg.encrypt, repo_cfg.one_way_sync) {
                eprintln!("Warning: Failed to sync repo '{}' from config to DB: {}", repo_cfg.name, e);
            }
        }
    }

    if config_updated {
        if let Err(e) = config::save_config(&config) {
            eprintln!("Warning: Failed to save config after syncing from DB: {}", e);
        }
    }

    // Step 4: Process command-line args

    match cli.command {
        Some(Commands::Run { repo, password }) => {
            if let Err(e) = handle_run_cmd(&config, &db, repo.as_deref(), password.as_deref()) {
                eprintln!("Backup failed: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Register { name, path, encrypt, snapshot, password }) => {
            let one_way_sync = !snapshot;
            if let Err(e) = handle_register_cmd(&mut config, &db, &name, &path, encrypt, one_way_sync, password.as_deref()) {
                eprintln!("Registration failed: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Restore { repo, dest, archive_id, password }) => {
            if let Err(e) = handle_restore_cmd(&config, &db, &repo, &dest, archive_id, password.as_deref()) {
                eprintln!("Restore failed: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Status) => {
            if let Err(e) = handle_status_cmd(&db) {
                eprintln!("Failed to show status: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Unregister { repo }) => {
            if let Err(e) = handle_unregister_cmd(&mut config, &db, &repo) {
                eprintln!("Unregistration failed: {}", e);
                std::process::exit(1);
            }
        }
        None => {
            // Interactive CLI Loop
            run_interactive_menu(&mut config, &db);
        }
    }
}

// Command handlers
fn handle_run_cmd(
    config: &AppConfig,
    db: &DbConnection,
    repo_name: Option<&str>,
    password: Option<&str>,
) -> Result<(), String> {
    let repos_in_db = db.list_repositories()?;

    if repos_in_db.is_empty() {
        return Err("No repositories registered yet. Use 'loar register' or interactive menu.".to_string());
    }

    let target_repos = if let Some(name) = repo_name {
        let found = repos_in_db.iter().find(|r| r.name == name);
        match found {
            Some(r) => vec![r],
            None => return Err(format!("Repository '{}' not found in database", name)),
        }
    } else {
        repos_in_db.iter().collect::<Vec<_>>()
    };

    for repo in target_repos {
        println!("Archiving repository: {} ({})", repo.name, repo.path);
        
        let pwd = if repo.encrypt {
            let p = password.map(String::from)
                .or_else(|| crypto::get_stored_password(&repo.name));
            
            if p.is_none() {
                return Err(format!("Encryption password is required for '{}' but not provided or stored.", repo.name));
            }
            p
        } else {
            None
        };

        match archive::run_backup(repo, &config.target_dir, &config.global_exclude, db, pwd.as_deref()) {
            Ok(msg) => println!("Success: {}", msg),
            Err(e) => eprintln!("Error archiving '{}': {}", repo.name, e),
        }
        println!();
    }

    Ok(())
}

fn handle_register_cmd(
    config: &mut AppConfig,
    db: &DbConnection,
    name: &str,
    path: &str,
    encrypt: bool,
    one_way_sync: bool,
    password: Option<&str>,
) -> Result<(), String> {
    let canonical_path = Path::new(path).canonicalize()
        .map_err(|e| format!("Invalid absolute path '{}': {}", path, e))?;
    let path_str = canonical_path.to_string_lossy().to_string();

    // Check duplication in DB
    if let Some(existing) = db.get_repository_by_path(&path_str)? {
        return Err(format!("Repository path already registered under name '{}'", existing.name));
    }

    // SQLite DB Save
    db.add_repository(name, &path_str, encrypt, one_way_sync)?;

    // config TOML update
    config.repositories.push(RepositoryConfig {
        name: name.to_string(),
        path: path_str.clone(),
        encrypt,
        one_way_sync,
    });
    config::save_config(config)?;

    // If password provided and encrypt is enabled, store in Keyring
    if encrypt {
        if let Some(pwd) = password {
            crypto::store_password(name, pwd)?;
            println!("Encryption password successfully stored in OS Keyring.");
        } else {
            println!("Warning: AES-256 encryption is enabled, but no keyring password was saved.");
        }
    }

    println!("Repository '{}' successfully registered.", name);
    Ok(())
}

fn handle_restore_cmd(
    config: &AppConfig,
    db: &DbConnection,
    repo_name: &str,
    dest_path: &str,
    archive_id: Option<i64>,
    password: Option<&str>,
) -> Result<(), String> {
    let repos = db.list_repositories()?;
    let repo = repos.iter().find(|r| r.name == repo_name)
        .ok_or_else(|| format!("Repository '{}' not registered in database", repo_name))?;

    let session_id = match archive_id {
        Some(id) => id,
        None => {
            let latest = db.get_latest_archive(repo.id)?
                .ok_or_else(|| format!("No archive history found for '{}'", repo_name))?;
            latest.id
        }
    };

    let pwd = if repo.encrypt {
        let p = password.map(String::from)
            .or_else(|| crypto::get_stored_password(&repo.name));
        
        if p.is_none() {
            return Err("Decryption password is required but not provided or stored.".to_string());
        }
        p
    } else {
        None
    };

    let msg = archive::run_restore(repo, session_id, dest_path, &config.target_dir, db, pwd.as_deref())?;
    println!("{}", msg);
    Ok(())
}

fn handle_status_cmd(db: &DbConnection) -> Result<(), String> {
    let repos = db.list_repositories()?;
    if repos.is_empty() {
        println!("No repositories registered yet.");
        return Ok(());
    }

    println!("\x1b[1;36mRegistered Repositories Status\x1b[0m");
    println!();
    
    for r in repos {
        let last_backup = match db.get_latest_archive(r.id)? {
            Some(arc) => arc.timestamp,
            None => "Never".to_string(),
        };

        let formatted_backup = if last_backup != "Never" {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&last_backup) {
                dt.format("%Y-%m-%dT%H:%M:%S%:z").to_string()
            } else {
                last_backup
            }
        } else {
            last_backup
        };

        let mode_str = if r.one_way_sync { "One-way Sync" } else { "Snapshot" };
        println!("  {:<11}: \x1b[1;36m{}\x1b[0m", "Project", r.name);
        println!("  {:<11}: {}", "Path", r.path);
        println!("  {:<11}: {}", "Encrypted", r.encrypt);
        println!("  {:<11}: {}", "Mode", mode_str);
        println!("  {:<11}: {}", "Last Backup", formatted_backup);
        println!();
    }
    Ok(())
}

fn handle_unregister_cmd(
    config: &mut AppConfig,
    db: &DbConnection,
    repo_name: &str,
) -> Result<(), String> {
    let repos = db.list_repositories()?;
    let repo = repos.iter().find(|r| r.name == repo_name)
        .ok_or_else(|| format!("Repository '{}' not registered in database", repo_name))?;

    // 1. Delete physical backup directory
    let target_base = Path::new(&config.target_dir).join(&repo.name);
    if target_base.exists() {
        std::fs::remove_dir_all(&target_base)
            .map_err(|e| format!("Failed to delete backup files in '{}': {}", target_base.display(), e))?;
        println!("Deleted physical backup files in '{}'.", target_base.display());
    }

    // 2. Delete database records (Cascades to archives and file_records)
    db.delete_repository(repo.id)?;
    println!("Deleted database records for repository '{}'.", repo.name);

    // 3. Delete from keyring
    if let Err(e) = crypto::delete_stored_password(&repo.name) {
        eprintln!("Warning: Failed to delete keyring password for '{}': {}", repo.name, e);
    }

    // 4. Remove from config.toml
    config.repositories.retain(|r| r.name != repo_name);
    config::save_config(config)?;
    println!("Removed repository '{}' from configuration.", repo_name);

    Ok(())
}

// Interactive TUI Menu Loop
fn run_interactive_menu(config: &mut AppConfig, db: &DbConnection) {
    let selections = &[
        "Find Repositories",
        "Register a Repository",
        "Run Backup",
        "Browse & Restore archives",
        "Show Status",
        "Unregister a Repository",
        "Exit",
    ];

    loop {
        println!("Local Archive v{}\n", env!("CARGO_PKG_VERSION"));
        let selection = Select::with_theme(&custom_theme())
            .with_prompt("Select menu")
            .default(0)
            .items(&selections[..])
            .interact()
            .unwrap();

        match selection {
            0 => {
                // Find Repositories
                let scan_dir_str: String = match Input::<String>::new()
                    .with_prompt("Enter root directory to scan for Git repositories")
                    .interact_text()
                {
                    Ok(path) => path.trim_matches(|c| c == '\'' || c == '"').to_string(),
                    Err(e) => {
                        eprintln!("Error reading input: {}", e);
                        continue;
                    }
                };

                let scan_path = Path::new(&scan_dir_str);
                println!("Scanning directories (this may take a few seconds)...");
                match scanner::find_git_repositories(scan_path) {
                    Ok(repo_paths) => {
                        if repo_paths.is_empty() {
                            println!("No Git repositories found in target directory.");
                            continue;
                        }

                        // Create a set of canonicalized paths of already registered repositories
                        let registered_paths: std::collections::HashSet<String> = config.repositories.iter()
                            .map(|r| {
                                Path::new(&r.path)
                                    .canonicalize()
                                    .ok()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_else(|| r.path.clone())
                            })
                            .collect();

                        println!("\nFound {} Git repositories:", repo_paths.len());
                        
                        let repo_names: Vec<String> = repo_paths.iter().map(|p| {
                            let canonical_str = p.canonicalize()
                                .ok()
                                .map(|canonical| canonical.to_string_lossy().to_string())
                                .unwrap_or_else(|| p.to_string_lossy().to_string());

                            if registered_paths.contains(&canonical_str) {
                                format!("{} (registered already)", p.display())
                            } else {
                                p.to_string_lossy().to_string()
                            }
                        }).collect();
                        
                        let checked = MultiSelect::with_theme(&custom_theme())
                            .with_prompt("Select repositories to register (Space to select, Enter to confirm)")
                            .items(&repo_names)
                            .interact()
                            .unwrap();

                        if checked.is_empty() {
                            println!("No repositories selected. Registration cancelled.");
                            continue;
                        }

                        for idx in checked {
                            let path = &repo_paths[idx];
                            
                            let canonical_str = path.canonicalize()
                                .ok()
                                .map(|canonical| canonical.to_string_lossy().to_string())
                                .unwrap_or_else(|| path.to_string_lossy().to_string());

                            if registered_paths.contains(&canonical_str) {
                                println!("\nSkipping '{}': already registered.", path.display());
                                continue;
                            }

                            println!("\n--- Setting up repository: {} ---", path.display());

                            // Use the repository folder name as the default project name
                            let default_name = path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("unnamed");

                            let use_default = Confirm::new()
                                .with_prompt(format!("Use default project name '{}'?", default_name))
                                .default(true)
                                .interact()
                                .unwrap();

                            let name = if use_default {
                                default_name.to_string()
                            } else {
                                Input::new()
                                    .with_prompt("Enter custom project name")
                                    .interact_text()
                                    .unwrap()
                            };

                            let encrypt = Confirm::new()
                                .with_prompt("Encrypt this archive?")
                                .default(false)
                                .interact()
                                .unwrap();

                            let mode_selections = &["One-way Sync (Reflect local deletions - Default)", "Snapshot (Keep history)"];
                            let mode_idx = Select::with_theme(&custom_theme())
                                .with_prompt("Select Backup Mode")
                                .default(0)
                                .items(&mode_selections[..])
                                .interact()
                                .unwrap();
                            let one_way_sync = mode_idx == 0;

                            let mut password = None;
                            if encrypt {
                                let pwd1: String = Input::new().with_prompt("Enter encryption password").interact_text().unwrap();
                                let save_keyring = Confirm::new().with_prompt("Store password securely in OS Keyring?").default(true).interact().unwrap();
                                if save_keyring {
                                    password = Some(pwd1);
                                }
                            }

                            match handle_register_cmd(config, db, &name, &path.to_string_lossy(), encrypt, one_way_sync, password.as_deref()) {
                                Ok(_) => {}
                                Err(e) => eprintln!("Error: {}", e),
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error scanning directory: {}", e);
                    }
                }
            }
            1 => {
                // Register
                let name: String = Input::new().with_prompt("Repository name").interact_text().unwrap();
                let path_str: String = Input::new().with_prompt("Absolute source directory").interact_text().unwrap();
                let path_str = path_str.trim_matches(|c| c == '\'' || c == '"').to_string();
                let encrypt = Confirm::new().with_prompt("Encrypt this archive?").default(false).interact().unwrap();
                
                // Select backup mode
                let mode_selections = &["One-way Sync (Reflect local deletions - Default)", "Snapshot (Keep history)"];
                let mode_idx = Select::with_theme(&custom_theme())
                    .with_prompt("Select Backup Mode")
                    .default(0)
                    .items(&mode_selections[..])
                    .interact()
                    .unwrap();
                let one_way_sync = mode_idx == 0;
                
                let mut password = None;
                if encrypt {
                    let pwd1: String = Input::new().with_prompt("Enter encryption password").interact_text().unwrap();
                    let save_keyring = Confirm::new().with_prompt("Store password securely in OS Keyring?").default(true).interact().unwrap();
                    if save_keyring {
                        password = Some(pwd1);
                    }
                }

                match handle_register_cmd(config, db, &name, &path_str, encrypt, one_way_sync, password.as_deref()) {
                    Ok(_) => {}
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            2 => {
                // Run backup
                let repos = match db.list_repositories() {
                    Ok(list) => list,
                    Err(e) => {
                        eprintln!("Error loading repositories: {}", e);
                        continue;
                    }
                };

                if repos.is_empty() {
                    println!("No repositories registered. Please register one first.");
                    continue;
                }

                let repo_names = repos.iter().map(|r| r.name.as_str()).collect::<Vec<_>>();
                let checked = MultiSelect::with_theme(&custom_theme())
                    .with_prompt("Select repositories to backup (Space to select, Enter to confirm)")
                    .items(&repo_names)
                    .interact()
                    .unwrap();

                for idx in checked {
                    let repo = &repos[idx];
                    println!("\nArchiving repository: {} ({})", repo.name, repo.path);
                    
                    let mut pwd = None;
                    if repo.encrypt {
                        pwd = crypto::get_stored_password(&repo.name);
                        if pwd.is_none() {
                            let pwd_input: String = Input::new()
                                .with_prompt(format!("Enter password for '{}'", repo.name))
                                .interact_text()
                                .unwrap();
                            
                            let save_keyring = Confirm::new()
                                .with_prompt("Save this password to OS Keyring?")
                                .default(true)
                                .interact()
                                .unwrap();
                            
                            if save_keyring {
                                let _ = crypto::store_password(&repo.name, &pwd_input);
                            }
                            pwd = Some(pwd_input);
                        }
                    }

                    match archive::run_backup(repo, &config.target_dir, &config.global_exclude, db, pwd.as_deref()) {
                        Ok(msg) => println!("Success: {}", msg),
                        Err(e) => eprintln!("Error: {}", e),
                    }
                }
            }
            3 => {
                // Browse & Restore
                let repos = db.list_repositories().unwrap_or_default();
                if repos.is_empty() {
                    println!("No repositories registered.");
                    continue;
                }

                let mut repo_names = repos.iter().map(|r| r.name.as_str()).collect::<Vec<_>>();
                repo_names.push("<- Cancel (Go Back)");

                let repo_idx = Select::with_theme(&custom_theme())
                    .with_prompt("Select repository to restore")
                    .items(&repo_names)
                    .interact()
                    .unwrap();

                if repo_idx == repos.len() {
                    println!("Restore cancelled.");
                    continue;
                }
                let repo = &repos[repo_idx];

                let latest_arc = db.get_latest_archive(repo.id).unwrap_or_default();
                if latest_arc.is_none() {
                    println!("No backups found for '{}'", repo.name);
                    continue;
                }

                let arc = latest_arc.unwrap();
                println!("Latest backup details: Session ID: {}, Timestamp: {}, Files: {}, Size: {} bytes", 
                    arc.id, arc.timestamp, arc.file_count, arc.total_size);

                let dest: String = Input::new().with_prompt("Enter restore destination directory").interact_text().unwrap();
                
                let mut pwd = None;
                if repo.encrypt {
                    pwd = crypto::get_stored_password(&repo.name);
                    if pwd.is_none() {
                        let pwd_input: String = Input::new()
                            .with_prompt(format!("Enter decryption password for '{}'", repo.name))
                            .interact_text()
                            .unwrap();
                        pwd = Some(pwd_input);
                    }
                }

                match archive::run_restore(repo, arc.id, &dest, &config.target_dir, db, pwd.as_deref()) {
                    Ok(msg) => println!("Success: {}", msg),
                    Err(e) => eprintln!("Error: {}", e),
                }
            }
            4 => {
                // Status
                let _ = handle_status_cmd(db);
            }
            5 => {
                // Unregister
                let repos = match db.list_repositories() {
                    Ok(list) => list,
                    Err(e) => {
                        eprintln!("Error loading repositories: {}", e);
                        continue;
                    }
                };

                if repos.is_empty() {
                    println!("No repositories registered.");
                    continue;
                }

                let mut repo_names = repos.iter().map(|r| r.name.as_str()).collect::<Vec<_>>();
                repo_names.push("<- Cancel (Go Back)");

                let repo_idx = Select::with_theme(&custom_theme())
                    .with_prompt("Select repository to unregister")
                    .items(&repo_names)
                    .interact()
                    .unwrap();

                if repo_idx == repos.len() {
                    println!("Unregistration cancelled.");
                    continue;
                }
                let repo = &repos[repo_idx];

                let confirm = Confirm::new()
                    .with_prompt(format!("Are you sure you want to unregister '{}'? This will DELETE ALL backup files and DB records!", repo.name))
                    .default(false)
                    .interact()
                    .unwrap();

                if confirm {
                    match handle_unregister_cmd(config, db, &repo.name) {
                        Ok(_) => println!("Successfully unregistered '{}'.", repo.name),
                        Err(e) => eprintln!("Error: {}", e),
                    }
                } else {
                    println!("Unregistration cancelled.");
                }
            }
            _ => {
                break;
            }
        }
    }
}
