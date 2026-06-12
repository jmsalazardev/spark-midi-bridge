#!/bin/bash
# Spark MIDI Bridge automated installer script for Raspberry Pi.
# Must be run with sudo privileges.

set -e

# Ensure script is run with sudo
if [ "$EUID" -ne 0 ]; then
  echo "❌ Please run this installer with sudo: sudo ./install.sh"
  exit 1
fi

# Get the actual user who invoked sudo to run as service owner
REAL_USER=${SUDO_USER:-$USER}
INSTALL_DIR="/opt/spark-midi-bridge"
BINARY_NAME="spark_midi_bridge"
SERVICE_FILE="/etc/systemd/system/spark_midi_bridge.service"

echo "========================================================"
echo "🔧 Installing Spark MIDI Bridge"
echo "========================================================"

# 1. Create target folder and set owner permissions
echo "📁 Setting up installation directory at $INSTALL_DIR..."
mkdir -p "$INSTALL_DIR"
chown -R "$REAL_USER:$REAL_USER" "$INSTALL_DIR"

# 2. Check and copy the binary if present in current directory
if [ -f "$BINARY_NAME" ]; then
  echo "🚀 Copying '$BINARY_NAME' binary..."
  cp "$BINARY_NAME" "$INSTALL_DIR/"
  chmod +x "$INSTALL_DIR/$BINARY_NAME"
  chown "$REAL_USER:$REAL_USER" "$INSTALL_DIR/$BINARY_NAME"
else
  echo "⚠️  Warning: '$BINARY_NAME' binary not found in the current folder."
  echo "   Make sure to place the compiled binary next to this script and rerun to complete setup."
fi

# 3. Create Systemd Service configuration
echo "📝 Generating systemd service at $SERVICE_FILE..."
cat <<EOF > "$SERVICE_FILE"
[Unit]
Description=Spark MIDI Bridge
After=bluetooth.target sound.target dbus.service
Wants=bluetooth.target

[Service]
Type=simple
User=$REAL_USER
WorkingDirectory=$INSTALL_DIR
ExecStart=$INSTALL_DIR/$BINARY_NAME
Restart=always
RestartSec=5
Environment=RUST_LOG=info
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

# 4. Reload and enable the systemd service
echo "🔄 Reloading systemd daemon and enabling service..."
systemctl daemon-reload
systemctl enable spark_midi_bridge.service

echo "========================================================"
echo "✅ Installation completed successfully!"
echo "========================================================"
echo "👉 1. Run the interactive configurator to pair devices:"
echo "      cd $INSTALL_DIR"
echo "      ./$BINARY_NAME --configure"
echo ""
echo "👉 2. Start the background service:"
echo "      sudo systemctl start spark_midi_bridge.service"
echo ""
echo "👉 3. Monitor live log outputs:"
echo "      journalctl -u spark_midi_bridge.service -f"
echo "========================================================"
