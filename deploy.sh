#!/bin/bash
# Spark MIDI Bridge deployment script.
# Copies the packaged tarball to the Raspberry Pi.

set -e

# Load custom environment variables if .env exists
if [ -f .env ]; then
  echo "ℹ️  Loading configuration from .env file..."
  source .env
fi

# Defaults (customized for your target board)
DEFAULT_IP=${PI_IP:-"192.168.0.208"}
DEFAULT_USER=${PI_USER:-"jmsalazardev"}
ARCHIVE_NAME=${ARCHIVE_NAME:-"spark_midi_bridge.tar.gz"}

# Allow override via command line arguments
PI_IP=${1:-$DEFAULT_IP}
PI_USER=${2:-$DEFAULT_USER}

echo "========================================================"
echo "🚀 Deploying Spark MIDI Bridge to Raspberry Pi"
echo "========================================================"

# Auto-package if the tarball is missing
if [ ! -f "$ARCHIVE_NAME" ]; then
  echo "⚠️  Package '$ARCHIVE_NAME' not found."
  echo "   Triggering build and packaging first..."
  if [ -f "package.sh" ]; then
    ./package.sh
  else
    echo "❌ Error: package.sh not found in the current directory."
    exit 1
  fi
fi

echo "📤 Transferring '$ARCHIVE_NAME' to $PI_USER@$PI_IP..."
scp "$ARCHIVE_NAME" "$PI_USER@$PI_IP:/home/$PI_USER/"

echo "========================================================"
echo "✅ Transfer completed successfully!"
echo "========================================================"
echo "👉 Next steps (run on your Raspberry Pi):"
echo "   1. SSH into the Pi:"
echo "      ssh $PI_USER@$PI_IP"
echo ""
echo "   2. Extract the archive:"
echo "      tar -xzf $ARCHIVE_NAME"
echo ""
echo "   3. Run the installer:"
echo "      sudo ./install.sh"
echo "========================================================"
