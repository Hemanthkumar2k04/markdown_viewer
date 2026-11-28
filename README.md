# MDpane

A fast, GPU-accelerated Markdown editor and live preview pane built with Rust and [`gpui-kit`](https://crates.io/crates/gpui-kit).

## Features

- **Resizable Split View**: Side-by-side Markdown editor and live rendered preview (`h_resizable`), with debounced updates (150 ms) as you type.
- **Save & Dirty-State Tracking**: Save changes with `Ctrl+S` (Linux/Windows) or `Cmd+S` (macOS), with a `Saved` / `Not Saved` status badge and active file path in the footer.
- **Persistent Recent Files**: Remembers up to 10 recently opened files on the home screen, pruning missing paths automatically and allowing one-click removal.
- **Configurable Editor Typography**: Built-in **Config** modal (under the titlebar menu) with a searchable system font picker and font size controls (`10 px` – `32 px`), persisted to disk.
- **Custom Dark Theme**: Styled with a bundled `Charcoal Dark` theme (`src/themes/charcoal.json`) and transparent window support.
- **Cross-Platform Packaging**: Ready-to-build desktop bundles for Linux (`.deb`, `.AppImage`), macOS (`.app`, `.dmg`), and Windows (`.exe` NSIS installer) via `cargo-packager` and GitHub Actions.

## Installation

### Pre-built Releases

Download installers or portable packages from the [GitHub Releases](https://github.com/Hemanthkumar2k04/MDpane/releases) page:

- **Linux**: `.deb` (Debian/Ubuntu) or `.AppImage`
- **macOS**: `.dmg` or `.app`
- **Windows**: `.exe` (NSIS installer)

On Debian/Ubuntu:

```bash
sudo apt install ./mdpane_0.1.0_amd64.deb
```

### Build from Source

#### Prerequisites

- [Rust toolchain](https://rustup.rs/) (Edition 2024)
- **Linux system libraries** (Ubuntu/Debian):

```bash
sudo apt-get update
sudo apt-get install -y \
  pkg-config \
  libfontconfig1-dev \
  libfreetype-dev \
  libvulkan-dev \
  libwayland-dev \
  libx11-dev \
  libxcb1-dev \
  libxcb-render0-dev \
  libxcb-shape0-dev \
  libxcb-xfixes0-dev \
  libxkbcommon-dev \
  libxkbcommon-x11-dev \
  librsvg2-dev
```

#### Run

```bash
git clone https://github.com/Hemanthkumar2k04/MDpane.git
cd MDpane
cargo run --release
```

## Usage & Shortcuts

| Action | Shortcut / UI |
| --- | --- |
| Open a Markdown file | **Open a file** button on Home or **Menu → File** |
| Save current file | `Ctrl+S` (Linux/Windows) · `Cmd+S` (macOS) |
| Return to Home / Recent files | Home button in the titlebar |
| Configure editor font & size | **Menu → Config** |

### Configuration & Data Files

MDpane stores user data under your OS configuration directory (`dirs::config_dir()/mdpane/`):

- `config.json` — Editor font family (default: `JetBrains Mono`) and font size (default: `14 px`).
- `recent_files.json` — Up to 10 most recently opened Markdown file paths.

Typical locations:

- **Linux**: `~/.config/mdpane/`
- **macOS**: `~/Library/Application Support/mdpane/`
- **Windows**: `%APPDATA%\mdpane\`

## Packaging Locally

Packaging uses [`cargo-packager`](https://github.com/crabnebula-dev/cargo-packager):

```bash
cargo install cargo-packager --locked

# Linux (.deb / .AppImage)
cargo packager --release --formats deb,appimage

# macOS (.app / .dmg)
cargo packager --release --formats app,dmg

# Windows (NSIS installer)
cargo packager --release --formats nsis
```

Packages are written to `./dist`.

## Project Structure

```text
MDpane/
├── .github/
│   ├── pull_request_template.md
│   └── workflows/release.yml   # Cross-platform release builds (v* tags)
├── assets/                     # App icons (PNG & ICO)
├── bundle/linux/mdpane.desktop # Linux desktop entry
├── src/
│   ├── main.rs                 # App startup, keybindings, theme & window setup
│   ├── config.rs               # Persistent AppConfig & EditorConfig
│   ├── recent_files.rs         # Bounded MRU recent files persistence
│   ├── icons/                  # Bundled SVG icons
│   ├── themes/charcoal.json    # Charcoal Dark GPUI theme
│   └── viewer/
│       ├── mod.rs              # MarkdownViewer state & debounced editor updates
│       ├── io.rs               # Async file I/O, dialogs, config & history saving
│       └── view.rs             # Titlebar, split view, home screen, footer & config modal
├── build.rs                    # Windows icon embedding via winresource
└── Cargo.toml
```

## Contributing

Contributions are welcome!

1. Fork the repository and create a feature branch from `main`.
2. Ensure checks pass before opening a pull request:
   ```bash
   cargo fmt --check
   cargo clippy --all-targets
   cargo check
   ```
3. Open a Pull Request using the [PR template](.github/pull_request_template.md).

## License

This project is licensed under the [MIT License](LICENSE).
