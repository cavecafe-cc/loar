# LoAr (Local Archive) - Windows Installer
$ErrorActionPreference = "Stop"

# Detect Architecture
$arch = $env:PROCESSOR_ARCHITECTURE
if ($arch -eq "AMD64") {
    $ARCH_NAME = "x86_64"
} elseif ($arch -eq "ARM64") {
    $ARCH_NAME = "arm64"
} else {
    Write-Error "Unsupported CPU architecture: $arch"
    exit 1
}

$ASSET_NAME = "loar-windows-$ARCH_NAME-latest.zip"
$DOWNLOAD_URL = "https://bin.cavecafe.cc/downloads/loar/$ASSET_NAME"
$INSTALL_DIR = "$env:USERPROFILE\AppData\Local\Programs\loar"

Write-Host "==> Detecting Windows architecture: $ARCH_NAME"
Write-Host "==> Downloading latest release from $DOWNLOAD_URL..."

if (-not (Test-Path $INSTALL_DIR)) {
    New-Item -ItemType Directory -Force -Path $INSTALL_DIR | Out-Null
}

$tempZip = [System.IO.Path]::GetTempFileName() + ".zip"

# Download Zip
Invoke-WebRequest -Uri $DOWNLOAD_URL -OutFile $tempZip

Write-Host "==> Extracting files to $INSTALL_DIR..."
# Extract (overwrite if exists)
Expand-Archive -Path $tempZip -DestinationPath $INSTALL_DIR -Force
Remove-Item -Path $tempZip -Force

# Add to User PATH if not present
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -split ';' -notcontains $INSTALL_DIR) {
    Write-Host "==> Adding $INSTALL_DIR to User PATH environment variable..."
    $newUserPath = "$userPath;$INSTALL_DIR"
    [Environment]::SetEnvironmentVariable("Path", $newUserPath, "User")
    # Update current session path
    $env:Path = "$env:Path;$INSTALL_DIR"
}

Write-Host "==> LoAr installation completed successfully!"
Write-Host "==> Please restart your terminal/shell, then run: loar --version"
