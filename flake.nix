{
  description = "NQ Stats Rust Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        libraries = with pkgs; [
          wayland
          libxkbcommon
          libGL
          xorg.libX11
          xorg.libXcursor
          xorg.libXrandr
          xorg.libXi
          xorg.libxcb
          fontconfig
          openssl
          vulkan-loader
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustToolchain
            pkgs.pkg-config
          ] ++ libraries;

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath libraries;
          
          shellHook = ''
            echo "Environnement Rust + GUI prêt."
          '';
        };
      }
    );
}