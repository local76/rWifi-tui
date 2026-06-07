#!/bin/sh
# Resolve script directory and change to it
cd "$(dirname "$0")"

echo "Building Flatpak package..."
flatpak-builder --force-clean build-dir org.local76.rwif.yaml
