build-armv6:
  NIX_STORE=/nix/store cross build --target arm-unknown-linux-gnueabihf --release

debian-armv6: build-armv6
  cargo deb --target=arm-unknown-linux-gnueabihf --no-build --no-strip
