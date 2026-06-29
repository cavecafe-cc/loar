#!/bin/sh
# LoAr (Local Archive) - Universal Installer for Linux & macOS
set -e

# Detect OS
OS="$(uname -s)"
case "${OS}" in
    Darwin*)    OS_NAME="macos"; EXT="tar.gz";;
    Linux*)     OS_NAME="linux"; EXT="deb";;
    *)          echo "Error: Unsupported OS type: ${OS}" >&2; exit 1;;
esac

# Detect Architecture
ARCH="$(uname -m)"
case "${ARCH}" in
    x86_64*)            ARCH_NAME="x86_64";;
    arm64*|aarch64*)    ARCH_NAME="arm64";;
    *)                  echo "Error: Unsupported CPU architecture: ${ARCH}" >&2; exit 1;;
esac

ASSET_NAME="loar-${OS_NAME}-${ARCH_NAME}-latest.${EXT}"
DOWNLOAD_URL="https://bin.cavecafe.cc/downloads/loar/${ASSET_NAME}"

echo "==> Detecting system: ${OS_NAME} (${ARCH_NAME})"
echo "==> Downloading latest release from ${DOWNLOAD_URL}..."

TEMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TEMP_DIR}"' EXIT

curl -fsSL "${DOWNLOAD_URL}" -o "${TEMP_DIR}/${ASSET_NAME}"

if [ "${OS_NAME}" = "macos" ]; then
    echo "==> Extracting and installing to /usr/local/bin/loar (requires sudo)..."
    tar -xzf "${TEMP_DIR}/${ASSET_NAME}" -C "${TEMP_DIR}"
    sudo mv "${TEMP_DIR}/loar" /usr/local/bin/loar
    sudo chmod +x /usr/local/bin/loar
else
    echo "==> Installing Debian package (requires sudo)..."
    sudo dpkg -i "${TEMP_DIR}/${ASSET_NAME}"
fi

echo "==> LoAr installation completed successfully! Try running 'loar --version'."
