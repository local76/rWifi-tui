#!/bin/sh
# Resolve script directory and change to it
cd "$(dirname "$0")"

echo "Testing Arch AUR package locally..."

# Get version from Cargo.toml
VERSION=$(grep -m1 '^version = ' ../../Cargo.toml | cut -d '"' -f2)
if [ -z "$VERSION" ]; then
    VERSION="3.0.1"
fi

# Substitute version in PKGBUILD
sed "s/pkgver=TEMPLATE_VERSION/pkgver=$VERSION/g" PKGBUILD > PKGBUILD.tmp && mv PKGBUILD.tmp PKGBUILD

# Run makepkg
makepkg -f
