{
  description = "Hielo - Apache Iceberg Table Viewer";

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

        # Rust toolchain specification
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        # System-specific dependencies
        systemDeps = with pkgs; [
          # Core build tools
          pkg-config

          # GUI dependencies for Dioxus desktop
          gtk3
          glib
          glib.dev

          # Audio support
          alsa-lib

          # Additional Linux GUI dependencies
          libxcb
          xorg.libX11
          xorg.libXcursor
          xorg.libXrandr
          xorg.libXi

          # For UI testing with xvfb
          xvfb-run

          # Required for some native dependencies
          openssl
          openssl.dev
        ] ++ lib.optionals stdenv.isLinux [
          # Linux-specific WebKit dependencies
          webkitgtk_4_1
          libayatana-appindicator

          # Additional Linux dependencies
          libsoup_3
          librsvg

          # For headless testing
          xorg.xorgserver
        ] ++ lib.optionals stdenv.isDarwin [
          # macOS-specific dependencies
          darwin.apple_sdk.frameworks.WebKit
          darwin.apple_sdk.frameworks.AppKit
          darwin.apple_sdk.frameworks.Security
          darwin.apple_sdk.frameworks.CoreServices
        ];

        # Development tools
        devTools = with pkgs; [
          # Rust development
          rustToolchain

          # Node.js for UI testing (includes npm)
          nodejs_20

          # Git and version control
          git

          # Additional development tools
          cargo-watch
          cargo-edit
          cargo-audit

          # For debugging and profiling
          gdb
          valgrind

          # Text processing and utilities
          jq
          ripgrep
          fd

          # Process management for tests
          psmisc
        ];

      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = systemDeps ++ devTools;

          # Environment variables
          shellHook = ''
            echo "🧊 Welcome to Hielo development environment!"
            echo ""
            echo "Available tools:"
            echo "  Rust: $(rustc --version)"
            echo "  Node.js: $(node --version)"
            echo "  npm: $(npm --version)"
            echo ""
            echo "Quick start:"
            echo "  cargo build          # Build Hielo"
            echo "  cargo run            # Run Hielo"
            echo "  cd tests && npm install && npm test  # Run UI tests"
            echo "  ./scripts/test-ui-setup.sh          # Verify test setup"
            echo ""

            # Set up environment for WebView debugging
            export WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9222"
            export RUST_BACKTRACE=1

            # Ensure npm is in PATH and working
            if command -v npm >/dev/null 2>&1; then
              echo "✅ npm is available"
            else
              echo "❌ npm not found in PATH"
            fi

            # Set up display for headless testing
            export DISPLAY=:99.0

            echo "Environment configured for Hielo development! 🚀"
          '';

          # Platform-specific environment variables
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig:${pkgs.glib.dev}/lib/pkgconfig";

          # For WebKit on Linux
          WEBKIT_DISABLE_COMPOSITING_MODE = "1";

          # Cargo environment
          CARGO_TARGET_DIR = "./target";
        };

        # Package definition for building Hielo
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "hielo";
          version = "0.5.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            rustToolchain
          ];

          buildInputs = systemDeps;

          # Skip tests in package build (they require GUI)
          doCheck = false;

          meta = with pkgs.lib; {
            description = "Apache Iceberg Table Viewer";
            homepage = "https://github.com/your-org/hielo";
            license = licenses.mit;
            maintainers = [];
            platforms = platforms.linux ++ platforms.darwin;
          };
        };

        # Development apps
        apps = {
          # Run Hielo
          hielo = flake-utils.lib.mkApp {
            drv = self.packages.${system}.default;
          };

          # Run UI tests
          test-ui = flake-utils.lib.mkApp {
            drv = pkgs.writeShellScriptBin "test-ui" ''
              set -e
              echo "🧪 Running Hielo UI tests..."

              # Build first
              cargo build

              # Install test dependencies if needed
              if [ ! -d "tests/node_modules" ]; then
                echo "Installing test dependencies..."
                cd tests
                npm install
                npm run install-browsers
                cd ..
              fi

              # Run tests with proper environment
              cd tests

              if [ "$(uname)" = "Linux" ]; then
                echo "Running tests with xvfb on Linux..."
                xvfb-run -a -s "-screen 0 1280x720x24" npm test
              else
                echo "Running tests directly..."
                npm test
              fi
            '';
          };

          # Verify development environment
          test-setup = flake-utils.lib.mkApp {
            drv = pkgs.writeShellScriptBin "test-setup" ''
              ./scripts/test-ui-setup.sh
            '';
          };
        };

        # Formatter for nix files
        formatter = pkgs.nixpkgs-fmt;
      });
}