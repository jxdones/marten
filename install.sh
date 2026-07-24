#!/usr/bin/env sh
# Install Marten from a GitHub Release.
# Usage: curl -fsSL https://raw.githubusercontent.com/jxdones/marten/main/install.sh | sh
# Pass a version with: ... | sh -s -- v0.1.0
# Set BINDIR to choose the install directory (default: /usr/local/bin).

set -eu

APP="marten"
REPO="jxdones/marten"
VERSION="${1:-latest}"
BINDIR="${BINDIR:-/usr/local/bin}"

if command -v curl >/dev/null 2>&1; then
  download() {
    curl -fsSL "$1" -o "$2"
  }
elif command -v wget >/dev/null 2>&1; then
  download() {
    wget -qO "$2" "$1"
  }
else
  echo "marten: curl or wget is required" >&2
  exit 1
fi

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
case "$OS" in
  darwin|linux) ;;
  *)
    echo "marten: unsupported operating system: $OS" >&2
    exit 1
    ;;
esac

ARCH="$(uname -m)"
case "$ARCH" in
  x86_64|amd64) ARCH="amd64" ;;
  aarch64|arm64) ARCH="arm64" ;;
  *)
    echo "marten: unsupported architecture: $ARCH" >&2
    exit 1
    ;;
esac

MARTEN_TMPDIR="$(mktemp -d "${TMPDIR:-/tmp}/marten-install.XXXXXX")"
trap 'rm -rf "$MARTEN_TMPDIR"' EXIT HUP INT TERM

if [ "$VERSION" = "latest" ]; then
  download \
    "https://api.github.com/repos/${REPO}/releases/latest" \
    "$MARTEN_TMPDIR/latest.json"
  VERSION="$(sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$MARTEN_TMPDIR/latest.json" | head -n 1)"
  if [ -z "$VERSION" ]; then
    echo "marten: failed to resolve the latest release" >&2
    exit 1
  fi
else
  case "$VERSION" in
    v*) ;;
    *) VERSION="v$VERSION" ;;
  esac
fi

if ! printf '%s\n' "$VERSION" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$'; then
  echo "marten: invalid version: $VERSION" >&2
  exit 1
fi

SEMVER="${VERSION#v}"
ARCHIVE="marten_${SEMVER}_${OS}_${ARCH}.tar.gz"
BASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"

echo "Installing marten ${VERSION} (${OS}/${ARCH})..."
download "$BASE_URL/$ARCHIVE" "$MARTEN_TMPDIR/$ARCHIVE"
download "$BASE_URL/checksums.txt" "$MARTEN_TMPDIR/checksums.txt"

EXPECTED="$(awk -v archive="$ARCHIVE" '$2 == archive { print $1; exit }' "$MARTEN_TMPDIR/checksums.txt")"
if [ -z "$EXPECTED" ]; then
  echo "marten: checksum not found for $ARCHIVE" >&2
  exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
  ACTUAL="$(sha256sum "$MARTEN_TMPDIR/$ARCHIVE" | awk '{ print $1 }')"
elif command -v shasum >/dev/null 2>&1; then
  ACTUAL="$(shasum -a 256 "$MARTEN_TMPDIR/$ARCHIVE" | awk '{ print $1 }')"
else
  echo "marten: sha256sum or shasum is required" >&2
  exit 1
fi

if [ "$ACTUAL" != "$EXPECTED" ]; then
  echo "marten: checksum verification failed for $ARCHIVE" >&2
  exit 1
fi

tar -xzf "$MARTEN_TMPDIR/$ARCHIVE" -C "$MARTEN_TMPDIR" "$APP"

if [ -d "$BINDIR" ] && [ -w "$BINDIR" ]; then
  install -m 755 "$MARTEN_TMPDIR/$APP" "$BINDIR/$APP"
elif [ ! -e "$BINDIR" ] && mkdir -p "$BINDIR" 2>/dev/null; then
  install -m 755 "$MARTEN_TMPDIR/$APP" "$BINDIR/$APP"
elif command -v sudo >/dev/null 2>&1; then
  echo "Installing to $BINDIR requires elevated permissions..."
  sudo install -d "$BINDIR"
  sudo install -m 755 "$MARTEN_TMPDIR/$APP" "$BINDIR/$APP"
else
  echo "marten: cannot write to $BINDIR; set BINDIR to a writable directory" >&2
  exit 1
fi

echo "Marten installed to $BINDIR/$APP"
if ! printf ':%s:' "$PATH" | grep -Fq ":$BINDIR:"; then
  echo "Add $BINDIR to your PATH:"
  echo "  export PATH=\"\$PATH:$BINDIR\""
fi
