#!/usr/bin/env bash

set -euo pipefail

PROJECT_NAME="OmegaCode"

ROOT_DIR="$(pwd)"

CONSOLE_UI_DIR="$ROOT_DIR/console/console-ui"
CONSOLE_PANEL_DIR="$ROOT_DIR/console/console-panel"
MCP_SERVER_DIR="$ROOT_DIR/mcp-server"

BUILD_DIR="$ROOT_DIR/build"
BUNDLE_DIR="$BUILD_DIR/release_bundle"

RUST_TARGET="${RUST_TARGET:-}"

echo "=================================="
echo "Build Started"
echo "=================================="

rm -rf "$BUILD_DIR"
mkdir -p "$BUNDLE_DIR"

echo "[UI] Building"

pushd "$CONSOLE_UI_DIR"

npm ci
npm run build

if [ ! -d dist ]; then
    echo "dist directory not found"
    exit 1
fi

popd

echo "[JAVA] Building"

pushd "$CONSOLE_PANEL_DIR"

mvn -B clean package

PANEL_JAR=$(find target -type f -name "*.jar" \
    ! -name "*sources.jar" \
    ! -name "*javadoc.jar" \
    | head -n 1)

if [ ! -f "$PANEL_JAR" ]; then
    echo "Jar not found"
    exit 1
fi

popd

echo "[RUST] Building"

if [ -n "$RUST_TARGET" ]; then

    cargo build \
        --release \
        --target "$RUST_TARGET"

    RELEASE_DIR="$ROOT_DIR/target/$RUST_TARGET/release"

else

    cargo build --release

    RELEASE_DIR="$ROOT_DIR/target/release"

fi

EXECUTABLE="$RELEASE_DIR/OmegaCode"

if [ -f "$EXECUTABLE.exe" ]; then
    EXECUTABLE="$EXECUTABLE.exe"
fi

if [ ! -f "$EXECUTABLE" ]; then
    echo "OmegaCode executable not found"
    exit 1
fi

echo "[BUNDLE]"

mkdir -p "$BUNDLE_DIR/ui"

cp "$EXECUTABLE" \
   "$BUNDLE_DIR/"

cp "$PANEL_JAR" \
   "$BUNDLE_DIR/console_panel.jar"

cp -R "$CONSOLE_UI_DIR/dist/"* \
      "$BUNDLE_DIR/ui/"

cp -R "$MCP_SERVER_DIR" \
      "$BUNDLE_DIR/"

mkdir -p "$BUNDLE_DIR/sbom"

if command -v cargo-cyclonedx >/dev/null 2>&1; then
    cargo cyclonedx \
      --format json \
      --output-file "$BUNDLE_DIR/sbom/rust-bom.json" || true
fi

echo "Build Finished"

find "$BUNDLE_DIR"