#!/bin/sh
# Resolve script directory and change to it
cd "$(dirname "$0")"

echo "Building Alpine APK package..."

# Get version from Cargo.toml
VERSION=$(grep -m1 '^version = ' ../../Cargo.toml | cut -d '"' -f2)
if [ -z "$VERSION" ]; then
    VERSION="3.0.1"
fi

# Substitute version in APKBUILD
sed "s/pkgver=TEMPLATE_VERSION/pkgver=$VERSION/g" APKBUILD > APKBUILD.tmp && mv APKBUILD.tmp APKBUILD

# Run abuild
abuild -r
