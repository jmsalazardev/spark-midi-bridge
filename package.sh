#!/bin/bash
# Spark MIDI Bridge packaging script.
# Compiles the program for Raspberry Pi Zero (ARMv6) and bundles it with setup scripts.

set -e

# Load custom environment variables if .env exists
if [ -f .env ]; then
  echo "ℹ️  Loading configuration from .env file..."
  source .env
fi

# Configuration defaults
TARGET=${TARGET:-"arm-unknown-linux-gnueabihf"}
BINARY_SOURCE="target/$TARGET/release/spark_midi_bridge"
DIST_DIR="dist"
ARCHIVE_NAME=${ARCHIVE_NAME:-"spark_midi_bridge.tar.gz"}

echo "========================================================"
echo "📦 Packaging Spark MIDI Bridge"
echo "========================================================"

# 1. Compile the program for the target board
if [ -f "./build.sh" ]; then
  ./build.sh
else
  echo "🔨 Compiling binary for $TARGET in release mode..."
  if ! command -v cross &> /dev/null; then
    echo "❌ Error: 'cross' command-line tool is not installed."
    echo "   Install it with: cargo install cross --git https://github.com/cross-rs/cross"
    exit 1
  fi
  cross build --target "$TARGET" --release
fi

# 2. Recreate clean distribution folder
echo "📁 Preparing distribution folder '$DIST_DIR'..."
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR/spark_midi_bridge"

# 3. Copy execution binary and scripts
echo "🚚 Copying files to distribution folder..."
if [ -f "$BINARY_SOURCE" ]; then
  cp "$BINARY_SOURCE" "$DIST_DIR/spark_midi_bridge/"
else
  echo "❌ Error: Compiled binary not found at $BINARY_SOURCE"
  exit 1
fi

cp install.sh "$DIST_DIR/spark_midi_bridge/"
cp uninstall.sh "$DIST_DIR/spark_midi_bridge/"
cp README.md "$DIST_DIR/spark_midi_bridge/"

# 4. Create tarball archive containing all folder contents
echo "🗜️  Creating archive '$ARCHIVE_NAME'..."
tar -czf "$ARCHIVE_NAME" -C "$DIST_DIR" spark_midi_bridge

# Clean up temp folder
rm -rf "$DIST_DIR"

echo "========================================================"
echo "✅ Packaging completed successfully!"
echo "👉 Distributable archive created at: $ARCHIVE_NAME"
echo "👉 Transfer it to your Pi and run installation:"
echo "   scp $ARCHIVE_NAME <username>@<pi-ip-address>:/home/<username>/"
echo "========================================================"
