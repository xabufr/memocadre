{
  description = "Dev shell that syncs automatically with rust-toolchain.toml via Fenix";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix.url = "github:nix-community/fenix";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    fenix,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {inherit system;};

        rustStable = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-SJwZ8g0zF2WrKDVmHrVG3pD2RGoQeo24MEXnNx5FyuI=";
        };
        rustNightly = fenix.packages.${system}.complete;
        rust = fenix.packages.${system}.combine [rustNightly.rustfmt rustStable];

        nativeLibs = [
          pkgs.libGL
          pkgs.xorg.libX11
          pkgs.xorg.libXi
          pkgs.xorg.libXrender
          pkgs.xorg.libXcursor
          pkgs.libxkbcommon
          pkgs.mesa
          pkgs.libgbm
        ];
      in {
        devShells.default = pkgs.mkShell {
          buildInputs =
            [
              rust
              pkgs.cargo-deb
              pkgs.cargo-cross
              pkgs.just
            ]
            ++ nativeLibs;

          shellHook = ''
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath nativeLibs}:$LD_LIBRARY_PATH"
          '';
        };
      }
    );
}
