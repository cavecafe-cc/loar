#!/bin/bash

SNAP_FILE=".snap"

if [ ! -f "$SNAP_FILE" ]; then
  # Get github repo URL only, then convert it as the following values
  # Extract GitHub repo URL and convert it to the following values
  # loop until the user enters a valid GitHub repo URL
  REPO_URL=""
  while [ -z "$REPO_URL" ]; do
    read -p "Enter GitHub repo URL (i.e. 'https://github.com/{your-repo-name}.git'): " REPO_URL
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

  # Capitalize only the first letter of REPO_NAME (i.e.) 'MyCamelProject'
  PROJECT_NAME=$(echo "$REPO_NAME" | awk '{print toupper(substr($0,1,1)) tolower(substr($0,2))}')

  read -p "Enter PROJECT_SUMMARY: " PROJECT_SUMMARY
  read -p "Enter PROJECT_DESC: " PROJECT_DESC
  read -p "Enter VERSION_VALUE: " VERSION_VALUE
  read -p "Logo file path: " LOGO_FILE
  read -p "Enter LICENSE: " LICENSE

  cat <<EOF > "$SNAP_FILE"
REPO_OWNER="$REPO_OWNER"
REPO_NAME="$REPO_NAME"
PROJECT_NAME="$PROJECT_NAME"
PROJECT_SUMMARY="$PROJECT_SUMMARY"
PROJECT_DESC="$PROJECT_DESC"
PROJECT_HOME="$PROJECT_HOME"
VERSION_VALUE="$VERSION_VALUE"
LOGO_FILE="$LOGO_FILE"
LICENSE="$LICENSE"
EOF
fi

# Load the .brew file
source "$SNAP_FILE"

PROJECT_NAME_LOWER=$(echo "$PROJECT_NAME" | tr '[:upper:]' '[:lower:]')

# Get the Operating System and Architecture
OS_TYPE=$(uname -s | tr '[:upper:]' '[:lower:]')
if [ "$OS_TYPE" == "darwin" ]; then
  OS_TYPE="osx"
fi
ARCH_TYPE=$(uname -m)

## Get the Binary File Type and Tar Name (e.g., loar_osx-arm64.tar.gz)
#BIN_FILE_TYPE="$OS_TYPE-$ARCH_TYPE"
#echo file type: "'$BIN_FILE_TYPE'"
#PROJECT_TAR_NAME="$REPO_NAME"_"$BIN_FILE_TYPE".tar.gz
#echo tar name: "'$PROJECT_TAR_NAME'"
#
## Get the latest release URL
#function getLatestFileUrl() {
#  local ASSET_NAME=$1
#
#  LATEST_RELEASE_URL=$(curl -s \
#    "https://api.github.com/repos/$REPO_OWNER/$REPO_NAME/releases/latest" | \
#    jq -r ".assets[] | select(.name == \"$ASSET_NAME\") | .browser_download_url")
#  echo "$LATEST_RELEASE_URL"
#}
#
## Get the latest release
#PROJECT_TAR_URL=$(getLatestFileUrl "$PROJECT_TAR_NAME")
#PROJECT_TAR_SHA256_URL=$(getLatestFileUrl "$PROJECT_TAR_NAME.SHA256")
#PROJECT_TAR_SHA256=$(curl -sL "$PROJECT_TAR_SHA256_URL")

# Create Snapcraft YAML
cat <<EOF > snapscraft.yaml
name: $PROJECT_NAME_LOWER
version: '$VERSION_VALUE'
grade: stable
summary: $PROJECT_SUMMARY
description: $PROJECT_DESC
base: core22
confinement: strict

adopt-info: $PROJECT_NAME_LOWER
compression: lzo
icon: $LOGO_FILE
type: app
title: $PROJECT_NAME
issues: $PROJECT_HOME/issues
contact: https://github.com/$REPO_OWNER
website: $PROJECT_HOME
license: $LICENSE

architectures:
  - build-on: amd64
  #- build-on: arm64 # supported by MultiPass, but limited in macOS

apps:
  $PROJECT_NAME_LOWER:
    command: $PROJECT_NAME_LOWER
    environment:
      DOTNET_ROOT: $SNAP/usr/share/dotnet

parts:
  $PROJECT_NAME_LOWER:
    source: $REPO_URL
    source-type: git
    source-branch: main
    plugin: dotnet
    dotnet-build-configuration: Release
    dotnet-self-contained-runtime-identifier: linux-x64
    # supported but limited in macOS, Windows
    #dotnet-self-contained-runtime-identifier: osx-arm64
    #dotnet-self-contained-runtime-identifier: win-x64
    #dotnet-self-contained-runtime-identifier: osx-x64
    build-packages:
      - dotnet-sdk-8.0
    stage-packages:
      - libicu70
EOF

echo "Snapcraft YAML generated: snapscraft.yaml"
echo "----------------"
cat snapscraft.yaml
echo "----------------"
echo ""
echo "To execute via Snapcraft Pipeline:"
echo " login snapcraft.io, connect your repo"
echo " then commit snap/snapcraft.yaml"
echo ""
echo "To install snap package locally:"
echo " > snapcraft clean $PROJECT_NAME_LOWER && snapcraft"
echo " > snap install --dangerous ./"$PROJECT_NAME_LOWER"_"$VERSION_VALUE"_"$ARCH_TYPE".snap"
echo ""
echo "To publish:"
echo " > snapcraft login"
echo " > snapcraft push ./"$PROJECT_NAME_LOWER"_"$VERSION_VALUE"_"$ARCH_TYPE".snap --release edge"
echo ""
echo "To release:"
echo " > snapcraft release "$PROJECT_NAME_LOWER" "$VERSION_VALUE" <edge | beta | candidate | stable>"
echo ""
echo "To clean, run: snapcraft clean $PROJECT_NAME_LOWER && rm -rf prime parts stage"
echo "To remove, run: snap remove $PROJECT_NAME_LOWER"
echo "To check, run: snap list | grep $PROJECT_NAME_LOWER"
echo ""
echo "For more information, visit https://snapcraft.io/docs"
echo ""