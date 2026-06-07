#!/usr/bin/env bash

set -euo pipefail

VERSION="$1"
TARGET_OS="$2"
TARGET_ARCH="$3"

PROJECT_NAME="OmegaCode"

ROOT_DIR="$(pwd)"

BUILD_DIR="$ROOT_DIR/build"
BUNDLE_DIR="$BUILD_DIR/release_bundle"

OUTPUT_DIR="$ROOT_DIR/target/${PROJECT_NAME}-${VERSION}"

mkdir -p "$OUTPUT_DIR"

echo "Packaging..."
echo "Version : $VERSION"
echo "OS      : $TARGET_OS"
echo "Arch    : $TARGET_ARCH"

tar -czf \
"$OUTPUT_DIR/${PROJECT_NAME}_${VERSION}_${TARGET_OS}_${TARGET_ARCH}.tar.gz" \
-C "$BUILD_DIR" \
release_bundle

if [ "$TARGET_OS" = "linux" ]; then

    fpm \
      -s dir \
      -t deb \
      -n "$PROJECT_NAME" \
      -v "$VERSION" \
      --architecture "$TARGET_ARCH" \
      "$BUNDLE_DIR=/usr/local/OmegaCode"

    mv ./*.deb "$OUTPUT_DIR/" 2>/dev/null || true

    fpm \
      -s dir \
      -t rpm \
      -n "$PROJECT_NAME" \
      -v "$VERSION" \
      --architecture "$TARGET_ARCH" \
      "$BUNDLE_DIR=/usr/local/OmegaCode"

    mv ./*.rpm "$OUTPUT_DIR/" 2>/dev/null || true

fi

if [ "$TARGET_OS" = "windows" ]; then

    makensis \
      -DVERSION="$VERSION" \
      -DARCH="$TARGET_ARCH" \
      installer.nsi

    mv ./*.exe "$OUTPUT_DIR/" 2>/dev/null || true

fi

if [ "$TARGET_OS" = "macos" ]; then

    DMG_NAME="${PROJECT_NAME}_${VERSION}_macOS_${TARGET_ARCH}.dmg"

    hdiutil create \
      -volname "$PROJECT_NAME" \
      -srcfolder "$BUNDLE_DIR" \
      -ov \
      -format UDZO \
      "$OUTPUT_DIR/$DMG_NAME"

fi

if [ -d "$BUNDLE_DIR/sbom" ]; then

    tar -czf \
      "$OUTPUT_DIR/${PROJECT_NAME}_${VERSION}_sbom.tar.gz" \
      -C "$BUNDLE_DIR" \
      sbom

fi

echo "Packaging Finished"