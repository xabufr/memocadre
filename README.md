# MemoCadre

MemoCadre displays photos as a slideshow, ideal for digital picture frames,
kiosks, and dedicated screens.  
It currently uses an **Immich** server as its media source, but the architecture
allows adding more backends in the future.

MemoCadre focuses on lightweight performance and smooth visuals, making it
suitable for continuous operation on low-power hardware (like Raspberry Pi 1 /
Zero). It supports running without a display server (using DRM/KMS) for embedded
use cases and offers integration with home automation systems via MQTT/HTTP.

---

## Quick start

This assumes:

- you already have an **Immich** server running and reachable, and
- you are on a Linux machine (or Raspberry Pi) with OpenGL ES / DRM/KMS support.

1. **Install MemoCadre**: `sudo dpkg -i memo-cadre_0.1.0-1_armhf.deb`
2. **Fix dependencies (if needed)**: `sudo apt-get install -f`
3. **Update `/etc/memocadre/config.yaml`** (app configuration):

   ```yaml
   # /etc/memocadre/config.yaml
   sources:
     - type: immich
       instance:
         url: https://immich.example.com
         api_key: "YOUR_IMMICH_API_KEY"
       specs:
        # Simple random searches by person names, here we retrieve random photos of:
        # - Alice
        # - Bob
        # - Dad and Mom together
        - type: random-search
            persons:
            - name: Alice
        - type: random-search
            persons:
            - name: Bob
        - type: random-search
            persons:
            - name: Dad
            - name: Mom
   ```

4. (Optional) **Update `/etc/memocadre/settings.yaml`** (runtime settings):
5. **Run and enable MemoCadre**:

   ```bash
   sudo systemctl start memocadre && sudo systemctl enable memocadre
   ```

You should see a slideshow of photos coming from your Immich instance, with
blurred backgrounds and captions.

---

## Features

- **Media source:**

  - Connects to an **Immich** server (random search, smart search, private
    albums, Memory Lane, etc.).
  - Designed to be extensible to other backends (not planned in the short term,
    but contributions are welcome).

- **Display & transitions:**

  - Displays **one photo at a time**.
  - **Configurable transitions** between photos.
  - **Gaussian-blurred background** for photos that are smaller than the frame.
  - Handles **screen rotations at 0°, 90°, 180°, and 270°**.

- **Visuals & metadata:**

  - Blurred background based on the current photo.
  - Optional **metadata captions**:
    - city
    - date (when available, with configurable format and locale).
  - Smooth, GPU-accelerated rendering.

- **Performance & hardware:**

  - **GPU acceleration** via OpenGL ES 2.0 for blur and rendering.
  - Works with **X11**, **Wayland**, or **DRM/KMS** (direct rendering, no
    display server).
  - **ARMv6** target so it can run on Raspberry Pi 1 / Zero.
  - Typical resource usage:
    - ~70 MiB RAM for a 1600×900 display.
    - Low CPU usage, suitable for 24/7 operation on low-power devices.

- **Configuration & integrations:**
  - Static configuration via `/etc/memocadre/config.yaml` (media sources, MQTT,
    HTTP).
  - Dynamic/runtime settings via `/etc/memocadre/settings.yaml` (slideshow
    behavior, blur, captions, debug, etc.).
  - **MQTT / Home Assistant API** to:
    - change some options,
    - go to the next photo,
    - turn the display on/off (DRM/KMS mode).
  - Minimal **HTTP API**, similar in spirit to the MQTT API.

---

## Configuration overview

MemoCadre uses **two** main configuration files:

1. **App config** (static):

   - Path: `/etc/memocadre/config.yaml`
   - Configure media sources, MQTT, HTTP.
   - Loaded via the `CONFIG_PATH` environment variable; in a typical system
     install you will point `CONFIG_PATH` to `/etc/memocadre/config.yaml`.

2. **Settings** (dynamic / runtime):
   - Path: `/etc/memocadre/settings.yaml`
   - Contains slideshow timings, blur/background options, captions, debug
     overlay, etc.
   - Can be overridden at runtime via APIs.

You can override paths when running MemoCadre by setting the following
environment variables:

```bash
# base app configuration
CONFIG_PATH=/etc/memocadre/config.yaml

# base settings
SETTINGS_PATH=/etc/memocadre/settings.yaml

# optional: Where to store/load dynamic settings overrides
DYNAMIC_SETTINGS_PATH=/var/lib/memocadre/settings-override.json
```

---

## `/etc/memocadre/config.yaml` – AppConfig (static)

This file defines:

- the **media sources** (Immich),
- optional **MQTT** integration,
- optional **HTTP** API.

### Example: Immich + MQTT + HTTP

```yaml
# /etc/memocadre/config.yaml

sources:
  - type: immich
    # Single Immich instance definition
    instance:
      url: https://immich.example.com
      api_key: "YOUR_IMMICH_API_KEY"

    # Or multiple instances if you want to mix content
    # instances:
    #   - url: https://immich-1.example.com
    #     api_key: "API_KEY_1"
    #   - url: https://immich-2.example.com
    #     api_key: "API_KEY_2"
    # If multiple instances are defined, specs below apply to all of them.
    # e.g.: each spec will be queried on each instance in round-robin fashion.
    #       Useful if you have multiple accounts on the same immich server.

    # What to show from Immich
    specs:
      # Smart search example
      - type: smart-search
        query: "family vacation"
        city: "Bordeaux"
        persons:
          - type: name
            value: "Alice"
          - type: id
            value: "person-uuid-from-immich"

      # Simple random search example (by persons only)
      - type: random-search
        persons:
          - type: name
            value: "Bob"

      # Private album
      - type: private-album
        id: "ALBUM_UUID_FROM_IMMICH"

      # Memory Lane (photos from the same date in past years)
      - type: memory-lane

# Optional MQTT configuration
mqtt:
  enabled: true
  # Broker connection details
  host: "localhost"
  # Broker port
  port: 1883
  # Optional credentials
  credentials:
    username: "memo-cadre"
    password: "change-me"

# Optional HTTP API configuration
http:
  enabled: true
  bind_address: "0.0.0.0:3000"
```

---

## `/etc/memocadre/settings.yaml` – Settings (dynamic)

This file controls how the slideshow behaves at runtime:

- transition duration,
- initial “loading” slide,
- blur/background behavior,
- captions (on/off, date format, font size),
- debug overlay,
- and more.

Any field you omit will fall back to a sensible default.

Durations are specified as strings with units, e.g. `"500ms"`, `"2s"`, or even
`"1min 30s"`.

### Minimal example

```yaml
# /etc/memocadre/settings.yaml

transition_duration: "500ms"

background:
  type: blur

caption:
  enabled: true
  font_size: 28

debug:
  show_fps: false
```

### More complete example with comments

```yaml
# /etc/memocadre/settings.yaml

# Duration of the transition between two photos
transition_duration: "700ms" # default is "500ms"

# Initial slide shown while first photo is loading
init_slide:
  type: loading-circle # or "empty" for a black screen
  # loading-circle-specific options:
  velocity: 1.5 # rotations per second (default: 1.5)

# Background behavior when photo does not fill the screen
background:
  type: blur
  min_free_space: 50 # threshold to decide where blurred strips are used (default: 50 pixels)
  # type: black                # use a solid black background instead

# Caption (city/date text)
caption:
  enabled: true # default: true
  font_size: 30 # default: 28.0
  date_format:
    # Locale and format for chrono; this example is French
    locale: "fr_FR" # default is "en_US"
    format: "%A %e %B %Y" # e.g. "samedi 25 janvier 2025", see https://docs.rs/chrono/0.4.39/chrono/format/strftime/index.html

# Downscaling filter for images larger than the display
# Possible values depend on the implementation (e.g. "nearest", "linear").
downscaled_image_filter: "linear"

# Debug options (on-screen overlay, etc.)
debug:
  show_fps: false # show frames-per-second overlay
```

---

## Usage

You can run MemoCadre by specifying the config and settings paths via
environment variables, for example to use local files in the current directory:

```bash
CONFIG_PATH=./config.yaml \
SETTINGS_PATH=./settings.yaml \
memocadre
```

At startup, MemoCadre:

1. Loads the static **AppConfig** from `CONFIG_PATH` (media sources, MQTT,
   HTTP).
2. Loads **Settings** from `SETTINGS_PATH` (slideshow behavior).
3. Optionally merges in a dynamic JSON patch from `DYNAMIC_SETTINGS_PATH`, if
   configured.
4. Initializes the graphics backend:
   - X11/Wayland via `winit` if `DISPLAY` / `WAYLAND_DISPLAY` / `WAYLAND_SOCKET`
     is set.
   - Otherwise, tries **DRM/KMS** if compiled with that feature.
5. Starts the slideshow.

---

## Installation

<!-- TODO -->

---

## Local build

### 1. Standard build (without Nix)

Requirements:

- Rust stable (via [rustup](https://rustup.rs/)).
- System dependencies:
  - `libgl1-mesa-dev`, `libgbm-dev`,
  - appropriate X11/Wayland dev libraries for your environment.

On Debian/Ubuntu for example:

```bash
sudo apt update
sudo apt install build-essential pkg-config \
  libgl1-mesa-dev libgbm-dev \
  libx11-dev libxi-dev libxrender-dev libxcursor-dev libxkbcommon-dev \
  libwayland-dev
```

Then:

```bash
git clone https://github.com/xabufr/memocadre.git
cd memocadre

# debug build
cargo build

# release build
cargo build --release
```

Depending on the backend you want:

- X/Wayland backend (default):
  ```bash
  cargo build --release
  ```
- DRM/KMS-oriented build (e.g. for embedded usage):
  ```bash
  cargo build --release --no-default-features --features drm
  ```

The binary will be in `target/release/`.

---

### 2. Build with Nix (reproducible environment)

If you use Nix, the repository provides a `shell.nix` that sets up an
environment with the correct libraries (OpenGL, X11, GBM, etc.).

1. **Enable direnv + Nix (optional but convenient):**

   ```bash
   # at the repo root, if .envrc contains "use nix"
   direnv allow
   ```

2. **Or enter the Nix shell manually:**

   ```bash
   nix-shell
   ```

3. **Build with Cargo inside that environment:**
   ```bash
   cargo build --release
   ```

---

### 3. ARMv6 cross-compilation (Raspberry Pi 1 / Zero)

The repo includes build files (Docker/Containerfile, Justfile, etc.) for
cross-compiling to ARMv6 using `cross`:

```bash
just debian-armv6
```

Results (binary and Debian package) will be in
`target/arm-unknown-linux-gnueabihf/debian/` and
`target/arm-unknown-linux-gnueabihf/release/`.

---

## Local testing

The project includes some unit tests (sadly not for the graphics parts, but
thankfully some logic is still easily testable).

```bash
cargo test
```

To visually test the app on your development machine:

1. Set up a test Immich instance (local or remote).
2. Create a minimal `.config.yaml` and `.settings.yaml`.
3. Run MemoCadre with this config:
   ```bash
   CONFIG_PATH=./config.yaml \
   SETTINGS_PATH=./settings.yaml \
   cargo run
   ```

With X11/Wayland available, a window will open and show the slideshow.

---

## License

This project is released under the **GPLv3** license. See the
[`LICENSE`](LICENSE) file for details.
