#!/bin/bash
# Update Homebrew formula with latest release SHA
set -euo pipefail

VERSION="${1:-$(git describe --tags --abbrev=0)}"
FORMULA="homebrew/smol.rb"

# Download the tarball
TARBALL_URL="https://github.com/nnar1o/smol/archive/refs/tags/${VERSION}.tar.gz"
TARBALL_FILE="/tmp/smol-${VERSION}.tar.gz"

echo "Downloading ${TARBALL_URL}..."
curl -sL "${TARBALL_URL}" -o "${TARBALL_FILE}"

# Compute SHA256
SHA256=$(shasum -a 256 "${TARBALL_FILE}" | cut -d' ' -f1)
echo "SHA256: ${SHA256}"

# Update formula
if [[ "$OSTYPE" == "darwin"* ]]; then
    sed -i '' "s/sha256 \".*\"/sha256 \"${SHA256}\"/" "${FORMULA}"
else
    sed -i "s/sha256 \".*\"/sha256 \"${SHA256}\"/" "${FORMULA}"
fi

echo "Updated ${FORMULA} with SHA256 ${SHA256}"
rm -f "${TARBALL_FILE}"
