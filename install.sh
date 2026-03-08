#!/usr/bin/env sh
# dXon install script
# Usage: curl -sSfL https://raw.githubusercontent.com/P8labs/dxon/master/install.sh | sh
# Or:    wget -qO- https://raw.githubusercontent.com/P8labs/dxon/master/install.sh | sh

set -e

REPO="P8labs/dxon"
BINARY="dxon"
INSTALL_DIR="${DXON_INSTALL_DIR:-/usr/local/bin}"

# ── helpers ────────────────────────────────────────────────────────────────────

info()  { printf '\033[0;34m  info\033[0m  %s\n' "$*"; }
ok()    { printf '\033[0;32m    ok\033[0m  %s\n' "$*"; }
warn()  { printf '\033[0;33m  warn\033[0m  %s\n' "$*" >&2; }
error() { printf '\033[0;31m error\033[0m  %s\n' "$*" >&2; exit 1; }

need_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        error "required command not found: $1"
    fi
}

# ── detect OS ──────────────────────────────────────────────────────────────────

detect_os() {
    OS="$(uname -s 2>/dev/null | tr '[:upper:]' '[:lower:]')"
    case "$OS" in
        linux)  OS="linux" ;;
        darwin) error "macOS is not supported yet. dXon requires systemd-nspawn (Linux only)." ;;
        *)      error "unsupported OS: $OS" ;;
    esac
}

# ── detect architecture ────────────────────────────────────────────────────────

detect_arch() {
    ARCH="$(uname -m 2>/dev/null)"
    case "$ARCH" in
        x86_64 | amd64)          ARCH="x86_64" ;;
        aarch64 | arm64)         ARCH="aarch64" ;;
        armv7l | armv7 | armhf)  ARCH="armv7" ;;
        *)  error "unsupported architecture: $ARCH" ;;
    esac
}

# ── fetch latest release tag ───────────────────────────────────────────────────

latest_version() {
    if command -v curl >/dev/null 2>&1; then
        VERSION="$(curl -sSfL "https://api.github.com/repos/${REPO}/releases/latest" \
            | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')"
    elif command -v wget >/dev/null 2>&1; then
        VERSION="$(wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" \
            | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')"
    else
        error "curl or wget is required to download dXon"
    fi

    if [ -z "$VERSION" ]; then
        error "could not determine the latest release version"
    fi
}

# ── download binary ────────────────────────────────────────────────────────────

download() {
    ASSET="${BINARY}-${OS}-${ARCH}"
    URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}"

    TMP="$(mktemp)"
    trap 'rm -f "$TMP"' EXIT INT TERM

    info "downloading ${BINARY} ${VERSION} (${OS}/${ARCH})"

    if command -v curl >/dev/null 2>&1; then
        curl -sSfL --retry 3 --retry-delay 2 -o "$TMP" "$URL" \
            || error "download failed: $URL"
    else
        wget -qO "$TMP" "$URL" \
            || error "download failed: $URL"
    fi

    chmod +x "$TMP"
    echo "$TMP"
}

# ── install binary ─────────────────────────────────────────────────────────────

install_bin() {
    TMPFILE="$1"
    DEST="${INSTALL_DIR}/${BINARY}"

    if [ -w "$INSTALL_DIR" ]; then
        mv -f "$TMPFILE" "$DEST"
    else
        info "install directory requires elevated privileges: $INSTALL_DIR"
        if command -v sudo >/dev/null 2>&1; then
            sudo mv -f "$TMPFILE" "$DEST"
        elif command -v doas >/dev/null 2>&1; then
            doas mv -f "$TMPFILE" "$DEST"
        else
            error "cannot write to $INSTALL_DIR — run as root or set DXON_INSTALL_DIR to a writable path"
        fi
    fi
}

# ── check systemd-nspawn ────────────────────────────────────────────────────────

check_nspawn() {
    if ! command -v systemd-nspawn >/dev/null 2>&1; then
        warn "systemd-nspawn not found. dXon requires it to run containers."
        warn "install it for your distro:"
        warn "  Arch Linux:    sudo pacman -S systemd"
        warn "  Debian/Ubuntu: sudo apt install systemd-container"
        warn "  Fedora:        sudo dnf install systemd-container"
        warn "  openSUSE:      sudo zypper install systemd-container"
        warn "  Alpine Linux:  not supported (systemd-nspawn unavailable on musl)"
    fi
}

# ── main ───────────────────────────────────────────────────────────────────────

main() {
    detect_os
    detect_arch

    # Allow pinning a version via env var
    if [ -n "$DXON_VERSION" ]; then
        VERSION="$DXON_VERSION"
    else
        latest_version
    fi

    TMPFILE="$(download)"
    install_bin "$TMPFILE"

    ok "${BINARY} ${VERSION} installed to ${INSTALL_DIR}/${BINARY}"
    check_nspawn

    printf '\ndone! run: %s --version\n' "$BINARY"
}

main "$@"
