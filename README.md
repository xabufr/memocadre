# Photo Kiosk

A high-performance photo slideshow application designed for digital picture frames, kiosks, and automated displays. Built with Rust for efficiency and reliability, Photo Kiosk delivers smooth transitions and beautiful visuals while running on minimal hardware, from Raspberry Pi to full desktop systems.

## ✨ Features

### 🖼️ Media Management
- **Immich Integration**: Native support for Immich photo servers with multiple instance support
- **Smart Photo Selection**: Multiple source types:
  - **Memory Lane**: Display photos from this day in past years
  - **Smart Search**: AI-powered search using Immich's CLIP features
  - **Random Search**: Filter by persons, locations, or other metadata
  - **Private Albums**: Display specific album collections
- **Extensible Architecture**: Easy to add support for additional photo sources

### 🎨 Visual Features
- **Smooth Transitions**: Configurable crossfade effects between photos
- **Blurred Backgrounds**: Elegant blurred backgrounds for photos that don't fill the screen
- **Captions**: Display photo metadata including location, date, and time
- **Flexible Layouts**: Optimized display for any photo aspect ratio
- **Screen Rotation**: Support for 0°, 90°, 180°, and 270° rotation

### ⚡ Performance & Hardware
- **Hardware Acceleration**: OpenGL-powered rendering for smooth effects
- **Direct Rendering (DRM/KMS)**: Run without X11/Wayland, perfect for embedded systems
- **Low Resource Usage**: Optimized for Raspberry Pi and similar devices
- **Multi-Platform**: Works on Linux desktop (X11) and embedded systems (DRM/KMS)

### 🔧 Configuration & Control
- **YAML Configuration**: Simple, human-readable configuration files
- **HTTP API**: Remote control via REST API
- **MQTT Integration**: IoT integration for smart home systems
- **Runtime Settings**: Adjust slideshow settings without restart
- **Systemd Integration**: Run as a system service with automatic restart

## 📋 Table of Contents

- [Installation](#-installation)
- [Configuration](#-configuration)
- [Usage](#-usage)
- [API Reference](#-api-reference)
- [Development](#-development)
- [Troubleshooting](#-troubleshooting)
- [Contributing](#-contributing)
- [License](#-license)

## 🚀 Installation

### Debian/Ubuntu (Recommended)

Download the appropriate `.deb` package from the [releases page](https://github.com/xabufr/memovue/releases):

**Standard (x86_64, ARM64, ARMv7):**
```bash
sudo dpkg -i photo-kiosk_0.1.0-1_*.deb
sudo apt-get install -f  # Fix any dependency issues
```

**Raspberry Pi 1/Zero (ARMv6):**
```bash
sudo dpkg -i photo-kiosk_0.1.0-1_armhf.deb
sudo apt-get install -f
```

The package includes:
- Binary installed to `/usr/bin/photo-kiosk`
- Configuration files in `/etc/photo-kiosk/`
- Systemd service for automatic startup
- Dedicated `photo-kiosk` user with video group access

### Building from Source

#### Prerequisites
- Rust 1.70 or later ([installation guide](https://www.rust-lang.org/tools/install))
- System dependencies:
  ```bash
  # Debian/Ubuntu
  sudo apt-get install build-essential libgbm-dev
  ```

#### Build Steps
```bash
# Clone the repository
git clone https://github.com/xabufr/memovue.git
cd memovue

# Build release binary
cargo build --release

# Optional: Build with specific features
cargo build --release --no-default-features --features drm  # DRM/KMS only
cargo build --release --no-default-features --features winit  # X11 only
```

The binary will be available at `./target/release/photo-kiosk`.

#### Cross-compilation for Raspberry Pi 1/Zero (ARMv6)

Using [cross](https://github.com/cross-rs/cross):
```bash
cross build --target arm-unknown-linux-gnueabihf --release --no-default-features -F drm
```

Or use the included Justfile:
```bash
just build-armv6
```

## 📝 Configuration

Photo Kiosk uses two YAML configuration files:

1. **`config.yaml`**: Defines photo sources (Immich instances, albums, search queries)
2. **`settings.yaml`**: Controls slideshow behavior (duration, transitions, visual effects)

### Specifying Configuration Files

Configuration file locations can be set via environment variables:
- `CONFIG_PATH`: Path to config.yaml (default: `./config.yaml`)
- `SETTINGS_PATH`: Path to settings.yaml (default: `./settings.yaml`)

For system installations (Debian package):
- Config: `/etc/photo-kiosk/config.yaml`
- Settings: `/etc/photo-kiosk/settings.yaml`

### Photo Sources Configuration (`config.yaml`)

Define where to fetch photos from. See the [example config file](debian/config.yaml) for complete documentation.

#### Basic Immich Setup

```yaml
sources:
  - type: immich
    instances:
      - url: "https://immich.example.com"
        api_key: "YOUR_API_KEY"
    specs:
      - type: memory-lane  # Photos from this day in past years
```

#### Multiple Sources and Instances

```yaml
sources:
  # First Immich instance with multiple specs
  - type: immich
    instances:
      - url: "https://immich1.example.com"
        api_key: "API_KEY_1"
    specs:
      - type: memory-lane
      - type: random-search
        persons:
          - name: "John Doe"
      - type: smart-search
        query: "beach sunset"
        city: "San Diego"
  
  # Second Immich instance
  - type: immich
    instances:
      - url: "https://immich2.example.com"
        api_key: "API_KEY_2"
    specs:
      - type: private-album
        id: "album-uuid-here"
```

#### Available Spec Types

- **`memory-lane`**: Photos from this day in previous years
- **`random-search`**: Filter by persons, date ranges, or metadata
- **`smart-search`**: AI-powered semantic search (requires Immich CLIP)
- **`private-album`**: Display a specific album by ID

### Slideshow Settings (`settings.yaml`)

Control the slideshow appearance and behavior. See the [example settings file](debian/settings.yaml) for complete documentation.

#### Key Settings

```yaml
slideshow:
  display_duration: "30s"        # Time each photo is shown
  transition_duration: "500ms"   # Transition effect duration
  rotation: 0                    # Screen rotation: 0, 90, 180, 270
  
  background:
    type: blur                   # "blur" or "black"
    blur:
      min_free_space: 50         # Minimum % free space to enable blur
  
  blur_options:
    radius: 6.0                  # Blur intensity
    passes: 3                    # Blur quality (more passes = better quality)
  
  caption:
    enabled: true
    font_size: 28
    date_format:
      format: "%A, %e. %B %Y"    # e.g., "Monday, 1. January 2024"
      locale: "en_US"            # Date locale
  
  downscaled_image_filter: lanczos3  # Image scaling quality
  
  debug:
    show_fps: false              # Show FPS counter
```

## 🎯 Usage

### Manual Execution

```bash
# With default config files in current directory
photo-kiosk

# With custom configuration paths
CONFIG_PATH=/path/to/config.yaml SETTINGS_PATH=/path/to/settings.yaml photo-kiosk
```

### System Service (Debian Package)

```bash
# Enable and start the service
sudo systemctl enable photo-kiosk
sudo systemctl start photo-kiosk

# Check status
sudo systemctl status photo-kiosk

# View logs
sudo journalctl -u photo-kiosk -f

# Restart after configuration changes
sudo systemctl restart photo-kiosk

# Stop the service
sudo systemctl stop photo-kiosk
```

### Running on Boot (Raspberry Pi)

The Debian package automatically configures the service to start on boot. The service:
- Runs as the `photo-kiosk` user
- Has access to `/dev/dri` (video group)
- Uses DRM/KMS for direct rendering
- Automatically restarts on failure

## 🔌 API Reference

Photo Kiosk provides HTTP and MQTT interfaces for remote control and monitoring.

### HTTP API

Enable in `settings.yaml`:
```yaml
interfaces:
  http:
    enabled: true
    bind: "127.0.0.1:8080"  # Listen address
```

#### Endpoints

**Get Current Settings**
```bash
curl http://localhost:8080/settings
```

**Update Settings**
```bash
curl -X PATCH http://localhost:8080/settings \
  -H "Content-Type: application/json" \
  -d '{"slideshow": {"display_duration": "60s"}}'
```

**Control Commands**
```bash
# Next photo
curl -X POST http://localhost:8080/control/next

# Previous photo
curl -X POST http://localhost:8080/control/previous

# Reload configuration
curl -X POST http://localhost:8080/control/reload
```

### MQTT Integration

Enable in `settings.yaml`:
```yaml
interfaces:
  mqtt:
    enabled: true
    broker: "mqtt://broker.example.com:1883"
    username: "photo-kiosk"
    password: "your-password"
    topic_prefix: "home/kiosk"  # Optional
```

#### Topics

The application publishes and subscribes to topics based on the device ID (machine UID):

**State Updates (Published)**
- `photokiosk/<device-id>/state`: Current application state
- `photokiosk/<device-id>/settings`: Current settings

**Commands (Subscribed)**
- `photokiosk/<device-id>/control`: Control commands (next, previous, reload)
- `photokiosk/<device-id>/settings`: Settings updates

**Example MQTT Command**
```bash
# Using mosquitto_pub
mosquitto_pub -h broker.example.com \
  -t "photokiosk/<device-id>/control" \
  -m '{"command": "next"}'
```

## 🛠️ Development

### Project Structure

```
src/
├── application/     # Main application logic
│   ├── interfaces/  # HTTP and MQTT interfaces
│   └── slideshow/   # Slideshow controller and transitions
├── configuration/   # Configuration parsing and validation
├── gallery/         # Photo source implementations
│   └── immich/      # Immich integration
├── gl/              # OpenGL wrapper and utilities
├── graphics/        # Rendering engine (blur, image display)
├── support/         # Platform support (DRM/KMS, Winit)
└── worker.rs        # Background photo fetching
```

### Building and Testing

```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run tests
cargo test

# Run with specific features
cargo run --no-default-features --features drm   # DRM/KMS only
cargo run --no-default-features --features winit # X11 only

# Format code
cargo fmt

# Lint
cargo clippy -- -D warnings
```

### Creating a Development Build

```bash
cargo build --profile profiling  # Release with debug symbols
```

### Contributing

Contributions are welcome! Areas for improvement:
- Additional photo source implementations (Google Photos, local filesystem, etc.)
- More transition effects
- Additional layout options (multi-photo displays)
- Performance optimizations
- Documentation improvements

Please ensure:
- Code follows existing style (run `cargo fmt`)
- All tests pass (`cargo test`)
- New features include appropriate documentation

## 🐛 Troubleshooting

### Common Issues

**"Permission denied" on DRM/KMS**
```bash
# Add user to video group
sudo usermod -a -G video $USER
# Or for the service user
sudo usermod -a -G video photo-kiosk
```

**Service fails to start**
```bash
# Check logs for errors
sudo journalctl -u photo-kiosk -n 50

# Verify configuration syntax
photo-kiosk --validate-config  # If implemented

# Test manually
sudo -u photo-kiosk CONFIG_PATH=/etc/photo-kiosk/config.yaml photo-kiosk
```

**No photos displayed**
- Verify Immich URL and API key in config.yaml
- Check that the specified album/search returns photos
- Review logs for network or authentication errors
- Ensure Immich instance is accessible from the device

**Poor performance on Raspberry Pi**
- Reduce blur radius and passes in settings.yaml
- Increase `display_duration` to reduce transitions
- Use `black` background instead of `blur`
- Disable caption if not needed

**Screen rotation not working**
- Set `rotation` in settings.yaml (0, 90, 180, or 270)
- Restart the service after changes
- Verify DRM/KMS is being used (check logs)

### Debug Mode

Enable debug features for troubleshooting:
```yaml
slideshow:
  debug:
    show_fps: true  # Display FPS counter
```

Run with debug logging:
```bash
RUST_LOG=debug photo-kiosk
```

## 📄 License

This project is licensed under the MIT License. See the [Cargo.toml](Cargo.toml) for details.

```
MIT License - Copyright (c) Thomas Loubiou <xabufr@gmail.com>
```

## 🙏 Acknowledgments

Built with:
- [Rust](https://www.rust-lang.org/) - Systems programming language
- [Immich](https://immich.app/) - Self-hosted photo management
- [OpenGL](https://www.opengl.org/) - Graphics rendering
- [DRM/KMS](https://www.kernel.org/doc/html/latest/gpu/drm-kms.html) - Direct rendering for Linux

## 📬 Contact & Support

- **Issues**: [GitHub Issues](https://github.com/xabufr/memovue/issues)
- **Author**: Thomas Loubiou ([@xabufr](https://github.com/xabufr))

---

**Note**: This project was previously known as "PhotoKiosk" and may be referenced as such in some documentation or code comments.
