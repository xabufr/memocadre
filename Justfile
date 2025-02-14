build-armv6:
  NIX_STORE=/nix/store cross build --target arm-unknown-linux-gnueabihf --release --no-default-features -F drm

debian-armv6: build-armv6
  cargo deb --target=arm-unknown-linux-gnueabihf --no-build --no-strip

build: debian-armv6

deploy-armv6 TARGET: debian-armv6
  scp ./target/arm-unknown-linux-gnueabihf/debian/photo-kiosk_0.1.0-1_armhf.deb {{ TARGET }}:~
  ssh {{ TARGET }} sudo dpkg -i '~/photo-kiosk_0.1.0-1_armhf.deb'
