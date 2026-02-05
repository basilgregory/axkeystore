# AxKeyStore Installation Script for Windows
# This script downloads the latest or a specific version of AxKeyStore and installs it locally.

$ErrorActionPreference = "Stop"

$REPO = "basilgregory/axkeystore"
$BINARY_NAME = "axkeystore"
$INSTALL_BASE_DIR = Join-Path $HOME ".axkeystore"
$INSTALL_DIR = Join-Path $INSTALL_BASE_DIR "bin"

Write-Host "🚀 Starting AxKeyStore installation..." -ForegroundColor Cyan

# 1. Detect Architecture
$ARCH = "x86_64" # Default for Windows runners in the workflow
# If we want to be more specific later, we can use $env:PROCESSOR_ARCHITECTURE

# 2. Determine Version
$VERSION = $args[0]
if (-not $VERSION -or $VERSION -eq "latest") {
    Write-Host "🔍 Fetching latest version information..." -ForegroundColor Cyan
    $Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/latest"
    $VERSION = $Release.tag_name
    if (-not $VERSION) {
        Write-Host "❌ Failed to fetch latest version. Please specify a version tag (e.g., v0.1.6)." -ForegroundColor Red
        exit 1
    }
} else {
    if (-not $VERSION.StartsWith("v")) {
        $VERSION = "v$VERSION"
    }
}

Write-Host "📦 Target Version: $VERSION" -ForegroundColor Cyan
Write-Host "💻 Platform: windows-$ARCH" -ForegroundColor Cyan

# 3. Construct Asset Name
$ASSET_NAME = "${BINARY_NAME}-windows-${ARCH}.exe"
$DOWNLOAD_URL = "https://github.com/$REPO/releases/download/$VERSION/$ASSET_NAME"

# 4. Download Binary
$TMP_DIR = Join-Path $env:TEMP ([Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $TMP_DIR | Out-Null
$TMP_BINARY = Join-Path $TMP_DIR $ASSET_NAME

Write-Host "📥 Downloading $ASSET_NAME..." -ForegroundColor Cyan
try {
    Invoke-WebRequest -Uri $DOWNLOAD_URL -OutFile $TMP_BINARY
} catch {
    Write-Host "❌ Failed to download binary from $DOWNLOAD_URL" -ForegroundColor Red
    Write-Host "   Please check if the version is correct." -ForegroundColor Red
    exit 1
}

# 5. Install Binary
if (-not (Test-Path $INSTALL_DIR)) {
    New-Item -ItemType Directory -Path $INSTALL_DIR | Out-Null
}

$FINAL_BINARY_NAME = "${BINARY_NAME}-${VERSION}.exe"
$FINAL_BINARY_PATH = Join-Path $INSTALL_DIR $FINAL_BINARY_NAME
$SYMLINK_PATH = Join-Path $INSTALL_DIR "${BINARY_NAME}.exe"

Write-Host "🔧 Installing to $INSTALL_DIR..." -ForegroundColor Cyan

# Move binary
Move-Item -Path $TMP_BINARY -Destination $FINAL_BINARY_PATH -Force

# Create "symlink" (on Windows, we'll just copy it to be safe, or use a hardlink)
Copy-Item -Path $FINAL_BINARY_PATH -Destination $SYMLINK_PATH -Force

# 6. Update PATH
$USER_PATH = [Environment]::GetEnvironmentVariable("Path", "User")
if ($USER_PATH -notlike "*$INSTALL_DIR*") {
    Write-Host "🔧 Adding $INSTALL_DIR to User PATH..." -ForegroundColor Cyan
    $NEW_PATH = "$USER_PATH;$INSTALL_DIR"
    [Environment]::SetEnvironmentVariable("Path", $NEW_PATH, "User")
    $env:Path = "$env:Path;$INSTALL_DIR"
    Write-Host "✅ PATH updated." -ForegroundColor Green
}

# 7. Cleanup
Remove-Item -Path $TMP_DIR -Recurse -Force

Write-Host "✅ AxKeyStore $VERSION installed successfully!" -ForegroundColor Green
Write-Host "✨ You can now run it using the command: axkeystore" -ForegroundColor Green
Write-Host "ℹ️  You may need to restart your terminal for the PATH changes to take effect." -ForegroundColor Yellow

# Verify
if (Get-Command $BINARY_NAME -ErrorAction SilentlyContinue) {
    $INSTALLED_VER = & $BINARY_NAME --version
    Write-Host "Installed version: $INSTALLED_VER" -ForegroundColor Green
}
