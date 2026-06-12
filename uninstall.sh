#!/bin/bash
# Spark MIDI Bridge automated uninstaller script.
# Must be run with sudo privileges.

set -e

# Ensure script is run with sudo
if [ "$EUID" -ne 0 ]; then
  echo "❌ Please run this uninstaller with sudo: sudo ./uninstall.sh"
  exit 1
fi

INSTALL_DIR="/opt/spark-midi-bridge"
SERVICE_FILE="/etc/systemd/system/spark_midi_bridge.service"

echo "========================================================"
echo "🧹 Uninstalling Spark MIDI Bridge"
echo "========================================================"

# 1. Stop and disable Systemd service
if systemctl is-active --quiet spark_midi_bridge.service 2>/dev/null; then
  echo "🛑 Stopping spark_midi_bridge service..."
  systemctl stop spark_midi_bridge.service
fi

if systemctl is-enabled --quiet spark_midi_bridge.service 2>/dev/null; then
  echo "🔌 Disabling spark_midi_bridge service..."
  systemctl disable spark_midi_bridge.service
fi

# 2. Remove Systemd service file
if [ -f "$SERVICE_FILE" ]; then
  echo "🗑️ Removing systemd service file..."
  rm -f "$SERVICE_FILE"
fi

# 3. Reload systemd daemon
echo "🔄 Reloading systemd configuration..."
systemctl daemon-reload
systemctl reset-failed

# 4. Remove installation files
if [ -d "$INSTALL_DIR" ]; then
  REMOVE_DIR=false
  # If -y or --force is passed, or if we ask the user in an interactive terminal
  if [[ "$1" == "-y" || "$1" == "--force" ]]; then
    REMOVE_DIR=true
  elif [ -t 0 ]; then
    read -p "❓ Do you want to remove the installation directory $INSTALL_DIR (this deletes spark_config.json)? [y/N]: " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
      REMOVE_DIR=true
    fi
  fi

  if [ "$REMOVE_DIR" = true ]; then
    echo "🗑️ Removing directory $INSTALL_DIR..."
    rm -rf "$INSTALL_DIR"
  else
    echo "💾 Keeping directory $INSTALL_DIR (configuration saved)."
  fi
fi

echo "========================================================"
echo "✅ Uninstallation completed successfully!"
echo "========================================================"
