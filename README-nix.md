# Nix Development Environment for Hielo

This project includes a Nix flake for reproducible development environments.

## Quick Start

### Option 1: One-time setup with `nix develop`
```bash
# Enter the development environment
nix develop

# Now you have Rust, Node.js, and all dependencies available
cargo build
cd tests && npm install && npm test
```

### Option 2: Automatic setup with direnv (recommended)
```bash
# Install direnv if you haven't already
# Then allow the .envrc file:
direnv allow

# The development environment will automatically activate when you cd into the project
```

## What's Included

### Development Tools
- **Rust**: Latest stable with rust-analyzer and rust-src
- **Node.js 20**: For UI testing with Playwright
- **npm**: Package manager for test dependencies
- **Git**: Version control
- **Cargo extensions**: cargo-watch, cargo-edit, cargo-audit

### System Dependencies
- **GUI Libraries**: GTK3, WebKit, X11 libraries for Dioxus desktop
- **Testing Tools**: xvfb for headless UI testing
- **Build Tools**: pkg-config, OpenSSL, and other native dependencies

### Environment Configuration
- **WebView Debugging**: `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` set for testing
- **Display Setup**: `DISPLAY=:99.0` configured for headless testing
- **Rust Backtrace**: Enabled for better debugging

## Available Commands

### Standard Development
```bash
cargo build          # Build Hielo
cargo run            # Run Hielo
cargo test           # Run Rust unit tests
```

### UI Testing
```bash
cd tests && npm install && npm test    # Run Playwright UI tests
./scripts/test-ui-setup.sh            # Verify test environment
```

### Nix Apps (alternative commands)
```bash
nix run .#hielo                       # Run Hielo via Nix
nix run .#test-ui                     # Run UI tests via Nix
nix run .#test-setup                  # Verify environment via Nix
```

## Platform Support

### Linux
- ✅ Full support with GTK3 and WebKitGTK
- ✅ Headless testing with xvfb
- ✅ All UI testing features work

### macOS
- ✅ Full support with macOS frameworks
- ⚠️ Limited headless testing (Playwright constraints)
- ✅ Development and building work perfectly

### Windows/WSL
- ⚠️ Use WSL2 with Linux configuration
- ✅ Should work with proper WSL setup

## Troubleshooting

### Common Issues

**"command not found" errors:**
```bash
# Make sure you're in the nix develop shell
nix develop
# Or if using direnv, check it's activated
direnv status
```

**UI tests fail to connect:**
```bash
# Verify Hielo builds successfully
cargo build

# Check the test environment
./scripts/test-ui-setup.sh

# Run with debug output
cd tests && npm run test:debug
```

**Missing GUI libraries:**
```bash
# Rebuild the environment
nix develop --rebuild

# Or clear the cache and retry
nix flake update
nix develop
```

### Environment Variables

The flake automatically sets these for you:
- `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9222"`
- `RUST_BACKTRACE=1`
- `DISPLAY=:99.0` (for headless testing)
- `PKG_CONFIG_PATH` (for native dependencies)

## Benefits of Using Nix

1. **Reproducible**: Same environment across all machines and CI
2. **Isolated**: Doesn't interfere with system packages
3. **Complete**: All dependencies declared and managed
4. **Fast**: Dependencies are cached and reused
5. **Reliable**: No "works on my machine" issues

## Integration with CI/CD

The flake can be used in GitHub Actions:

```yaml
- uses: cachix/install-nix-action@v22
- name: Setup development environment
  run: nix develop --command bash -c "cargo build && cd tests && npm install"
- name: Run tests
  run: nix develop --command bash -c "cargo test && nix run .#test-ui"
```

This ensures CI uses exactly the same environment as local development.