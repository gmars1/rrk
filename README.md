# key-optimizer

Keyboard layout heatmap tracker and layout optimizer. Records your typing patterns, displays a live color-coded heatmap, and uses simulated annealing to find better key arrangements.

## OS support overview

| Feature | Linux | macOS | Windows |
|---|---|---|---|
| GUI heatmap viewer | ✅ | ✅ | ✅ |
| Background daemon (real keystrokes) | ✅ evdev | ❌ stub | ❌ stub |
| UI live capture (typing into window) | ✅ | ✅ | ✅ |
| Layout switching (QWERTY / Colemak) | ✅ | ✅ | ✅ |
| Layout optimization | ✅ | ✅ | ✅ |

**Note:** macOS and Windows platform listeners are stubs that emit dummy events. Real keystroke capture is only implemented on Linux. The GUI still works everywhere — you can type into the window and see heatmap updates.

## Build

Requires [Rust](https://rustup.rs/) (edition 2021).

```sh
git clone <repo-url>
cd keyboard_letters__manager
cargo build --release
```

Binary: `target/release/key-optimizer` (or `key-optimizer.exe` on Windows).

### Linux — system dependencies

**Ubuntu / Debian:**
```sh
sudo apt install -y \
  libevdev-dev \
  libegl1-mesa-dev libgles2-mesa-dev \
  libglib2.0-dev \
  libxkbcommon-dev libwayland-dev \
  libx11-dev libxcursor-dev libxi-dev libxrandr-dev
```

**Arch:**
```sh
sudo pacman -S --noconfirm \
  rust libevdev libglvnd glib2 libxkbcommon wayland \
  libx11 libxcursor libxi libxrandr mesa
```

### macOS

```sh
xcode-select --install
```

No extra libraries needed.

### Windows

Install Rust via [rustup.rs](https://rustup.rs/). Build with the MSVC toolchain — no extra libraries needed.

## Usage

### GUI mode (all OS)

```sh
key-optimizer
```

Opens a window with a color-coded keyboard (red = heavy use, green = light use). Type into the window to feed it text. Switch between QWERTY and Colemak with the dropdown.

### Daemon mode (Linux only)

```sh
key-optimizer --daemon
```

Collects real keystrokes in the background. Requires read access to `/dev/input/event*`:
```sh
sudo usermod -a -G input $USER
# log out and back in, then run the daemon
```

Stats auto-save every 30 seconds. Stop with Ctrl+C. Launch the GUI later to view the collected data.

### WSL2

No `/dev/input/` devices, so daemon mode is unavailable. GUI mode works through WSLg:

```sh
key-optimizer
```

## Data

Stats persist as JSON at:
- **Linux:** `~/.config/key-optimizer/stats.json`
- **macOS:** `~/Library/Application Support/key-optimizer/stats.json`
- **Windows:** `C:\Users\<you>\AppData\Roaming\key-optimizer\stats.json`

Delete or edit this file to reset or inject stats manually.

## Layouts

Built-in: QWERTY (US) and Colemak. Layouts are JSON files embedded at compile time from `src/assets/layouts/`.
