#!/bin/bash
# Spark MIDI Bridge build script.
# Compiles the program for the configured target architecture.

set -e

# Load custom environment variables if .env exists
if [ -f .env ]; then
  echo "ℹ️  Loading configuration from .env file..."
  source .env
fi

# Configuration defaults
TARGET=${TARGET:-"arm-unknown-linux-gnueabihf"}

echo "========================================================"
echo "🔨 Building Spark MIDI Bridge"
echo "========================================================"
echo "🎯 Target Architecture: $TARGET"

# Check for cross-compilation tool
if ! command -v cross &> /dev/null; then
  echo "❌ Error: 'cross' command-line tool is not installed."
  echo "   Install it with: cargo install cross --git https://github.com/cross-rs/cross"
  exit 1
fi

# Build in release mode
cross build --target "$TARGET" --release

echo "========================================================"
echo "✅ Build completed successfully!"
echo "👉 Binary generated at: target/$TARGET/release/spark_midi_bridge"
echo "========================================================"
