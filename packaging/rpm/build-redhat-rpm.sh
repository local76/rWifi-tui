#!/bin/sh
# Resolve script directory and change to it
cd "$(dirname "$0")"

echo "Building RedHat RPM package..."

# Ensure output directory exists
mkdir -p ../../dist/packages

# Ensure rpmbuild local directory exists
mkdir -p rpmbuild/SOURCES rpmbuild/SPECS rpmbuild/BUILD rpmbuild/RPMS rpmbuild/SRPMS

# Get version from Cargo.toml
VERSION=$(grep -m1 '^version = ' ../../Cargo.toml | cut -d '"' -f2)
if [ -z "$VERSION" ]; then
    VERSION="3.0.1"
fi

# Prepare spec file with version substituted
sed "s/TEMPLATE_VERSION/$VERSION/g" rwif.spec > rpmbuild/SPECS/rwif.spec

# Create source tarball
tar --exclude='rpmbuild' --exclude='target' --exclude='.git' -czf rpmbuild/SOURCES/rwif-$VERSION.tar.gz -C ../.. .

# Run rpmbuild locally
rpmbuild -ba rpmbuild/SPECS/rwif.spec --define "_topdir $(pwd)/rpmbuild"

# Copy output to dist/packages
cp rpmbuild/RPMS/*/*.rpm ../../dist/packages/rwif.rpm
