#!/bin/sh
# Resolve script directory and change to it
cd "$(dirname "$0")"

echo "Building Debian package..."

# Create staging directory structure
mkdir -p debian/usr/bin
mkdir -p ../../dist/packages

# Locate and copy binary
if [ -f "../../dist/binaries/rwif" ]; then
    cp ../../dist/binaries/rwif debian/usr/bin/rwif
elif [ -f "../../target/x86_64-unknown-linux-musl/release/rwif" ]; then
    cp ../../target/x86_64-unknown-linux-musl/release/rwif debian/usr/bin/rwif
elif [ -f "../../target/release/rwif" ]; then
    cp ../../target/release/rwif debian/usr/bin/rwif
else
    echo "Error: compiled rwif binary not found in target/ or dist/binaries/."
    exit 1
fi

chmod 755 debian/usr/bin/rwif

# Run dpkg-deb to build the package
dpkg-deb --build debian ../../dist/packages/rwif.deb

# Clean up staging binary
rm -f debian/usr/bin/rwif
