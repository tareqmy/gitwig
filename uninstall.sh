#!/bin/sh
set -e

# Colors
setup_colors() {
    if [ -t 1 ]; then
        RED='\033[0;31m'
        GREEN='\033[0;32m'
        YELLOW='\033[0;33m'
        BLUE='\033[0;34m'
        BOLD='\033[1m'
        NC='\033[0m'
    else
        RED=''
        GREEN=''
        YELLOW=''
        BLUE=''
        BOLD=''
        NC=''
    fi
}

info() { printf "${BLUE}[info]${NC} %s\n" "$1"; }
success() { printf "${GREEN}[success]${NC} %s\n" "$1"; }
warn() { printf "${YELLOW}[warn]${NC} %s\n" "$1"; }
error() { printf "${RED}[error]${NC} %s\n" "$1" >&2; exit 1; }

setup_colors

# Find gitwig
BINARY_PATH=""
if command -v gitwig >/dev/null 2>&1; then
    BINARY_PATH=$(command -v gitwig)
fi

# Fallback check standard locations if not in PATH
if [ -z "${BINARY_PATH}" ]; then
    if [ -f "${HOME}/.local/bin/gitwig" ]; then
        BINARY_PATH="${HOME}/.local/bin/gitwig"
    elif [ -f "/usr/local/bin/gitwig" ]; then
        BINARY_PATH="/usr/local/bin/gitwig"
    fi
fi

if [ -z "${BINARY_PATH}" ]; then
    error "Gitwig binary could not be found on your system."
fi

info "Found Gitwig binary at: ${BINARY_PATH}"

# Prompt for confirmation if interactive
if [ -t 0 ] && [ -t 1 ]; then
    printf "Are you sure you want to uninstall Gitwig? [y/N] "
    read -r ANSWER
    case "${ANSWER}" in
        [yY]|[yY][eE][sS])
            ;;
        *)
            info "Uninstall canceled."
            exit 0
            ;;
    esac
fi

# Remove binary
if rm "${BINARY_PATH}" 2>/dev/null; then
    success "Gitwig has been successfully uninstalled."
else
    # Try with sudo if write permissions are missing (e.g. /usr/local/bin)
    warn "Permission denied. Trying with sudo..."
    if sudo rm "${BINARY_PATH}"; then
        success "Gitwig has been successfully uninstalled."
    else
        error "Failed to remove ${BINARY_PATH}."
    fi
fi
