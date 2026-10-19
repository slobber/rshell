#!/usr/bin/env bash
set -euo pipefail

ARCH="${1:-x64}"
case "$ARCH" in
    x64)
        TARGET="x86_64-unknown-linux-gnu"
        DEB_ARCH="amd64"
        RPM_ARCH="x86_64"
        ;;
    arm64)
        TARGET="aarch64-unknown-linux-gnu"
        DEB_ARCH="arm64"
        RPM_ARCH="aarch64"
        ;;
    *)
        echo "Usage: $0 [x64|arm64]" >&2
        exit 2
        ;;
esac

if ! command -v fpm >/dev/null 2>&1; then
    echo "fpm is required. Install Ruby and run: gem install --no-document fpm" >&2
    exit 2
fi

VERSION="$(awk -F'"' '/^version = / { print $2; exit }' Cargo.toml)"
PACKAGER_VERSION="$(awk -F'"' '/^version = / { print $2; exit }' Packager.toml)"
if [ "$VERSION" != "$PACKAGER_VERSION" ]; then
    echo "Version mismatch: Cargo.toml=$VERSION, Packager.toml=$PACKAGER_VERSION" >&2
    exit 1
fi
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

mkdir -p "$STAGE/usr/bin"
mkdir -p "$STAGE/usr/share/applications"
mkdir -p "$STAGE/usr/share/icons/hicolor/256x256/apps"

rustup target add "$TARGET"
cargo build --release --target "$TARGET" --bin rshell
install -m 0755 "target/$TARGET/release/rshell" "$STAGE/usr/bin/rshell"
install -m 0644 assets/app-icon.png \
    "$STAGE/usr/share/icons/hicolor/256x256/apps/rshell.png"

cat > "$STAGE/usr/share/applications/rshell.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=RShell
Comment=SSH, SFTP, WSL and RDP workbench
Exec=rshell
Icon=rshell
Terminal=false
Categories=Network;Development;Utility;
StartupWMClass=RShell
EOF

mkdir -p dist
fpm -s dir -t deb \
    -n rshell \
    -v "$VERSION" \
    -a "$DEB_ARCH" \
    -C "$STAGE" \
    --description "RShell SSH, SFTP, WSL and RDP workbench" \
    --url "https://github.com/slobber/rshell" \
    --license "MIT" \
    --maintainer "slobber" \
    --vendor "slobber" \
    --category "Development" \
    -p "dist/rshell-$VERSION-linux-$ARCH.deb" \
    usr

fpm -s dir -t rpm \
    -n rshell \
    -v "$VERSION" \
    -a "$RPM_ARCH" \
    -C "$STAGE" \
    --description "RShell SSH, SFTP, WSL and RDP workbench" \
    --url "https://github.com/slobber/rshell" \
    --license "MIT" \
    --maintainer "slobber" \
    --vendor "slobber" \
    --category "Development" \
    -p "dist/rshell-$VERSION-linux-$ARCH.rpm" \
    usr

echo "Created: dist/rshell-$VERSION-linux-$ARCH.deb"
echo "Created: dist/rshell-$VERSION-linux-$ARCH.rpm"
