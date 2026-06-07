#!/bin/sh
# Resolve script directory and change to it
cd "$(dirname "$0")"

echo "Building AppImage package..."
appimage-builder --recipe appimage-builder.yml
