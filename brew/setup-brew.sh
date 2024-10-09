#!/bin/bash

REPO_OWNER="cavecafe-cc"
REPO_NAME="loar"
PROJECT_NAME="Loar"
PROJECT_DESC="LOAR - Local Archive Utility"
PROJECT_HOME="https://github.com/cavecafe-cc/loar"
VERSION_VALUE='0.7.0'
PROJECT_NAME_LOWER=$(echo "$PROJECT_NAME" | tr '[:upper:]' '[:lower:]')
PROJECT_TAR_URL=getLatestFileUrl "loar_osx-arm64.tar.gz"
PROJECT_TAR_SHA256_URL=getLatestFileUrl "loar_osx-arm64.tar.gz.SHA256"

function getLatestFileUrl() {
  local ASSET_NAME=$1

  LATEST_RELEASE_URL=$(curl -s \
    "https://api.github.com/repos/$REPO_OWNER/$REPO_NAME/releases/latest" | \
    jq -r ".assets[] | select(.name == \"$ASSET_NAME\") | .browser_download_url")
  echo "$LATEST_RELEASE_URL"
  return "$LATEST_RELEASE_URL"
}

# for private repo
#GITHUB_PAT=${GITHUB_PAT_PULL_BINARY_LOAR:-"your_github_token_here"}
#function getLatestFileUrl() {
#  local ASSET_NAME=$1
#
#  LATEST_RELEASE_URL=$(curl -s -H "Authorization token $GITHUB_PAT" \
#    "https://api.github.com/repos/$REPO_OWNER/$REPO_NAME/releases/latest" | \
#    jq -r ".assets[] | select(.name == \"$ASSET_NAME\") | .browser_download_url")
#  echo "$LATEST_RELEASE_URL"
#  return "$LATEST_RELEASE_URL"
#}
#PROJECT_TAR=$(curl -sL -H "Authorization: token $GITHUB_PAT" "$PROJECT_TAR_URL")
#PROJECT_TAR_SHA256=$(curl -sL -H "Authorization: token $GITHUB_PAT" "$PROJECT_TAR_SHA256_URL")


# Create Brew Formula
cat <<EOF > "$PROJECT_NAME_LOWER".rb
class $PROJECT_NAME < Formula
  desc "$PROJECT_DESC"
  homepage "$PROJECT_HOME"
  url "$PROJECT_TAR_URL"
  sha256 "$PROJECT_TAR_SHA256"
  version "$VERSION_VALUE"

  def install
    bin.install "$PROJECT_NAME_LOWER"
  end
end
EOF

# Homebrew Installation
# brew install ./"$PROJECT_NAME_LOWER".rb
echo "Check the formula: $PROJECT_NAME_LOWER.rb"
echo "------"
cat "$PROJECT_NAME_LOWER".rb
echo "------"
echo "To install, run: brew install ./$PROJECT_NAME_LOWER.rb"