#!/usr/bin/env sh

set -e

REPO="P8labs/dxon"
BINARY="dxon"


info()  { printf '\033[0;34m  info\033[0m  %s\n' "$*"; }
ok()    { printf '\033[0;32m    ok\033[0m  %s\n' "$*"; }
warn()  { printf '\033[0;33m  warn\033[0m  %s\n' "$*" >&2; }
error() { printf '\033[0;31m error\033[0m  %s\n' "$*" >&2; exit 1; }


find_current() {
    CURRENT_BIN="$(command -v "$BINARY" 2>/dev/null)"
    if [ -z "$CURRENT_BIN" ]; then
        warn "$BINARY is not on PATH"
        warn "run the install script instead:"
        warn "  curl -sSfL https://raw.githubusercontent.com/P8labs/dxon/master/install.sh | sh"
        exit 1
    fi

    INSTALL_DIR="$(dirname "$CURRENT_BIN")"
    CURRENT_VERSION="$("$CURRENT_BIN" --version 2>/dev/null | awk '{print $NF}' || true)"
    info "current: ${CURRENT_VERSION:-(unknown)} at ${CURRENT_BIN}"
}


latest_version() {
    if command -v curl >/dev/null 2>&1; then
        LATEST="$(curl -sSfL "https://api.github.com/repos/${REPO}/releases/latest" \
            | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')"
    elif command -v wget >/dev/null 2>&1; then
        LATEST="$(wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" \
            | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')"
    else
        error "curl or wget is required"
    fi

    if [ -z "$LATEST" ]; then
        error "could not determine the latest release version"
    fi

    info "latest:  ${LATEST}"
}


strip_v() { echo "${1#v}"; }

already_current() {
    [ "$(strip_v "$CURRENT_VERSION")" = "$(strip_v "$LATEST")" ]
}


detect_platform() {
    OS="$(uname -s 2>/dev/null | tr '[:upper:]' '[:lower:]')"
    ARCH="$(uname -m 2>/dev/null)"
    case "$OS" in
        linux)  ;;
        darwin) error "macOS is not supported. dXon requires systemd-nspawn (Linux only)." ;;
        *)      error "unsupported OS: $OS" ;;
    esac
    case "$ARCH" in
        x86_64 | amd64)         ARCH="amd64" ;;
        aarch64 | arm64)        ARCH="aarch64" ;;
        armv7l | armv7 | armhf) ARCH="armv7" ;;
        *)  error "unsupported architecture: $ARCH" ;;
    esac
}


download_and_install() {
    ASSET="${BINARY}-${LATEST}-${OS}-${ARCH}"
    URL="https://github.com/${REPO}/releases/download/${LATEST}/${ASSET}"

    TMP="$(mktemp)"
    trap 'rm -f "$TMP"' EXIT INT TERM

    info "downloading ${BINARY} ${LATEST} (${OS}/${ARCH})"

    if command -v curl >/dev/null 2>&1; then
        curl -sSfL --retry 3 --retry-delay 2 -o "$TMP" "$URL" \
            || error "download failed: $URL"
    else
        wget -qO "$TMP" "$URL" \
            || error "download failed: $URL"
    fi

    chmod +x "$TMP"

    if [ -w "$INSTALL_DIR" ]; then
        mv -f "$TMP" "${INSTALL_DIR}/${BINARY}"
    else
        info "install directory requires elevated privileges: $INSTALL_DIR"
        if command -v sudo >/dev/null 2>&1; then
            sudo mv -f "$TMP" "${INSTALL_DIR}/${BINARY}"
        elif command -v doas >/dev/null 2>&1; then
            doas mv -f "$TMP" "${INSTALL_DIR}/${BINARY}"
        else
            error "cannot write to $INSTALL_DIR — run as root"
        fi
    fi
}


main() {
    find_current
    latest_version
    detect_platform

    if already_current; then
        ok "${BINARY} is already up to date (${CURRENT_VERSION})"
        exit 0
    fi

    download_and_install
    ok "upgraded ${BINARY}: ${CURRENT_VERSION} → ${LATEST}"
}

main "$@"
