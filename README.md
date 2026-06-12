# Spark MIDI Bridge

The **Spark MIDI Bridge** is a lightweight application designed to bridge MIDI controllers (such as foot pedals) with Positive Grid **Spark** amplifiers. It allows guitarists to control presets and settings on their Spark amps using generic MIDI hardware.

The application is written in Rust and highly optimized to run reliably on low-resource hardware like the **Raspberry Pi Zero (ARMv6 / Single-Core)**.

---

## 🔌 Compatibility & Tested Hardware

Currently, this bridge has been tested and verified to work with the following setup:
* **Host Device**: Raspberry Pi Zero
* **Amplifier**: Positive Grid Spark 40
* **MIDI Pedal**: M-vave Chocolate Plus

### 🤝 Call for Collaboration (Support for Additional Devices)
Since we want this bridge to be useful for the entire Spark community, **we need your help!** If you own other hardware, please consider collaborating:
* **Test other Spark Amplifiers**: Help us test compatibility and report success or issues with **Spark Mini**, **Spark GO**, or **Spark LIVE**.
* **Test other MIDI Pedals**: Verify if other USB or BLE MIDI pedals work with the interactive configuration wizard.
* **Contribute**: Pull requests are highly welcome! Feel free to submit code improvements, bug fixes, or mappings for new controllers.

---

## 📁 Project Structure

The project follows a modular architecture:
- [src/main.rs](src/main.rs): Entry point of the application, responsible for loading the configuration and spawning/joining background engines.
- [src/config.rs](src/config.rs): Handles strongly-typed configurations (`AppConfig`), parsing them from `spark_config.json`, and translating button keys.
- [src/spark.rs](src/spark.rs): Manages Bluetooth BLE communication, Spark amplifier connections, preset payloads mapping, and connection drop safety intervals.
- [src/midi.rs](src/midi.rs): Manages MIDI pedal connection, monitors disconnection via a watchdog loop, and captures/translates incoming MIDI events.
- [src/led.rs](src/led.rs): Manages the GPIO status LED, triggering slow blinks when configuring/pairing, or solid ON when both connections are established.
- [src/configurator.rs](src/configurator.rs): Operates the interactive configuration wizard CLI menu for initial device selection and pedal button mappings.

---

## 🛠️ 1. Compilation & Targets

The application can be compiled for several target architectures. To compile without dynamic linking issues (such as Glibc or D-Bus version mismatches) when cross-compiling, we recommend using `cross` and Docker.

### 🎯 Supported Targets

* **Raspberry Pi Zero (ARMv6 / Single-Core)**: `arm-unknown-linux-gnueabihf` (runs the bridge in headless/service mode)
* **Windows (64-bit)**: `x86_64-pc-windows-gnu` (runs the bridge on a Windows PC with native Bluetooth LE MIDI support)
* **Linux x86_64 (64-bit)**: `x86_64-unknown-linux-gnu` (runs the bridge on a 64-bit Linux desktop/server or Raspberry Pi 4/5 running a 64-bit OS)

### Prerequisites (for Cross-Compilation)
Make sure you have Docker running and `cross` installed:
```bash
cargo install cross --git https://github.com/cross-rs/cross
```

### 🍓 Compiling for Raspberry Pi Zero (ARMv6)
You can compile the binary for your configured target using the automated build script:
```bash
chmod +x build.sh
./build.sh
```
*(By default, it uses target `arm-unknown-linux-gnueabihf`. You can change this target in your `.env` file)*

Alternatively, run `cross` directly:
```bash
cross build --target arm-unknown-linux-gnueabihf --release
```
The compiled binary will be generated at:
`target/arm-unknown-linux-gnueabihf/release/spark_midi_bridge`

### 🖥️ Compiling for Windows
The Windows build automatically integrates native Bluetooth MIDI (BLE MIDI) support via the Windows WinRT API.

#### Method A: Cross-Compiling from Linux (using `cross`)
```bash
cross build --target x86_64-pc-windows-gnu --release
```
The compiled executable will be generated at:
`target/x86_64-pc-windows-gnu/release/spark_midi_bridge.exe`

#### Method B: Natively on Windows
Install Rust on Windows and run the following command in terminal/command prompt:
```cmd
cargo build --release
```
The compiled executable will be generated at:
`target\release\spark_midi_bridge.exe`

### 🐧 Compiling for Linux x86_64
You can compile the binary to run natively on a Linux desktop/server or 64-bit Raspberry Pi.

#### Method A: Natively on Linux x86_64
If you are already on a Linux x86_64 machine, run:
```bash
cargo build --release
```
The compiled binary will be generated at:
`target/release/spark_midi_bridge`

#### Method B: Cross-Compiling from Linux to another target architecture
```bash
cross build --target x86_64-unknown-linux-gnu --release
```
The compiled binary will be generated at:
`target/x86_64-unknown-linux-gnu/release/spark_midi_bridge`

### 📦 Package a Distributable Bundle (Recommended)
Alternatively, you can compile the binary and bundle it automatically with the `install.sh` and `uninstall.sh` helper scripts into a single compressed tarball `spark_midi_bridge.tar.gz` by running:
```bash
chmod +x package.sh
./package.sh
```
This is the easiest way to prepare the app for deployment.

---

## 📦 2. Transfer and Installation to the Raspberry Pi

An automated installation script (`install.sh`) is provided to handle the directory setup, file copying, and Systemd service configuration on the Raspberry Pi.

### Step A: Transfer the files to the Raspberry Pi (from your host)
You can use the deployment helper script `deploy.sh` to copy the tarball to your Pi automatically:
```bash
chmod +x deploy.sh
./deploy.sh
```
*(By default, the script reads configuration variables like `PI_IP` and `PI_USER` from the `.env` file. You can copy `.env.example` to `.env` and customize these variables for your environment. Alternatively, you can override them directly by passing arguments: `./deploy.sh <ip-address> <username>`)*

Alternatively, copy the files manually using `scp`:
```bash
scp spark_midi_bridge.tar.gz <username>@<pi-ip-address>:/home/<username>/
```

### Step B: Run the installer script (on the Pi)
SSH into your Raspberry Pi. If you transferred the tarball, extract it first:
```bash
tar -xzf spark_midi_bridge.tar.gz
```
Make the script executable and run it with `sudo`:
```bash
chmod +x install.sh
sudo ./install.sh
```
The script will automatically stop any running instance, copy the binary to `/opt/spark-midi-bridge/`, generate the systemd service matching your user, reload systemd, and enable it on boot.

---

## ⚙️ 3. Interactive Configuration Wizard

To pair your Spark amplifier and map your MIDI pedal buttons for the first time, navigate to the installation folder and run the interactive configuration wizard on the Raspberry Pi:

```bash
cd /opt/spark-midi-bridge
./spark_midi_bridge --configure
```

The configurator runs a step-by-step wizard to guide you through the initial setup:

1. **Step 1/3 (Pair with Spark amplifier)**: Scans for nearby Spark amplifiers over Bluetooth BLE. Once scanned, you select your amplifier from the list to register its MAC address and name.
2. **Step 2/3 (Pair with MIDI pedal)**: Scans for and lists all available MIDI input devices (USB MIDI hardware or Bluetooth BLE MIDI devices like the M-Vave Chocolate). Select your pedal from the list (e.g., `FootCtrlPlus`).
3. **Step 3/3 (Map pedal buttons)**: Sequentially prompts you to press the button on your pedal that you wish to assign to each preset (Preset 1 through 4). You can press **Enter** on your keyboard to skip mapping any preset. The wizard completes the mapping step automatically after processing all 4 presets.

### Wizard Actions:
After Step 3, a **Configuration Summary** is displayed showing your paired devices and current button mappings. You will then be prompted to select an action using your keyboard's arrow keys:
* **Save & Run**: Saves all settings to `spark_config.json` and immediately launches the bridge (starts the Spark BLE and MIDI listener loops).
* **Save & Exit**: Saves all settings to `spark_config.json` and exits the program (ideal if you only wanted to configure it for a background service).
* **Start over**: Discards the current selections and restarts the wizard from Step 1.
* **Exit (without saving)**: Exits the configurator immediately and discards any changes.


---

## 🔄 4. Running as a Systemd Service

The automated installer automatically registers and enables the service. You can control it using standard service management commands:

### Commands:
* **Check current status**: `sudo systemctl status spark_midi_bridge.service`
* **Monitor logs in real-time**: `journalctl -u spark_midi_bridge.service -f`
* **Stop the service**: `sudo systemctl stop spark_midi_bridge.service`
* **Restart the service**: `sudo systemctl restart spark_midi_bridge.service`

---

## 🧹 5. Cleaning Up Old Services

If you had previously created systemd services for the older Python scripts, you can stop, disable, and clean them up completely by running:

```bash
# Stop and disable old services
sudo systemctl stop pgsparklite.service sparkmidi.service
sudo systemctl disable pgsparklite.service sparkmidi.service

# Remove old service files
sudo rm /etc/systemd/system/pgsparklite.service
sudo rm /etc/systemd/system/sparkmidi.service

# Apply changes to systemd
sudo systemctl daemon-reload
sudo systemctl reset-failed
```

---

## 🗑️ 6. Uninstalling the Bridge

To completely remove the service and configuration from your Raspberry Pi, use the automated uninstaller:

### Step A: Transfer the uninstaller (if not already present)
```bash
scp uninstall.sh <username>@<pi-ip-address>:/home/<username>/
```

### Step B: Run the uninstaller (on the Pi)
```bash
chmod +x uninstall.sh
sudo ./uninstall.sh
```
Add `-y` or `--force` to automatically delete the `/opt/spark-midi-bridge` directory and your `spark_config.json` configuration without prompting.

---

## 💡 7. Physical Status LED (Optional)

The application supports controlling a physical status LED connected directly to the Raspberry Pi GPIO pins to give visual feedback on connection status:
* **Blinking (500ms intervals)**: Scanning or attempting to connect to either the Spark amplifier or the MIDI pedal.
* **Solid ON**: Both devices are successfully connected and the bridge is active and ready.

### Wiring
* Connect the **positive leg (Anode)** of your LED to **GPIO Pin 17** (physical pin 11 on the Raspberry Pi header) through an appropriate resistor (e.g., 220Ω or 330Ω).
* Connect the **negative leg (Cathode)** to a **GND pin** (e.g., physical pin 9, 14, or 25).

### Configuration
By default, the LED uses **GPIO 17**. If you wish to use a different pin, add an `"led_pin"` key with the desired BCM/GPIO pin number to your `spark_config.json`:
```json
{
  "spark_mac": "F7:EB:ED:3B:CF:6C",
  "spark_name": "Spark 40",
  "midi_name": "FootCtrlPlus",
  "led_pin": 18,
  "btn20": "Preset 1",
  "btn21": "Preset 2",
  "btn22": "Preset 3",
  "btn23": "Preset 4"
}
```
*Note: The pin number refers to the Broadcom (BCM) GPIO numbering, not the physical pin number on the board header.*

---

## 🔧 8. Troubleshooting Bluetooth & MIDI

If the configuration wizard fails to scan or detect your devices, verify the following system configurations:

### A. User Group Permissions
Ensure your user has direct access to the audio (MIDI) and bluetooth (D-Bus) systems without root privileges:
```bash
sudo usermod -aG audio,bluetooth $USER
```
*Note: You must restart the Pi or log out/in for group permissions to take effect.*

### B. Bluetooth Adapter Status
If the scanner returns immediately with `No Spark amplifiers found nearby`, check if your Bluetooth interface is blocked or shut down:

1. Check if the adapter is soft-blocked:
   ```bash
   rfkill list
   ```
   If it is blocked, run:
   ```bash
   sudo rfkill unblock bluetooth
   ```

2. Check if the adapter is down:
   ```bash
   hciconfig
   ```
   If it lists the adapter (e.g. `hci0`) as `DOWN`, turn it on:
   ```bash
   sudo hciconfig hci0 up
   ```

3. Restart the Bluetooth service to apply changes:
   ```bash
   sudo systemctl restart bluetooth
   ```

### C. USB MIDI Hardware Detection
If your MIDI pedal is not detected:
1. Verify the hardware connection:
   ```bash
   lsusb
   ```
   You should see your USB MIDI pedal interface listed. If not, verify the USB OTG / data cable connection.
2. Verify if ALSA registers the device:
   ```bash
   amidi -l
   ```

### D. Pairing M-Vave Chocolate (BLE MIDI) Pedal
The M-Vave Chocolate MIDI pedal communicates over Bluetooth BLE MIDI. To use it with the bridge on Linux (Raspberry Pi), you need to pair and connect it via `bluetoothctl` first:

1. Open `bluetoothctl`:
   ```bash
   sudo bluetoothctl
   ```

2. Start scanning for Bluetooth devices:
   ```bluetoothctl
   scan on
   ```

3. Look for a device named **`FootCtrlPlus`** (this is how the M-Vave Chocolate pedal identifies itself) and copy its MAC address (for example, `5D:06:47:86:E3:35`).

4. Pair the pedal (replace `5D:06:47:86:E3:35` with your pedal's actual MAC address):
   ```bluetoothctl
   pair 5D:06:47:86:E3:35
   ```

5. Trust the pedal so it reconnects automatically on boot:
   ```bluetoothctl
   trust 5D:06:47:86:E3:35
   ```

6. Connect the pedal:
   ```bluetoothctl
   connect 5D:06:47:86:E3:35
   ```

7. Exit `bluetoothctl`:
   ```bluetoothctl
   exit
   ```

Once connected, ALSA will expose the BLE MIDI device through the **ALSA Sequencer** system. Note that Bluetooth BLE MIDI devices will **not** appear in `amidi -l` (which only lists hardware/USB RawMIDI devices and returns `cannot determine device number: Inappropriate ioctl for device` when no USB MIDI devices are connected).

To verify the connection, list the ALSA Sequencer ports:
```bash
aconnect -l
```
You should see `FootCtrlPlus` listed in the output. You can then run the configuration wizard (`./spark_midi_bridge --configure` from `/opt/spark-midi-bridge`) and select `FootCtrlPlus` (during Step 2/3) to configure and map the pedal.

### E. Bluetooth MIDI (BLE MIDI) on Windows

The Windows build utilizes the Windows WinRT (UWP) MIDI backend natively (enabled via the `winrt` feature of `midir` in Cargo.toml).

To use a Bluetooth MIDI pedal (such as the M-Vave Chocolate) on Windows:
1. Open Windows Settings, navigate to **Bluetooth & devices**, and pair/connect your pedal (e.g., `FootCtrlPlus`).
2. Verify that the device status shows as **Connected** before starting the bridge or the configurator.
3. Run the configuration wizard: `spark_midi_bridge.exe --configure`
4. In **Step 2/3**, the WinRT MIDI backend will scan and list your connected Bluetooth MIDI device.


