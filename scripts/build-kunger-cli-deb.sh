#!/bin/bash
# Build minimal kunger-cli .deb package
# Usage: ./scripts/build-kunger-cli-deb.sh [target-triple]
#
# Creates a lightweight .deb with just the CLI binary and minimal runtime deps
# Useful for headless/VPS installations without GUI dependencies

set -e

TARGET="${1:-x86_64-unknown-linux-gnu}"
ARCH="${TARGET%%-*}"

# Map Rust targets to Debian architectures
case "$ARCH" in
  x86_64) DEB_ARCH="amd64" ;;
  aarch64) DEB_ARCH="arm64" ;;
  *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

# Get version from Cargo.toml
VERSION=$(grep '^version' src-tauri/Cargo.toml | head -1 | cut -d'"' -f2)

# Build the CLI binary
echo "Building kunger-cli for $ARCH ($TARGET)..."
cd src-tauri
cargo build --release --target "$TARGET" --bin kunger-cli
cd ..

# Create temporary .deb directory structure
TEMP_DEB="src-tauri/target/deb-cli-$ARCH"
rm -rf "$TEMP_DEB"
mkdir -p "$TEMP_DEB/DEBIAN"
mkdir -p "$TEMP_DEB/usr/local/bin"
mkdir -p "$TEMP_DEB/usr/share/doc/kunger-cli"

# Copy CLI binary
cp "src-tauri/target/$TARGET/release/kunger-cli" "$TEMP_DEB/usr/local/bin/kunger-cli"
chmod +x "$TEMP_DEB/usr/local/bin/kunger-cli"

# Create control file
cat > "$TEMP_DEB/DEBIAN/control" << EOF
Package: kunger-cli
Version: $VERSION
Architecture: $DEB_ARCH
Maintainer: Kunger Contributors <https://github.com/KaungHtetSoe/kunger>
Depends: libc6, libssl3
Homepage: https://github.com/KaungHtetSoe/kunger
Description: Kunger CLI - Read-only Linux software inventory (headless)
 Kunger is a read-only software inventory tool for Debian/Ubuntu systems.
 This package provides kunger-cli, suitable for headless systems, VPS, and
 containers. No GUI or display server required.
 .
 For the graphical interface, install the 'kunger' package instead.
EOF

# Create copyright file
cat > "$TEMP_DEB/usr/share/doc/kunger-cli/copyright" << EOF
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: kunger
Upstream-Contact: Kunger Contributors <https://github.com/KaungHtetSoe/kunger>
Source: https://github.com/KaungHtetSoe/kunger

Files: *
Copyright: 2025 Kunger Contributors
License: GPL-3.0-or-later

License: GPL-3.0-or-later
 This program is free software: you can redistribute it and/or modify
 it under the terms of the GNU General Public License as published by
 the Free Software Foundation, either version 3 of the License, or
 (at your option) any later version.
 .
 This program is distributed in the hope that it will be useful,
 but WITHOUT ANY WARRANTY; without even the implied warranty of
 MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 GNU General Public License for more details.
 .
 You should have received a copy of the GNU General Public License
 along with this program.  If not, see <https://www.gnu.org/licenses/>.
EOF

# Build the .deb package
DEB_NAME="kunger-cli_${VERSION}_${DEB_ARCH}.deb"
DEB_PATH="src-tauri/target/release/bundle/deb/$DEB_NAME"

mkdir -p "src-tauri/target/release/bundle/deb"
dpkg-deb --build "$TEMP_DEB" "$DEB_PATH"

echo "✅ Built: $DEB_PATH"
echo "📦 Size: $(du -h "$DEB_PATH" | cut -f1)"
