#!/bin/bash

BREW_FILE=".brew"

if [ ! -f "$BREW_FILE" ]; then
  # Get github repo URL only, then convert it as the following values
  # Extract GitHub repo URL and convert it to the following values
  # loop until the user enters a valid GitHub repo URL
  REPO_URL=""
  while [ -z "$REPO_URL" ]; do
    read -p "Enter GitHub repo URL: " REPO_URL
    REPO_URL=${REPO_URL%.git}
    # check if the URL is valid
    if ! grep -qE '^https://github.com/.+/.+$' <<< "$REPO_URL"; then
      echo "Invalid GitHub repo URL. Please enter a valid GitHub repo URL."
      REPO_URL=""
    fi
  done

  # Extract the following values from the GitHub repo URL
  REPO_OWNER=$(echo "$REPO_URL" | awk -F'/' '{print $(NF-1)}')
  REPO_NAME=$(echo "$REPO_URL" | awk -F'/' '{print $NF}')
  PROJECT_HOME=$(echo "$REPO_URL" | sed 's/.git$//')

  # Capitalize only the first letter of REPO_NAME
  PROJECT_NAME=$(echo "$REPO_NAME" | awk '{print toupper(substr($0,1,1)) tolower(substr($0,2))}')

  read -p "Enter PROJECT_DESC: " PROJECT_DESC
  read -p "Enter VERSION_VALUE: " VERSION_VALUE

  cat <<EOF > "$BREW_FILE"
REPO_OWNER="$REPO_OWNER"
REPO_NAME="$REPO_NAME"
PROJECT_NAME="$PROJECT_NAME"
PROJECT_DESC="$PROJECT_DESC"
PROJECT_HOME="$PROJECT_HOME"
VERSION_VALUE="$VERSION_VALUE"
EOF
fi

# Load the .brew file
source "$BREW_FILE"

PROJECT_NAME_LOWER=$(echo "$PROJECT_NAME" | tr '[:upper:]' '[:lower:]')

# Get the Operating System and Architecture
OS_TYPE=$(uname -s | tr '[:upper:]' '[:lower:]')
if [ "$OS_TYPE" == "darwin" ]; then
  OS_TYPE="osx"
fi
ARCH_TYPE=$(uname -m)

# Get the Binary File Type and Tar Name (e.g., loar_osx-arm64.tar.gz)
BIN_FILE_TYPE="$OS_TYPE-$ARCH_TYPE"
echo file type: "'$BIN_FILE_TYPE'"
PROJECT_TAR_NAME="$REPO_NAME"_"$BIN_FILE_TYPE".tar.gz
echo tar name: "'$PROJECT_TAR_NAME'"

# Get the latest release URL
function getLatestFileUrl() {
  local ASSET_NAME=$1

  LATEST_RELEASE_URL=$(curl -s \
    "https://api.github.com/repos/$REPO_OWNER/$REPO_NAME/releases/latest" | \
    jq -r ".assets[] | select(.name == \"$ASSET_NAME\") | .browser_download_url")
  echo "$LATEST_RELEASE_URL"
}

# Get the latest release
PROJECT_TAR_URL=$(getLatestFileUrl "$PROJECT_TAR_NAME")
PROJECT_TAR_SHA256_URL=$(getLatestFileUrl "$PROJECT_TAR_NAME.SHA256")
PROJECT_TAR_SHA256=$(curl -sL "$PROJECT_TAR_SHA256_URL")

# Create Brew Formula
cat <<EOF > "$REPO_NAME".rb
class $PROJECT_NAME < Formula
  desc "$PROJECT_DESC"
  homepage "$PROJECT_HOME"
  url "$PROJECT_TAR_URL"
  sha256 "$PROJECT_TAR_SHA256"
  version "$VERSION_VALUE"

  def install
    bin.install "$REPO_NAME"
  end
end
EOF

# Homebrew Installation
echo "Check the formula: $REPO_NAME.rb"
echo "------"
cat "$REPO_NAME".rb
echo "------"
echo "To install, run: brew install ./$REPO_NAME.rb"