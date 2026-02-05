#!/bin/bash

# AxKeyStore Installation Script
# This script downloads the latest or a specific version of AxKeyStore and installs it locally.

set -e

REPO="basilgregory/axkeystore"
BINARY_NAME="axkeystore"
INSTALL_BASE_DIR="$HOME/.axkeystore"
INSTALL_DIR="$INSTALL_BASE_DIR/bin"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 Starting AxKeyStore installation...${NC}"

# 1. Detect OS and Architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
    darwin)
        PLATFORM="macos"
        ;;
    linux)
        PLATFORM="linux"
        ;;
    *)
        echo -e "${RED}❌ Unsupported operating system: $OS${NC}"
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64|amd64)
        ARCH="x86_64"
        ;;
    arm64|aarch64)
        ARCH="aarch64"
        ;;
    *)
        echo -e "${RED}❌ Unsupported architecture: $ARCH${NC}"
        exit 1
        ;;
esac

# 2. Check for dependencies
if ! command -v curl >/dev/null 2>&1; then
    echo -e "${RED}❌ curl is required but not installed. Please install curl and try again.${NC}"
    exit 1
fi

# 3. Determine Version
VERSION=$1
if [ -z "$VERSION" ] || [ "$VERSION" = "latest" ]; then
    echo -e "${BLUE}🔍 Fetching latest version information...${NC}"
    VERSION=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    if [ -z "$VERSION" ]; then
        echo -e "${RED}❌ Failed to fetch latest version. Please specify a version tag (e.g., v0.1.6).${NC}"
        exit 1
    fi
else
    # Ensure version starts with 'v'
    if [[ ! "$VERSION" =~ ^v ]]; then
        VERSION="v$VERSION"
    fi
fi

echo -e "${BLUE}📦 Target Version: $VERSION${NC}"
echo -e "${BLUE}💻 Platform: $PLATFORM-$ARCH${NC}"

# 3. Construct Asset Name
ASSET_NAME="${BINARY_NAME}-${PLATFORM}-${ARCH}"
if [ "$PLATFORM" = "windows" ]; then
    ASSET_NAME="${ASSET_NAME}.exe"
fi

DOWNLOAD_URL="https://github.com/$REPO/releases/download/$VERSION/$ASSET_NAME"

# 4. Download Binary
TMP_DIR=$(mktemp -d)
TMP_BINARY="$TMP_DIR/$ASSET_NAME"

echo -e "${BLUE}📥 Downloading $ASSET_NAME...${NC}"
if ! curl -L --fail -o "$TMP_BINARY" "$DOWNLOAD_URL"; then
    echo -e "${RED}❌ Failed to download binary from $DOWNLOAD_URL${NC}"
    echo -e "${RED}   Please check if the version and platform are correct.${NC}"
    exit 1
fi

chmod +x "$TMP_BINARY"

# 5. Install Binary
FINAL_BINARY_PATH="$INSTALL_DIR/${BINARY_NAME}-${VERSION}"
SYMLINK_PATH="$INSTALL_DIR/${BINARY_NAME}"

echo -e "${BLUE}🔧 Installing to $INSTALL_DIR...${NC}"

# Create directories if they don't exist
mkdir -p "$INSTALL_DIR"

# Move binary and create symlink
mv "$TMP_BINARY" "$FINAL_BINARY_PATH"
ln -sf "$FINAL_BINARY_PATH" "$SYMLINK_PATH"

# 6. Cleanup
rm -rf "$TMP_DIR"

echo -e "${GREEN}✅ AxKeyStore $VERSION installed successfully!${NC}"
echo -e "${GREEN}✨ Installation path: ${NC}${BLUE}$SYMLINK_PATH${NC}"

# 7. Update PATH in shell profile
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    SHELL_TYPE=$(basename "$SHELL")
    PROFILE=""

    case "$SHELL_TYPE" in
        zsh)
            PROFILE="$HOME/.zshrc"
            [ ! -f "$PROFILE" ] && [ -f "$HOME/.zprofile" ] && PROFILE="$HOME/.zprofile"
            ;;
        bash)
            PROFILE="$HOME/.bashrc"
            [ ! -f "$PROFILE" ] && [ -f "$HOME/.bash_profile" ] && PROFILE="$HOME/.bash_profile"
            ;;
        *)
            PROFILE="$HOME/.profile"
            ;;
    esac

    if [ -n "$PROFILE" ]; then
        if [ ! -f "$PROFILE" ]; then
            touch "$PROFILE"
        fi

        if ! grep -q "$INSTALL_DIR" "$PROFILE"; then
            echo -e "${BLUE}🔧 Adding $INSTALL_DIR to PATH in $PROFILE...${NC}"
            echo "" >> "$PROFILE"
            echo "# AxKeyStore PATH" >> "$PROFILE"
            echo "export PATH=\"\$PATH:$INSTALL_DIR\"" >> "$PROFILE"
            echo -e "${GREEN}✅ PATH updated in $PROFILE${NC}"
        else
            echo -e "${BLUE}ℹ️  $INSTALL_DIR is already mentioned in $PROFILE.${NC}"
        fi
        echo -e "${BLUE}ℹ️  Please run 'source $PROFILE' or restart your terminal to use '${NC}${BLUE}${BINARY_NAME}${NC}${BLUE}' from anywhere.${NC}"
    fi
else
    echo -e "${GREEN}✨ AxKeyStore is ready! You can run it using the command: ${NC}${BLUE}${BINARY_NAME}${NC}"
fi

# Verify installation
if [ -f "$SYMLINK_PATH" ]; then
    INSTALLED_VER=$("$SYMLINK_PATH" --version 2>/dev/null || echo "$VERSION")
    echo -e "${GREEN}Installed version: $INSTALLED_VER${NC}"
fi
