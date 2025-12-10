# Taskbar Hider

A minimal Windows utility that hides the taskbar by default and shows it only when needed.

## Features

- Hides the Windows taskbar on startup
- Shows the taskbar when the Windows key is held down
- Keeps the taskbar visible when the Start menu or system windows are active
- System tray icon with right-click quit option
- Automatically recovers if Explorer restarts
- Tiny footprint (~250KB)

## Requirements

- Windows 10/11
- Rust toolchain (for building from source)

## Installation

### Pre-built Binary

Download the latest release from the [Releases](https://github.com/yourusername/taskbar-hider/releases) page.

### Build from Source

**On Windows:**
```bash
cargo build --release
```

**Cross-compile from Linux (WSL):**
```bash
# Install the Windows target
rustup target add x86_64-pc-windows-gnu

# Install mingw-w64 (Ubuntu/Debian)
sudo apt-get install mingw-w64

# Build
cargo build --release --target x86_64-pc-windows-gnu
```

The executable will be at `target/release/taskbar-hider.exe` (or `target/x86_64-pc-windows-gnu/release/taskbar-hider.exe` for cross-compilation).

## Usage

1. Run `taskbar-hider.exe`
2. The taskbar will hide automatically
3. Press and hold the Windows key to show the taskbar
4. The taskbar stays visible while the Start menu or system windows are open
5. Right-click the system tray icon and select "Quit" to exit

## How It Works

- Uses `SetWindowsHookEx` with `WH_KEYBOARD_LL` to detect Windows key press/release
- Registers a shell hook via `RegisterShellHookWindow` to detect Start menu and system window activation
- Hides the taskbar using `ShowWindow` with `SW_HIDE` and enables auto-hide mode via `SHAppBarMessage`
- Keeps the taskbar visible for 400ms after Windows key release to allow the Start menu to activate

## License

MIT License - see [LICENSE](LICENSE) for details.
