{
  description = "MicroKit development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
          targets = [ "wasm32-unknown-unknown" ];
        };

        # Linux deps for wry/webkit desktop webview
        linuxBuildInputs = with pkgs; lib.optionals stdenv.isLinux [
          webkitgtk_4_1
          gtk3
          glib
          cairo
          pango
          atk
          gdk-pixbuf
          libsoup_3
          openssl
          at-spi2-atk
          at-spi2-core
          xdotool
        ];

        # macOS deps for wry/webkit desktop webview
        darwinBuildInputs = with pkgs; lib.optionals stdenv.isDarwin [
          darwin.apple_sdk.frameworks.WebKit
          darwin.apple_sdk.frameworks.CoreServices
          darwin.apple_sdk.frameworks.Carbon
          darwin.apple_sdk.frameworks.AppKit
          darwin.apple_sdk.frameworks.Security
        ];

      in {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            pkg-config
            rustc
            cargo
            rustfmt
            clippy
            rust-analyzer
          ];

          buildInputs = [
            rustToolchain
            pkgs.dioxus-cli
            pkgs.openssl
          ] ++ linuxBuildInputs ++ darwinBuildInputs;

          shellHook = ''
            export RUST_SRC_PATH="${rustToolchain}/lib/rustlib/src/rust/library"
          '' + pkgs.lib.optionalString pkgs.stdenv.isLinux ''
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath linuxBuildInputs}:$LD_LIBRARY_PATH"
          '';
        };
      }
    );
}
