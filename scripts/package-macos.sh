#!/usr/bin/env bash
set -euo pipefail

TARGET="aarch64-apple-darwin"
VERSION="$(awk -F'"' '/^version = / { print $2; exit }' Cargo.toml)"
PACKAGER_VERSION="$(awk -F'"' '/^version = / { print $2; exit }' Packager.toml)"
if [ "$VERSION" != "$PACKAGER_VERSION" ]; then
    echo "Version mismatch: Cargo.toml=$VERSION, Packager.toml=$PACKAGER_VERSION" >&2
    exit 1
fi

if ! command -v cargo-packager >/dev/null 2>&1; then
    cargo install cargo-packager --version 0.11.8 --locked
fi

rustup target add "$TARGET"
cargo build --release --target "$TARGET" --bin rshell
mkdir -p dist
cp "target/$TARGET/release/rshell" "dist/rshell"
cargo packager --release --target "$TARGET" --formats dmg --out-dir dist
rm -f "dist/rshell"
rm -rf "dist/RShell.app"

DMG="$(find dist -maxdepth 1 -type f -name '*.dmg' -print | sort | tail -n 1)"
if [ -z "$DMG" ]; then
    echo "cargo-packager did not produce a dmg" >&2
    exit 1
fi
mv -f "$DMG" "dist/rshell-$VERSION-macos-aarch64.dmg"
echo "Created: dist/rshell-$VERSION-macos-aarch64.dmg"
