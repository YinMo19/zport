#!/bin/sh
set -eu

REPO="YinMo19/zport"
BIN="zport"

# ── Detect OS & architecture ────────────────────────────────────────

OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
    Linux)  goos="unknown-linux-gnu" ;;
    Darwin) goos="apple-darwin" ;;
    *)
        echo "error: unsupported OS: $OS" >&2
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64|amd64)  goarch="x86_64" ;;
    aarch64|arm64) goarch="aarch64" ;;
    *)
        echo "error: unsupported architecture: $ARCH" >&2
        exit 1
        ;;
esac

TARGET="${goarch}-${goos}"

# ── Resolve version ─────────────────────────────────────────────────

VERSION="${1:-latest}"

if [ "$VERSION" = "latest" ]; then
    echo "Fetching latest release..."
    VERSION=$(curl -sSL "https://api.github.com/repos/$REPO/releases/latest" \
        | grep '"tag_name"' | head -1 \
        | sed 's/.*"tag_name": *"\(.*\)".*/\1/')
    if [ -z "$VERSION" ]; then
        echo "error: could not determine latest version" >&2
        exit 1
    fi
fi

# ── Download & install ──────────────────────────────────────────────

URL="https://github.com/$REPO/releases/download/${VERSION}/${BIN}-${TARGET}.tar.gz"

echo "Downloading $BIN $VERSION for $TARGET..."
curl -fsSL "$URL" | tar xz

INSTALL_DIR="${HOME}/.local/bin"
mkdir -p "$INSTALL_DIR"
mv "$BIN" "$INSTALL_DIR/$BIN"
chmod +x "$INSTALL_DIR/$BIN"

echo "✓ Installed to $INSTALL_DIR/$BIN"

if ! echo "$PATH" | tr ':' '\n' | grep -qxF "$INSTALL_DIR"; then
    printf 'note: %s is not in PATH. Add this to your shell profile:\n' "$INSTALL_DIR"
    printf '  export PATH="%s:$PATH"\n' "$INSTALL_DIR"
fi
