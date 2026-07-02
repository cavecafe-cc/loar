# LoAr (Local Archive Utility)

`LoAr` is a local archiving and synchronization tool written in Rust, designed to back up multiple software development projects and directories scattered across your system into a single backup destination safely and efficiently.

It optimizes storage space and enhances security by combining **Incremental Skip**, **One-way Sync**, **AES-256-GCM encryption**, and **SQLite metadata indexing**.

---

## Key Features

1. **High-Speed Incremental Skip**
   - Compares file size, modification time (mtime), and SHA-256 hash to automatically skip backup copying (and encryption) for unchanged files.
   - Saves extreme I/O operations, completing synchronization for gigabyte-scale repositories in just a few seconds.
2. **One-Way Sync & Auto Cleanup**
   - Automatically deletes backed-up files from the target drive if they are deleted in the source directory, keeping your backup in perfect synchronization.
   - Cleans up empty folders or leftover directories with only macOS metadata (like `.DS_Store`) using a bottom-up Depth-First Search (DFS) cleanup algorithm.
3. **AES-256-GCM Secure Encryption**
   - Allows independent encryption configuration (`encrypt = true`) per repository.
   - Uses Argon2id for key derivation and AES-256-GCM to obfuscate filenames and securely encrypt all file contents.
4. **Global Backup Database (SQLite DB)**
   - Manages archive histories and file record indices in an SQLite database file (`loar.db`) located at the root of the target directory.
   - Optimizes database physical file size automatically using the `VACUUM` command whenever old backup sessions are pruned.
5. **Dual Interactive TUI & CLI Modes**
   - Provides an interactive Terminal User Interface (TUI) menu for easy backup and restore management.
   - Supports a fully non-interactive CLI mode suitable for automated cron jobs or scheduler tasks.

---

## Installation

`LoAr` can be installed in four ways: via a platform-detecting installer script (recommended), Homebrew (for macOS), Scoop (for Windows), direct package download, or building from source.

### 1. Using One-Click Script Installer (All Platforms - Recommended)
You can install `LoAr` with a single command. The script automatically detects your OS and architecture, downloads the latest matching release, and configures the executable paths.

*   **Linux / macOS**:
    ```bash
    curl -fsSL https://raw.githubusercontent.com/cavecafe-cc/homebrew-tap/main/install.sh | sh
    ```

*   **Windows (PowerShell)**:
    ```powershell
    irm https://raw.githubusercontent.com/cavecafe-cc/homebrew-tap/main/install.ps1 | iex
    ```

### 2. Using Homebrew (macOS)
You can easily install `LoAr` on macOS using our Homebrew tap:

```bash
brew tap cavecafe-cc/homebrew-tap
brew install loar
```

### 3. Using Scoop (Windows)
You can easily install `LoAr` on Windows using `Scoop` with a single command:

```powershell
scoop install https://raw.githubusercontent.com/cavecafe-cc/local-archive/main/scoop/loar.json
```

### 4. Using APT (Linux / Debian / Ubuntu)
You can install `LoAr` using the official Debian/APT repository hosted on Cloudflare R2:

```bash
# 1. Import the repository public GPG key
curl -fsSL https://bin.cavecafe.cc/downloads/loar/apt/gpg.key | sudo gpg --dearmor -o /etc/apt/trusted.gpg.d/cavecafe-cc.gpg

# 2. Add the APT source list entry
echo "deb [arch=amd64,arm64] https://bin.cavecafe.cc/downloads/loar/apt stable main" | sudo tee /etc/apt/sources.list.d/loar.list

# 3. Update index and install the package
sudo apt-get update
sudo apt-get install loar
```

### 5. Using Snapcraft (Linux Universal)
`LoAr` is published as a Snap package. You can install it on any Snap-supported Linux distribution:

```bash
sudo snap install loar
```

> [!IMPORTANT]
> **Post-installation Setup (Interface Connections)**
> Because the Snap version runs in a secure sandbox (strict confinement), you must manually connect the required interfaces depending on your usage:
> 
> *   **External storage backup/restore** (to access `/media` or `/mnt`):
>     ```bash
>     sudo snap connect loar:removable-media
>     ```
> *   **Store encryption passwords securely** (OS Keyring integration):
>     ```bash
>     sudo snap connect loar:password-manager-service
>     ```
> 
> Note: Standard user folders and nested project directories (including hidden files like `.env`) within your `$HOME` directory are fully accessible out of the box once the `home` interface is connected.

### 6. Direct Package Downloads (Pre-built Binaries)
You can directly download the pre-built binaries hosted on Cloudflare R2. Replace `v1.0.28` in the URL with the desired target version if installing a different version.

*   **macOS (Apple Silicon arm64)**:
    [loar-macos-arm64-latest.tar.gz](https://bin.cavecafe.cc/downloads/loar/loar-macos-arm64-latest.tar.gz)
*   **Linux (Debian x86_64)**:
    [loar-linux-x86_64-latest.deb](https://bin.cavecafe.cc/downloads/loar/loar-linux-x86_64-latest.deb)
*   **Linux (Debian ARM64)**:
    [loar-linux-arm64-latest.deb](https://bin.cavecafe.cc/downloads/loar/loar-linux-arm64-latest.deb)
*   **Windows (x86_64)**:
    [loar-windows-x86_64-latest.zip](https://bin.cavecafe.cc/downloads/loar/loar-windows-x86_64-latest.zip)
*   **Windows (ARM64)**:
    [loar-windows-arm64-latest.zip](https://bin.cavecafe.cc/downloads/loar/loar-windows-arm64-latest.zip)

#### Installation Steps for Debian (.deb)
```bash
# Example: Download and install the latest x86_64 package
curl -LO https://bin.cavecafe.cc/downloads/loar/loar-linux-x86_64-latest.deb
sudo dpkg -i loar-linux-x86_64-latest.deb
```

#### Installation Steps for Windows (.zip)
Unzip the package and move `loar.exe` to a folder registered in your system PATH environment variable (e.g., `C:\Windows\System32` or a custom execution folder).

---

### 3. Build from Source (Cargo)
If you prefer building from source, ensure you have [Rust / Cargo](https://rustup.rs/) installed.

```bash
# 1. Navigate to the project 'src' directory
cd src

# 2. Build with the release profile
cargo build --release

# 3. Copy the compiled binary to your PATH
cp target/release/loar ~/.cargo/bin/loar
```

---

## Configuration

`LoAr` is configured via `~/.loar/loar.toml` in your home directory, defining target backup paths, global exclusions, and directories to sync.

### Configuration Template (`~/.loar/loar.toml`)

```toml
# Destination folder where all backups are copied and stored
target_dir = "/Volumes/Backup/LoAr"

# Global exclude patterns applied to all repositories (Gitignore syntax)
global_exclude = [
    ".DS_Store",
    "node_modules/",
    "target/",
    "bin/",
    "obj/",
    ".idea/",
    ".vscode/",
    "DerivedData/",
    "*.xcodeproj/xcuserdata/",
    "*.xcworkspace/xcuserdata/",
    "*.xcodeproj/project.xcworkspace/xcuserdata/",
    "ephemeral/",
    "Pods/",
    "gradle-wrapper.jar",
    "GeneratedPluginRegistrant.*",
    "Generated.xcconfig",
    "generated_plugin*",
]

# List of repositories to sync
[[repositories]]
name = "LoAr"                                 # Target backup folder name identifier
path = "/Users/username/Projects/LoAr"        # Absolute path to local source repository
encrypt = false                              # Encryption disabled
one_way_sync = true                          # Automatically sync deletions

[[repositories]]
name = "Super7"
path = "/Users/username/Repo/GitHub/Super7"
encrypt = false
one_way_sync = true

[[repositories]]
name = "Private-Repo"
path = "/Users/username/Projects/Secret"
encrypt = true                               # Encryption enabled
one_way_sync = true
```

---

## Usage Guide

### 1. Interactive Terminal UI Mode (TUI Mode)
Run `loar` without arguments to launch the TUI menu console.

```bash
$ loar
```
Navigate with arrow keys and use Space (select/deselect) and Enter to execute tasks:
- **1. List Repositories**: View currently registered repositories and status.
- **2. Run Backup**: Select and run incremental backups.
- **3. Register Repository**: Add a new repository to sync interactively.
- **4. Restore Backup**: Restore selected backup sessions to a specified directory.
- **5. Exit**: Close the utility.

---

### 2. Command Line Interface Mode (CLI Mode)

#### 1) Back Up All Repositories
Runs incremental backups for all registered repositories sequentially (ideal for cron jobs).
```bash
$ loar run --all
```

#### 2) Back Up a Specific Repository
Specify a repository name (`name` in the TOML configuration) to back up.
```bash
$ loar run --repo Super7
```
*For encrypted repositories, you can bypass the interactive password prompt by passing `--password`:*
```bash
$ loar run --repo Private-Repo --password "your_secure_password"
```

#### 3) Check Backup Status
Displays registered repositories, encryption states, sync modes, and the last backup timestamps.
```bash
$ loar status
```

#### 4) Register a Repository Manually
Register a new repository directly via the command line.
```bash
$ loar register --name "WWW" --path "/path/to/www" --encrypt
```
- `--name`: Backup folder name identifier (Required)
- `--path`: Source directory absolute path (Required)
- `--encrypt`: Enable AES-256-GCM encryption (Optional)
- `--no-sync`: Disable One-way Sync and keep history snapshots instead (Optional)

#### 5) Restore a Backup
Restore backed-up files from a specific session to a local directory.
```bash
$ loar restore --repo Super7 --dest /Users/username/Restored/Super7
```

#### 6) Unregister a Repository
Unregister a repository, deleting its database history and backed-up files safely.
```bash
$ loar unregister --repo Super7
```
- `--repo`: Name of the repository to unregister (Required)

---

## Backup Excludes (`.loar.ignore`)

`LoAr` applies global excludes (`global_exclude`) first, then prioritizes a local `.loar.ignore` file generated at the root of each repository.

### 1. Ignore Behavior
- A `.loar.ignore` file is automatically created in the source folder root when backing up a repository for the first time.
- Any files or folders matching Gitignore-style patterns inside this file are excluded from archiving.

### 2. Default Exclusions Template
By default, the following files are excluded to optimize size and avoid backing up dependency bloat:
- **OS System Metadata**: `.DS_Store`, `Thumbs.db`, etc.
- **Build Output & Dependencies**: `node_modules/`, `target/`, `bin/`, `obj/`, `*.o`, `*.exe`, etc.
- **iOS & CocoaPods Artifacts**: `DerivedData/`, `Pods/`, `ephemeral/`, etc.
- **Automated Build Scripts & Configs**: `gradle-wrapper.jar`, `GeneratedPluginRegistrant.*`, `Generated.xcconfig`, `generated_plugin*`, etc.
- **IDE Settings**: `.idea/`, `.vscode/`, etc.
