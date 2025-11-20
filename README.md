# MemoCadre

PhotoKiosk displays photos in a slideshow format,
Perfect for digital picture frames, kiosks, and automated displays. 
Currently only supports Immich, with easy extensibility for other sources.

## Features

* **Media Sources:** Immich servers (easily extendable).
* **Slideshow Control:** Configurable duration, transitions, initial slide.
* **Visuals:** Blurred backgrounds, smooth transitions.
* **Meta** Displays city, date (if available).
* **Hardware Acceleration:** OpenGL for efficient effects.
* **Direct Rendering (DRM/KMS):** No X/Wayland, ideal for embedded systems.
* **YAML Configuration:** Simple, flexible configuration.

## Configuration

PhotoKiosk uses a YAML configuration file.
Specify the path with environment variable `CONFIG_PATH=/path/to/config.yaml`.
If omitted, it defaults to `config.yaml` in the current directory.
Please refer to `debian/config.yaml` for an example configuration file.

## Usage

1. **Install:** See instructions below.
2. **Configure:** Edit `config.yaml` for your media and slideshow preferences.
3. **Run:** `CONFIG_PATH=/path/to/config.yaml photo-kiosk` (or `photo-kiosk` if `config.yaml` is in the current directory).

## Installation

### Debian-based OS (including Raspberry Pi OS)

1. **Download:** Get the `.deb` package from the releases page.
2. **Install:** `sudo dpkg -i <package_name>.deb`
3. **Fix Dependencies (if needed):** `sudo apt-get install -f`
4. **Configure:** Edit `/etc/photo-kiosk/config.yaml`.
5. **Start Service:** `sudo systemctl start photo-kiosk && sudo systemctl enable photo-kiosk`

### Raspberry Pi 1 (armv6)

Use the Debian-based OS instructions with the armv6 `.deb` package.

### Building from Source

1. **Install Rust:** [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install)
2. **Clone:** `git clone https://github.com/xabufr/photo-kiosk.git`
3. **Navigate:** `cd photo-kiosk`
4. **Build:** `cargo build --release`
5. **Run:** `./target/release/photo-kiosk`
