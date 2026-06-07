#!/bin/sh
# Resolve script directory and change to it
cd "$(dirname "$0")"

echo "Building Nix derivation package..."
nix-build default.nix
