# 🧊 Hielo

A modern, native desktop application for visualizing Apache Iceberg table metadata and snapshot history. Built with Rust and Dioxus for performance and cross-platform compatibility.

*Hielo* (Spanish for "ice") is a lightweight, fast tool for exploring your Iceberg tables.

## Installation

### Pre-built Binaries

Download the latest release for your platform from the [Releases](../../releases) page:

- **Linux x86_64**: `hielo-linux-x86_64`
- **Linux ARM64**: `hielo-linux-aarch64`
- **macOS x86_64**: `hielo-macos-x86_64`
- **macOS ARM64**: `hielo-macos-aarch64`
- **Windows x86_64**: `hielo-windows-x86_64.exe`

### Building from Source

#### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Platform-specific dependencies:

**Linux (Ubuntu/Debian)**:
```bash
sudo apt-get install libgtk-3-dev libwebkit2gtk-4.0-dev libappindicator3-dev librsvg2-dev patchelf
```

**macOS**:
```bash
# Xcode command line tools (if not already installed)
xcode-select --install
```

**Windows**:
- WebView2 (usually pre-installed on Windows 10/11)
- Visual Studio Build Tools or Visual Studio with C++ tools

#### Build Commands

```bash
# Clone the repository
git clone https://github.com/yourusername/hielo.git
cd hielo

# Build in release mode
cargo build --release

# Run the application
cargo run --release
```

## Development

### Running in Development

```bash
# Run with hot reload
cargo run

# Run tests
cargo test

# Check formatting
cargo fmt --check

# Run clippy lints
cargo clippy -- -D warnings
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [Apache Iceberg](https://iceberg.apache.org/) for the amazing table format
- [Dioxus](https://dioxuslabs.com/) for the excellent Rust UI framework
- [Tailwind CSS](https://tailwindcss.com/) for the styling system

---
