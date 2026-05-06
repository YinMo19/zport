#!/bin/sh
set -eu

REPO="YinMo19/zport"
BIN="zport"

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

VERSION="${1:-latest}"
if [ "$VERSION" = "latest" ]; then
    URL="https://github.com/$REPO/releases/latest/download/${BIN}-${TARGET}.tar.gz"
else
    URL="https://github.com/$REPO/releases/download/${VERSION}/${BIN}-${TARGET}.tar.gz"
fi

echo "Downloading $BIN $VERSION for $TARGET..."
curl -fsSL "$URL" | tar xz

INSTALL_DIR="${HOME}/.local/bin"
mkdir -p "$INSTALL_DIR"
mv "$BIN" "$INSTALL_DIR/$BIN"
chmod +x "$INSTALL_DIR/$BIN"

echo "Installed to $INSTALL_DIR/$BIN"

if ! echo "$PATH" | tr ':' '\n' | grep -qxF "$INSTALL_DIR"; then
    printf 'note: %s is not in PATH. Add this to your shell profile:\n' "$INSTALL_DIR"
    printf '  export PATH="%s:$PATH"\n' "$INSTALL_DIR"
fi
