let
  pkgs = import <nixpkgs> { };
in
pkgs.mkShell {
  buildInputs = with pkgs; [
    # rustc
    # cargo

    libGL
    xorg.libX11
    xorg.libXi
    xorg.libXrender
    xorg.libXcursor
    libxkbcommon
    mesa
  ];
 # TODO add rust nightly for fmt
  shellHook = ''
    export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${
      with pkgs;
      pkgs.lib.makeLibraryPath [
        libGL
        libxkbcommon
        wayland
        xorg.libX11
        xorg.libXrender
        xorg.libXcursor
        xorg.libXi
        xorg.libxcb
      ]
    }"
  '';
}
