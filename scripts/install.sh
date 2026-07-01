#!/bin/sh

# Gitwig Installer Script
# Supported Platforms: macOS, Linux (x86_64, arm64)
# Usage: curl -fsSL https://raw.githubusercontent.com/tareqmy/gitwig/master/scripts/install.sh | sh

set -eu

# Configuration
REPO_OWNER="tareqmy"
REPO_NAME="gitwig"
GITHUB_RAW_URL="https://raw.githubusercontent.com/${REPO_OWNER}/${REPO_NAME}/master"
GITHUB_API_URL="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}"
GITHUB_RELEASES_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases"

# Colored Output Helpers
setup_colors() {
    if [ -t 1 ]; then
        RED='\033[0;31m'
        GREEN='\033[0;32m'
        YELLOW='\033[0;33m'
        BLUE='\033[0;34m'
        BOLD='\033[1m'
        NC='\033[0m' # No Color
    else
        RED=''
        GREEN=''
        YELLOW=''
        BLUE=''
        BOLD=''
        NC=''
    fi
}

info() {
    printf "${BLUE}[info]${NC} %s\n" "$1"
}

success() {
    printf "${GREEN}[success]${NC} %s\n" "$1"
}

warn() {
    printf "${YELLOW}[warn]${NC} %s\n" "$1"
}

error() {
    printf "${RED}[error]${NC} %s\n" "$1" >&2
    exit 1
}

# Detect Platform (OS and Architecture)
detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "${OS}" in
        Darwin)
            case "${ARCH}" in
                x86_64) TARGET="x86_64-apple-darwin" ;;
                arm64|aarch64) TARGET="aarch64-apple-darwin" ;;
                *) error "Unsupported macOS architecture: ${ARCH}" ;;
            esac
            ;;
        Linux)
            case "${ARCH}" in
                x86_64|amd64) TARGET="x86_64-unknown-linux-musl" ;;
                *) error "Unsupported Linux architecture: ${ARCH}" ;;
            esac
            ;;
        *)
            error "Unsupported Operating System: ${OS}"
            ;;
    esac
    info "Detected platform: ${OS} (${ARCH}) -> Target: ${TARGET}"
}

# Resolve target version
resolve_version() {
    if [ "${VERSION:-}" = "" ]; then
        info "Querying latest version..."
        
        # Check if running locally in a cloned repo with .version
        if [ -f ".version" ]; then
            LATEST_RELEASE=$(cat .version)
            info "Found local .version file: ${LATEST_RELEASE}"
        else
            # Try to fetch .version from GitHub raw
            CURL_CMD="curl -fsSL"
            WGET_CMD="wget -qO-"
            if [ -n "${GITHUB_TOKEN:-}" ]; then
                CURL_CMD="${CURL_CMD} -H \"Authorization: token ${GITHUB_TOKEN}\""
                WGET_CMD="${WGET_CMD} --header=\"Authorization: token ${GITHUB_TOKEN}\""
            fi
            
            LATEST_VERSION_URL="${GITHUB_RAW_URL}/.version"
            if command -v curl >/dev/null 2>&1; then
                LATEST_RELEASE=$(eval "${CURL_CMD} ${LATEST_VERSION_URL}" 2>/dev/null || true)
            elif command -v wget >/dev/null 2>&1; then
                LATEST_RELEASE=$(eval "${WGET_CMD} ${LATEST_VERSION_URL}" 2>/dev/null || true)
            fi
            
            # Fallback to GitHub API
            if [ -z "${LATEST_RELEASE:-}" ]; then
                info "Failed to fetch .version from raw URL, querying GitHub API..."
                API_URL="${GITHUB_API_URL}/releases/latest"
                
                if command -v curl >/dev/null 2>&1; then
                    if [ -n "${GITHUB_TOKEN:-}" ]; then
                        JSON=$(curl -fsSL -H "Authorization: token ${GITHUB_TOKEN}" "${API_URL}" 2>/dev/null || true)
                    else
                        JSON=$(curl -fsSL "${API_URL}" 2>/dev/null || true)
                    fi
                elif command -v wget >/dev/null 2>&1; then
                    if [ -n "${GITHUB_TOKEN:-}" ]; then
                        JSON=$(wget -qO- --header="Authorization: token ${GITHUB_TOKEN}" "${API_URL}" 2>/dev/null || true)
                    else
                        JSON=$(wget -qO- "${API_URL}" 2>/dev/null || true)
                    fi
                fi
                
                if [ -n "${JSON:-}" ]; then
                    LATEST_RELEASE=$(echo "${JSON}" | grep '"tag_name":' | sed -E 's/.*"tag_name": "([^"]+)".*/\1/' || true)
                fi

                # Second fallback: check tags API if releases/latest is empty or failed
                if [ -z "${LATEST_RELEASE:-}" ] || echo "${LATEST_RELEASE:-}" | grep -iq "Not Found" || echo "${LATEST_RELEASE:-}" | grep -iq "rate limit" || echo "${LATEST_RELEASE:-}" | grep -iq "message"; then
                    API_URL="${GITHUB_API_URL}/tags"
                    if command -v curl >/dev/null 2>&1; then
                        if [ -n "${GITHUB_TOKEN:-}" ]; then
                            JSON=$(curl -fsSL -H "Authorization: token ${GITHUB_TOKEN}" "${API_URL}" 2>/dev/null || true)
                        else
                            JSON=$(curl -fsSL "${API_URL}" 2>/dev/null || true)
                        fi
                    elif command -v wget >/dev/null 2>&1; then
                        if [ -n "${GITHUB_TOKEN:-}" ]; then
                            JSON=$(wget -qO- --header="Authorization: token ${GITHUB_TOKEN}" "${API_URL}" 2>/dev/null || true)
                        else
                            JSON=$(wget -qO- "${API_URL}" 2>/dev/null || true)
                        fi
                    fi
                    if [ -n "${JSON:-}" ]; then
                        LATEST_RELEASE=$(echo "${JSON}" | grep '"name":' | head -n 1 | sed -E 's/.*"name": "([^"]+)".*/\1/' || true)
                    fi
                fi
            fi
        fi

        # Clean up any potential error JSON message parsed as version
        if echo "${LATEST_RELEASE:-}" | grep -iq "Not Found" || echo "${LATEST_RELEASE:-}" | grep -iq "rate limit" || echo "${LATEST_RELEASE:-}" | grep -iq "message"; then
            LATEST_RELEASE=""
        fi

        if [ -z "${LATEST_RELEASE:-}" ]; then
            error "Could not auto-detect latest version (this can happen due to network issues or GitHub API rate limits). Please specify the version manually, for example: VERSION=v2.3.1 ./install.sh"
        fi
        VERSION="${LATEST_RELEASE}"
    fi

    # Normalize version format (ensure it has the 'v' prefix for tags)
    case "${VERSION}" in
        v*) ;;
        *) VERSION="v${VERSION}" ;;
    esac

    info "Using version: ${VERSION}"
}

# Select installation directory
select_install_dir() {
    # Preferred: /usr/local/bin (system-wide)
    # Fallback: ~/.local/bin (user local)
    if [ "$(id -u)" -eq 0 ]; then
        INSTALL_DIR="/usr/local/bin"
    elif [ -w "/usr/local/bin" ]; then
        INSTALL_DIR="/usr/local/bin"
    else
        INSTALL_DIR="${HOME}/.local/bin"
        if [ ! -d "${INSTALL_DIR}" ]; then
            mkdir -p "${INSTALL_DIR}"
        fi
    fi
    info "Selected installation directory: ${INSTALL_DIR}"
}

# Download and Extract
download_and_extract() {
    # Build download URL (e.g. gitwig-v2.0.3-aarch64-apple-darwin.tar.gz)
    DOWNLOAD_URL="${GITHUB_RELEASES_URL}/download/${VERSION}/gitwig-${VERSION}-${TARGET}.tar.gz"
    
    # Create temporary directory
    TMP_DIR=$(mktemp -d -t gitwig-install.XXXXXX)
    trap 'rm -rf "${TMP_DIR}"' EXIT INT TERM

    ARCHIVE_PATH="${TMP_DIR}/gitwig.tar.gz"

    # If GITHUB_TOKEN is set, download via API asset endpoint
    if [ -n "${GITHUB_TOKEN:-}" ]; then
        info "Using GITHUB_TOKEN to download from private repository..."
        TAG_API_URL="${GITHUB_API_URL}/releases/tags/${VERSION}"
        
        if command -v curl >/dev/null 2>&1; then
            RELEASE_JSON=$(curl -fsSL -H "Authorization: token ${GITHUB_TOKEN}" "${TAG_API_URL}" 2>/dev/null || true)
        elif command -v wget >/dev/null 2>&1; then
            RELEASE_JSON=$(wget -qO- --header="Authorization: token ${GITHUB_TOKEN}" "${TAG_API_URL}" 2>/dev/null || true)
        fi
        
        if [ -z "${RELEASE_JSON:-}" ]; then
            error "Failed to retrieve release metadata for ${VERSION} using token."
        fi
        
        # We need the asset name matching the target
        ASSET_NAME="gitwig-${VERSION}-${TARGET}.tar.gz"
        
        # Find asset ID in the JSON using grep and sed
        ASSET_ID=$(echo "${RELEASE_JSON}" | grep -B 10 -A 10 "\"name\": \"${ASSET_NAME}\"" | grep "\"id\":" | head -n 1 | sed -E 's/.*"id": *([0-9]+).*/\1/' || true)
        
        if [ -z "${ASSET_ID}" ]; then
            # Try alternate pattern (without version prefix, e.g. gitwig-aarch64-apple-darwin.tar.gz)
            ALT_ASSET_NAME="gitwig-${TARGET}.tar.gz"
            ASSET_ID=$(echo "${RELEASE_JSON}" | grep -B 10 -A 10 "\"name\": \"${ALT_ASSET_NAME}\"" | grep "\"id\":" | head -n 1 | sed -E 's/.*"id": *([0-9]+).*/\1/' || true)
            if [ -n "${ASSET_ID}" ]; then
                ASSET_NAME="${ALT_ASSET_NAME}"
            fi
        fi

        if [ -z "${ASSET_ID}" ]; then
            error "Could not find asset '${ASSET_NAME}' in release ${VERSION}."
        fi
        
        ASSET_URL="${GITHUB_API_URL}/releases/assets/${ASSET_ID}"
        info "Downloading asset ${ASSET_NAME} (ID: ${ASSET_ID})..."
        
        if command -v curl >/dev/null 2>&1; then
            curl -fsSL -H "Authorization: token ${GITHUB_TOKEN}" -H "Accept: application/octet-stream" -o "${ARCHIVE_PATH}" "${ASSET_URL}"
        elif command -v wget >/dev/null 2>&1; then
            wget -q -O "${ARCHIVE_PATH}" --header="Authorization: token ${GITHUB_TOKEN}" --header="Accept: application/octet-stream" "${ASSET_URL}"
        fi
    else
        # Public download
        info "Downloading Gitwig from: ${DOWNLOAD_URL}"
        if command -v curl >/dev/null 2>&1; then
            curl -fsSL -o "${ARCHIVE_PATH}" "${DOWNLOAD_URL}"
        elif command -v wget >/dev/null 2>&1; then
            wget -q -O "${ARCHIVE_PATH}" "${DOWNLOAD_URL}"
        else
            error "Neither curl nor wget found."
        fi
    fi

    info "Extracting archive..."
    tar -xzf "${ARCHIVE_PATH}" -C "${TMP_DIR}"

    # Verify binary exists in archive (it might be in root of archive or nested)
    BINARY_PATH=$(find "${TMP_DIR}" -type f -name "gitwig" -perm -111 -print -quit || find "${TMP_DIR}" -type f -name "gitwig" -print -quit || true)
    if [ -z "${BINARY_PATH}" ] || [ ! -f "${BINARY_PATH}" ]; then
        error "Binary 'gitwig' not found in the downloaded archive."
    fi

    # Install binary
    info "Installing gitwig to ${INSTALL_DIR}..."
    mv "${BINARY_PATH}" "${INSTALL_DIR}/gitwig"
    chmod +x "${INSTALL_DIR}/gitwig"
}

# Check and install fzf dependency
check_and_install_fzf() {
    if command -v fzf >/dev/null 2>&1; then
        info "fzf is already installed."
        return 0
    fi

    info "fzf is not installed. Gitwig requires fzf for repository picking."
    
    # Prompt the user if running interactively
    if [ -t 0 ] && [ -t 1 ]; then
        printf "Would you like to install fzf now? [Y/n] "
        read -r ANSWER
        case "${ANSWER}" in
            [nN]|[nN][oO])
                warn "Skipping fzf installation. Some Gitwig features may not function correctly."
                return 0
                ;;
        esac
    else
        info "Installing fzf automatically..."
    fi

    # Try installing using package managers
    if [ "${OS}" = "Darwin" ]; then
        if command -v brew >/dev/null 2>&1; then
            info "Installing fzf via Homebrew..."
            brew install fzf
        else
            warn "Homebrew not found. Please install fzf manually: https://github.com/junegunn/fzf"
        fi
    elif [ "${OS}" = "Linux" ]; then
        if command -v apt-get >/dev/null 2>&1; then
            info "Installing fzf via apt-get..."
            sudo apt-get update && sudo apt-get install -y fzf
        elif command -v pacman >/dev/null 2>&1; then
            info "Installing fzf via pacman..."
            sudo pacman -S --noconfirm fzf
        elif command -v dnf >/dev/null 2>&1; then
            info "Installing fzf via dnf..."
            sudo dnf install -y fzf
        elif command -v yum >/dev/null 2>&1; then
            info "Installing fzf via yum..."
            sudo yum install -y fzf
        else
            warn "No supported package manager found. Please install fzf manually: https://github.com/junegunn/fzf"
        fi
    fi
}

# Check Path and notify user
verify_path() {
    case ":${PATH}:" in
        *:${INSTALL_DIR}:*)
            success "gitwig is installed and ready in your PATH!"
            ;;
        *)
            warn "Installation directory (${INSTALL_DIR}) is not in your PATH."
            warn "Please add it to your PATH by adding the following line to your shell configuration (.zshrc, .bashrc, or .profile):"
            printf "\n  export PATH=\"\$PATH:%s\"\n\n" "${INSTALL_DIR}"
            ;;
    esac
}

# Execution
setup_colors
detect_platform
resolve_version
select_install_dir
download_and_extract
check_and_install_fzf
verify_path

# Quick test run
if command -v "${INSTALL_DIR}/gitwig" >/dev/null 2>&1; then
    VERSION_OUT=$("${INSTALL_DIR}/gitwig" --version 2>&1 || echo "installed successfully")
    success "Verification: ${VERSION_OUT}"
else
    warn "Installed binary could not be invoked directly (possibly path issues)."
fi
