build-armv6:
  NIX_STORE=/nix/store cross build --target arm-unknown-linux-gnueabihf --release --no-default-features -F drm

debian-armv6: build-armv6
  cargo deb --target=arm-unknown-linux-gnueabihf --no-build --no-strip

build: debian-armv6
