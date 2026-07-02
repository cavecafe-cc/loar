use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use ignore::WalkBuilder;

const DEFAULT_IGNORE_TEMPLATE: &str = r"# LoAr Default Exclusions
# Files matching these patterns will NEVER be archived, regardless of git status.
# Feel free to edit this file. Lines starting with '#' are comments.

# OS Metadata
.DS_Store
Thumbs.db
desktop.ini
ehthumbs.db
.Spotlight-V100
.Trashes

# Common Build Output & Dependencies
node_modules/
target/
bin/
obj/
build/
dist/
out/
__pycache__/
venv/
.venv/
env/
.gradle/
.dart_tool/
vendor/
.next/
.nuxt/
.svelte-kit/
CMakeFiles/
*.o
*.class
*.exe
*.dll
*.pyc
*.pyo

# Logs & Diagnostics
*.log
npm-debug.log*
yarn-debug.log*
yarn-error.log*

# Database & Temp Files
*.tmp
*.temp
*.db-shm
*.db-wal

# Xcode Metadata & Build Artifacts
DerivedData/
*.xcodeproj/xcuserdata/
*.xcworkspace/xcuserdata/
*.xcodeproj/project.xcworkspace/xcuserdata/
ephemeral/
Pods/

# Generated build files & tools
gradle-wrapper.jar
GeneratedPluginRegistrant.*
Generated.xcconfig
generated_plugin*

# IDE Settings
.idea/
.vscode/
*.suo
*.user
*.ntvs*
*.njsproj
*.sln.docstates
";

/// Automatically create a default .loar.ignore if it doesn't exist.
pub fn ensure_loar_ignore(repo_path: &Path) -> Result<(), String> {
    let ignore_path = repo_path.join(".loar.ignore");
    if !ignore_path.exists() {
        fs::write(&ignore_path, DEFAULT_IGNORE_TEMPLATE)
            .map_err(|e| format!("Failed to create default .loar.ignore: {}", e))?;
    }
    Ok(())
}

/// Parse the .loar.ignore file using the ignore crate's GitignoreBuilder.
pub fn load_loar_ignore(repo_path: &Path, global_exclude: &[String]) -> Result<Gitignore, String> {
    ensure_loar_ignore(repo_path)?;
    let ignore_path = repo_path.join(".loar.ignore");
    let mut builder = GitignoreBuilder::new(repo_path);

    // 1. Add global excludes first
    for pattern in global_exclude {
        builder.add_line(None, pattern)
            .map_err(|e| format!("Failed to parse global exclude pattern '{}': {}", pattern, e))?;
    }

    let content = fs::read_to_string(&ignore_path)
        .map_err(|e| format!("Failed to read .loar.ignore: {}", e))?;

    for line in content.lines() {
        let trimmed = line.trim();
        // Skip comments and empty lines
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }
        builder.add_line(None, trimmed)
            .map_err(|e| format!("Failed to parse line '{}' in .loar.ignore: {}", trimmed, e))?;
    }

    let gitignore = builder.build()
        .map_err(|e| format!("Failed to build ignore filter: {}", e))?;

    Ok(gitignore)
}

/// Retrieve the set of Git tracked files relative to the repository path.
/// Returns an Err if the folder is not a git repo or git CLI is missing.
pub fn get_git_tracked_files(repo_path: &Path) -> Result<HashSet<String>, String> {
    let output = Command::new("git")
        .args(["ls-files", "-c", "-d", "-m"])
        .current_dir(repo_path)
        .output();

    match output {
        Ok(out) => {
            if !out.status.success() {
                let err_msg = String::from_utf8_lossy(&out.stderr);
                return Err(format!("git ls-files failed: {}", err_msg.trim()));
            }
            let files_str = String::from_utf8_lossy(&out.stdout);
            let mut set = HashSet::new();
            for line in files_str.lines() {
                if !line.trim().is_empty() {
                    set.insert(line.trim().to_string());
                }
            }
            Ok(set)
        }
        Err(e) => Err(format!("Failed to execute git command: {}", e)),
    }
}

/// Scans the target folder and filters out symlinks, special files,
/// loar ignore patterns, and git tracked files (dynamically detected for sub-repositories).
/// Returns a list of relative paths for files that should be archived.
pub fn scan_folder(repo_path: &Path, global_exclude: &[String]) -> Result<Vec<PathBuf>, String> {
    let repo_path = fs::canonicalize(repo_path)
        .map_err(|e| format!("Invalid repository path: {}", e))?;

    let loar_ignore = load_loar_ignore(&repo_path, global_exclude)?;
    
    let mut files_to_archive = Vec::new();
    
    // Initial git context for the root folder
    let initial_git_context = if repo_path.join(".git").exists() {
        get_git_tracked_files(&repo_path).ok().map(|set| (repo_path.clone(), set))
    } else {
        None
    };

    // Recursive directory traversal with whitelist logic
    // We only accept normal files and directories, ignoring symlinks and special files
    fn traverse(
        dir: &Path,
        root: &Path,
        loar_ignore: &Gitignore,
        git_context: Option<(PathBuf, HashSet<String>)>,
        results: &mut Vec<PathBuf>,
    ) -> Result<(), String> {
        // Detect git repository dynamically inside subdirectories
        let mut current_git_context = git_context;
        if dir != root && dir.join(".git").exists() {
            if let Ok(tracked_set) = get_git_tracked_files(dir) {
                current_git_context = Some((dir.to_path_buf(), tracked_set));
            }
        }

        let entries = fs::read_dir(dir)
            .map_err(|e| format!("Failed to read directory '{}': {}", dir.display(), e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Error reading entry: {}", e))?;
            let path = entry.path();
            let metadata = entry.metadata()
                .map_err(|e| format!("Failed to read metadata for '{}': {}", path.display(), e))?;
            let file_type = metadata.file_type();

            // Whitelist check: strictly ignore symlinks and special OS files
            if file_type.is_symlink() || (!file_type.is_file() && !file_type.is_dir()) {
                continue;
            }

            let relative_path = path.strip_prefix(root)
                .map_err(|e| format!("Path mapping error: {}", e))?;
            
            // Check loar ignore rule (Absolute exclusion)
            let is_dir = file_type.is_dir();
            if loar_ignore.matched(relative_path, is_dir).is_ignore() {
                continue;
            }

            // Exclude .loar.ignore itself, along with loar config files from backing up
            let rel_str = relative_path.to_string_lossy();
            if rel_str == ".loar.ignore" || rel_str == "loar.toml" || rel_str == "loar.db" {
                continue;
            }

            if file_type.is_dir() {
                // Ignore any .git directory recursively
                if path.file_name().map(|n| n == ".git").unwrap_or(false) {
                    continue;
                }
                traverse(&path, root, loar_ignore, current_git_context.clone(), results)?;
            } else if file_type.is_file() {
                // If there's a git context, verify if it's tracked
                if let Some((ref git_root, ref tracked_set)) = current_git_context {
                    if let Ok(rel_to_git) = path.strip_prefix(git_root) {
                        let rel_to_git_str = rel_to_git.to_string_lossy().to_string();
                        if tracked_set.contains(&rel_to_git_str) {
                            // Skip git tracked file
                            continue;
                        }
                    }
                }
                results.push(relative_path.to_path_buf());
            }
        }
        Ok(())
    }

    traverse(&repo_path, &repo_path, &loar_ignore, initial_git_context, &mut files_to_archive)?;
    Ok(files_to_archive)
}

/// Scans the target folder recursively and finds all Git repositories.
/// Nested repositories are ignored (once a git repository is detected, subdirectories are not scanned).
pub fn find_git_repositories(scan_path: &Path) -> Result<Vec<PathBuf>, String> {
    let scan_path = fs::canonicalize(scan_path)
        .map_err(|e| format!("Invalid scan path: {}", e))?;

    let mut repos = Vec::new();
    let walk = WalkBuilder::new(&scan_path)
        .standard_filters(true) // respects .gitignore, .ignore
        .hidden(true) // skip hidden directories (e.g., .config, .cache)
        .build();

    let mut skip_prefixes: Vec<PathBuf> = Vec::new();

    for entry in walk {
        if let Ok(entry) = entry {
            let path = entry.path();

            // Skip if this path starts with any already detected git repository
            if skip_prefixes.iter().any(|prefix| path.starts_with(prefix)) {
                continue;
            }

            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                if path.join(".git").exists() {
                    repos.push(path.to_path_buf());
                    skip_prefixes.push(path.to_path_buf());
                }
            }
        }
    }

    Ok(repos)
}
