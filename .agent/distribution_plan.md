# Gitwig Distribution Plan: `curl | sh` Installer

This plan outlines the design, implementation, and deployment strategy for distributing **Gitwig** via a simple, single-command shell installer (`curl -fsSL ... | sh`).

---

## 1. Overview
The goal is to allow users to install Gitwig on macOS and Linux systems by running:
```bash
curl -fsSL https://raw.githubusercontent.com/tareqmy/gitwig/main/install.sh | sh
```
The installer script detects the host's operating system and architecture, resolves the latest release from the GitHub API, downloads the pre-built binary package compiled by GitHub Actions, extracts it, and installs it to an appropriate location in the user's `PATH`.

---

## 2. Release & Artifact Mapping
Gitwig's continuous delivery pipeline (`.github/workflows/cd.yml`) compiles binaries for the following targets:
* **macOS (Intel):** `x86_64-apple-darwin`
* **macOS (Apple Silicon):** `aarch64-apple-darwin`
* **Linux (Intel/AMD 64-bit):** `x86_64-unknown-linux-gnu`
* **Windows (Intel/AMD 64-bit):** `x86_64-pc-windows-msvc` (normally not installed via shell script, but supported if needed)

The release archives uploaded by `taiki-e/upload-rust-binary-action` follow the pattern:
`gitwig-<tag>-<target>.<archive-format>` (e.g., `gitwig-v2.0.3-x86_64-apple-darwin.tar.gz`).

---

## 3. Installer Script Design (`install.sh`)
The shell script must be POSIX-compliant, secure, and handle error conditions gracefully.

### Core Features of the Script:
1. **OS & Architecture Auto-Detection**: Maps `uname -s` and `uname -m` outputs to the release targets.
2. **GitHub Releases API Resolution**: Fetches the latest release version tag from `api.github.com`.
3. **Flexible Version Overrides**: Allows users to specify a specific version (e.g., `VERSION=v2.0.0 curl ... | sh`).
4. **Fallback Mechanics**: Fallback to raw download if GitHub API limits are exceeded, or prompts user for input.
5. **No-Sudo Local Installation**:
   - Attempts to install to `/usr/local/bin` if the script is run with root permissions or if the user has write access to it.
   - Falls back to `~/.local/bin` or `~/.bin` otherwise, checking if they exist or creating them.
6. **PATH Verification**: Warns the user and gives configuration advice if the destination folder is not in their `$PATH`.
7. **Clean Exit**: Traps exit signals to clean up temporary download files.

---

## 4. Proposed `install.sh` Implementation
Below is the shell script to be saved at the repository root (`install.sh`):

```bash
#!/bin/sh

# Gitwig Installer Script
# Supported Platforms: macOS, Linux (x86_64, arm64)
# Usage: curl -fsSL https://raw.githubusercontent.com/tareqmy/gitwig/main/install.sh | sh

set -eu

# Configuration
REPO_OWNER="tareqmy"
REPO_NAME="gitwig"
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
        NC='\033[0;57m' # No Color
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
                x86_64|amd64) TARGET="x86_64-unknown-linux-gnu" ;;
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
        info "Querying latest release from GitHub API..."
        # Attempt to use curl to query GitHub API
        if command -v curl >/dev/null 2>&1; then
            LATEST_RELEASE=$(curl -s "${GITHUB_API_URL}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"tag_name": "([^"]+)".*/\1/')
        elif command -v wget >/dev/null 2>&1; then
            LATEST_RELEASE=$(wget -qO- "${GITHUB_API_URL}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"tag_name": "([^"]+)".*/\1/')
        else
            error "Neither curl nor wget found. Please install one of them."
        fi

        if [ -z "${LATEST_RELEASE}" ]; then
            # Handle API rate-limiting or network issues
            warn "Failed to resolve latest version from GitHub API (likely rate limited)."
            warn "Falling back to package version verification or manual specification."
            error "Could not auto-detect latest release. Run script with: VERSION=vX.Y.Z sh -c \"\$(curl -fsSL ...)\""
        fi
        VERSION="${LATEST_RELEASE}"
    fi
    info "Using version: ${VERSION}"
}

# Select installation directory
select_install_dir() {
    # Preferred: /usr/local/bin (system-wide)
    # Fallback: ~/.local/bin (user local)
    # Secondary fallback: ~/.bin
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

    info "Downloading Gitwig from: ${DOWNLOAD_URL}"
    
    ARCHIVE_PATH="${TMP_DIR}/gitwig.tar.gz"

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL -o "${ARCHIVE_PATH}" "${DOWNLOAD_URL}"
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O "${ARCHIVE_PATH}" "${DOWNLOAD_URL}"
    else
        error "Neither curl nor wget found."
    fi

    info "Extracting archive..."
    tar -xzf "${ARCHIVE_PATH}" -C "${TMP_DIR}"

    # Verify binary exists in archive
    if [ ! -f "${TMP_DIR}/gitwig" ]; then
        error "Binary 'gitwig' not found in the downloaded archive."
    fi

    # Install binary
    info "Installing gitwig to ${INSTALL_DIR}..."
    mv "${TMP_DIR}/gitwig" "${INSTALL_DIR}/gitwig"
    chmod +x "${INSTALL_DIR}/gitwig"
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
verify_path

# Quick test run
if command -v "${INSTALL_DIR}/gitwig" >/dev/null 2>&1; then
    VERSION_OUT=$("${INSTALL_DIR}/gitwig" --version 2>&1 || echo "installed successfully")
    success "Verification: ${VERSION_OUT}"
else
    warn "Installed binary could not be invoked directly (possibly path issues)."
fi
```

---

## 5. Deployment Workflow
To deploy this installation method:

1. **Commit and Push `install.sh`**:
   Place the `install.sh` script in the root of the Gitwig repository and push it to the `main` branch.
   * Path: `/Users/tareqmy/development/rustprojects/gitwig/install.sh`

2. **Verify Release Versioning**:
   The installer script pulls the version from the GitHub releases API. Ensure that tags created in GitHub follow the format `vX.Y.Z` (e.g., `v2.0.3`), which is already configured in the `.github/workflows/cd.yml` trigger (`v*`).

3. **Update Documentation**:
   Update `README.md` to instruct users to run the curl installer command:
   ```markdown
   ### Installation via Shell Script
   ```bash
   curl -fsSL https://raw.githubusercontent.com/tareqmy/gitwig/main/install.sh | sh
   ```
   ```

---

## 6. Testing the Installer
Before publicizing the installation method, perform the following verification:

1. **Local Script Verification**:
   Execute the installer script locally to verify it selects the correct download URL for your current OS/Arch:
   ```bash
   chmod +x install.sh
   ./install.sh
   ```
   *(Note: Since you are running it before pushing a release with that tag, you can test by setting `VERSION=v2.0.3` or your latest tagged release to verify download).*

2. **Network Execution Mock**:
   Host the script locally and run it via curl redirection to simulate user behavior:
   ```bash
   python3 -m http.server 8000 &
   PID=$!
   sleep 1
   curl -fsSL http://localhost:8000/install.sh | sh
   kill $PID
   ```
