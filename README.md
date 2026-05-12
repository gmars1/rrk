# Keyboard Layout Optimizer

Tracks which keys you press most and displays a live heat map. Helps you decide which layout (QWERTY, Colemak, etc.) suits your typing patterns.

## Installation

### Linux (x86_64)

**Arch:**
```sh
# Dependencies
sudo pacman -S rust libevdev libglvnd glib2 libxkbcommon wayland \
  libx11 libxcursor libxi libxrandr mesa

# Build
git clone https://github.com/YOUR_USER/keyboard-letters-manager
cd keyboard-letters-manager
cargo build --release
```

**Ubuntu / Debian:**
```sh
sudo apt install libevdev-dev libegl1-mesa-dev libgles2-mesa-dev \
  libglib2.0-dev libxkbcommon-dev libwayland-dev libx11-dev \
  libxcursor-dev libxi-dev libxrandr-dev

git clone https://github.com/YOUR_USER/keyboard-letters-manager
cd keyboard-letters-manager
cargo build --release
```

Pre-built binaries are available from [GitHub Releases](https://github.com/YOUR_USER/keyboard-letters-manager/releases).

### Windows

Download `key-optimizer-windows.exe` from Releases and run it.

### macOS

Currently not implemented (platform listener is a stub).

## Usage

### Linux (native)

**Daemon mode** — collects keystrokes in the background:
```sh
# Make sure you're in the input group for evdev access
sudo usermod -a -G input $USER
# Log out and back in, then:
./target/release/key-optimizer --daemon &

# Later, open the UI to see stats:
./target/release/key-optimizer
```

**UI-only mode** — captures keystrokes through the window:
```sh
./target/release/key-optimizer
```

If you're not in the `input` group, the daemon will idle (no data), but UI mode still works via window input.

### WSL2 (Windows Subsystem for Linux)

WSL has no `/dev/input/` devices, so daemon mode won't capture keystrokes. Use UI mode instead:

```sh
./target/release/key-optimizer
```

Type in the window — all keystrokes are captured through WSLg/Wayland.

### Windows

Run the `.exe` — keystrokes are captured through the window.

## Features

| Feature | Linux | WSL | Windows |
|---------|-------|-----|---------|
| Daemon (background) | ✓ (evdev) | ✗ | ✗ |
| UI live capture | ✓ | ✓ | ✓ |
| Heat map | ✓ | ✓ | ✓ |
| Layout switching | ✓ | ✓ | ✓ |

- **Heat map** — keys turn red (most used) to green (least used)
- **Layout switcher** — toggle between QWERTY and Colemak; stats remap automatically
- **Persistence** — stats saved to `~/.config/key-optimizer/stats.json`, loaded on restart
- **Daemon** — `--daemon` flag runs headlessly; saves every 30s; Ctrl+C for clean shutdown

## Layouts

Built-in: QWERTY (US), Colemak.

Add your own by placing a JSON file in the `layouts/` directory (format matches the built-in layouts).

## Building

```sh
git clone https://github.com/YOUR_USER/keyboard-letters-manager
cd keyboard-letters-manager
cargo build --release
./target/release/key-optimizer --help
```
