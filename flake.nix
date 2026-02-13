{
  description = "QuantumLauncher - A minimalistic Minecraft launcher";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        # Rust toolchain - using stable with MSRV 1.82.0
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        # Native build inputs (tools needed at build time)
        nativeBuildInputs = with pkgs; [
          pkg-config
          rustToolchain
          cargo
          rustfmt
          clippy
        ];

        # Build inputs (libraries needed for linking)
        buildInputs = with pkgs; [
          # OpenSSL (some deps may need it even with rustls)
          openssl

          # Font rendering
          fontconfig
          freetype

          # Wayland support
          wayland
          wayland-protocols
          libxkbcommon

          # X11 support
          xorg.libX11
          xorg.libXcursor
          xorg.libXrandr
          xorg.libXi
          xorg.libxcb

          # Graphics (wgpu backend)
          vulkan-loader
          vulkan-headers
          libGL

          # GTK/GLib (for rfd file dialogs with xdg-portal)
          gtk3
          glib
          gdk-pixbuf
          pango
          cairo
          atk

          # DBus (for dark-light theme detection and portals)
          dbus

          # Portal support
          xdg-desktop-portal
          xdg-desktop-portal-gtk

            # Keyring/secrets support
            libsecret
            gnome-keyring
            seahorse
        ];

        # Runtime library path for graphics drivers
        libPath = pkgs.lib.makeLibraryPath [
          pkgs.vulkan-loader
          pkgs.libGL
          pkgs.wayland
          pkgs.libxkbcommon
          pkgs.xorg.libX11
          pkgs.xorg.libXcursor
          pkgs.xorg.libXrandr
          pkgs.xorg.libXi
        ];

      in {
        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs buildInputs;

          # Environment variables
          LD_LIBRARY_PATH = libPath;
          RUST_BACKTRACE = "1";

          # For wgpu to find Vulkan
          VK_ICD_FILENAMES = "${pkgs.vulkan-loader}/share/vulkan/icd.d/intel_icd.x86_64.json:${pkgs.vulkan-loader}/share/vulkan/icd.d/radeon_icd.x86_64.json";

          # pkg-config path
          PKG_CONFIG_PATH = pkgs.lib.makeSearchPath "lib/pkgconfig" buildInputs;

          shellHook = ''  
            # Start gnome-keyring daemon
            eval $(gnome-keyring-daemon --start --components=secrets 2>/dev/null)
            export SSH_AUTH_SOCK
            # Create default keyring if it doesn't exist (empty password for dev)
            echo "" | gnome-keyring-daemon --unlock 2>/dev/null || true


            echo "QuantumLauncher dev shell"
            echo "Rust: $(rustc --version)"
            echo ""
            echo "Commands:"
            echo "  cargo run --release    Build and run"
            echo "  cargo build            Debug build"
            echo "  cargo test             Run tests"
          '';
        };
      }
    );
}
