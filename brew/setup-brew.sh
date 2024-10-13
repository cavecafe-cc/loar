#!/bin/bash

function getRepoVisibility() {
  local REPO_OWNER=$1
  local REPO_NAME=$2

  REPO_VISIBILITY=$(curl -s -H "Accept: application/vnd.github.v3+json" \
    "https://api.github.com/repos/$REPO_OWNER/$REPO_NAME" | \
    jq -r ".visibility")

  if [ "$REPO_VISIBILITY" == null ] || [ -z "$REPO_VISIBILITY" ]; then
    REPO_VISIBILITY="private"
  fi
  echo "$REPO_VISIBILITY"
}

function addGitIgnore() {
  local fileName=$1
  local GITIGNORE=".gitignore"
  if [ ! -f "$GITIGNORE" ]; then
    echo "create $GITIGNORE for current directory"
    touch $GITIGNORE
  fi
  if ! grep -q "$fileName" "$GITIGNORE"; then
    echo "$fileName" >> "$GITIGNORE"
    echo "$fileName entry to $GITIGNORE"
  fi
}

function getLatestFileUrl() {
  local ASSET_NAME=$1
  local LATEST_RELEASE_URL=""

  if [ "$REPO_VISIABLITY" == "public" ]; then
    LATEST_RELEASE_URL=$(curl -s \
      "https://api.github.com/repos/$REPO_OWNER/$REPO_NAME/releases/latest" | \
      jq -r ".assets[] | select(.name == \"$ASSET_NAME\") | .browser_download_url")
  else
    LATEST_RELEASE_URL=$(curl -s \
      -H "Accept: application/vnd.github.v3+json" \
      -H "Authorization: token $GITHUB_PAT" \
      "https://api.github.com/repos/$REPO_OWNER/$REPO_NAME/releases/latest" | \
      jq -r ".assets[] | select(.name == \"$ASSET_NAME\") | .browser_download_url")
  fi
  echo "$LATEST_RELEASE_URL"
}

function  generateBrewFile() {
  echo "Generating .brew file"

  # Get github repo URL only, then convert it as the following values
  # Extract GitHub repo URL and convert it to the following values
  # loop until the user enters a valid GitHub repo URL
  REPO_URL=""
  GITHUB_PAT=""
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
  PROJECT_NAME=$(echo "$REPO_NAME" | awk -F'-' '{for(i=1; i<=NF; i++) $i=toupper(substr($i,1,1)) tolower(substr($i,2))}1' OFS='')

  read -p "Enter PROJECT_DESC: " PROJECT_DESC
  read -p "Enter VERSION_VALUE: " VERSION_VALUE

  # Check if the GitHub repo is public or private
  if [ "$REPO_VISIBILITY" == "private" ]; then
    if [ -z "$GITHUB_PAT" ]; then
      echo "The GitHub repo is private."
      echo "Please provide a GitHub token to access the repo."
      read -p "Enter GitHub token (required for private repo): " GITHUB_PAT
      echo "GITHUB_PAT=$GITHUB_PAT"
    else
      echo "GITHUB_PAT is already set."
    fi
  else
    echo "The GitHub repo is public."
  fi

  cat <<EOF > $BREW_FILE
REPO_OWNER="$REPO_OWNER"
REPO_NAME="$REPO_NAME"
PROJECT_NAME="$PROJECT_NAME"
PROJECT_DESC="$PROJECT_DESC"
PROJECT_HOME="$PROJECT_HOME"
VERSION_VALUE="$VERSION_VALUE"
GITHUB_PAT="$GITHUB_PAT"
EOF

}

############ Main Script ############

addGitIgnore "*.rb"
addGitIgnore ".brew*"

BREW_FILE=".brew"
# Check if the GitHub repo is public or private
REPO_VISIBILITY=$(getRepoVisibility "$REPO_OWNER" "$REPO_NAME")
echo "Repo visibility: $REPO_VISIBILITY"

if [ ! -f "$BREW_FILE" ]; then
  generateBrewFile
fi

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

PROJECT_TAR_URL=$(getLatestFileUrl "$PROJECT_TAR_NAME")
echo "TAR URL: $PROJECT_TAR_URL"
PROJECT_TAR_SHA256_URL=$PROJECT_TAR_URL.SHA256
echo "SHA256 URL: $PROJECT_TAR_SHA256_URL"

# Fetch the SHA256 hash from the correct URL
if [ "$REPO_VISIBILITY" == "public" ]; then
  PROJECT_TAR_SHA256=$(curl -sL "$PROJECT_TAR_SHA256_URL")
else
  PROJECT_TAR_SHA256=$(curl -sL -H "Accept: application/vnd.github.v3+json" -H "Authorization: token $GITHUB_PAT" "$PROJECT_TAR_SHA256_URL")
fi
echo "SHA256: $PROJECT_TAR_SHA256"

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