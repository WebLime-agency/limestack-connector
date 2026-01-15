# LimeStack Connector

A lightweight desktop application that enables direct communication between LimeStack and local hardware devices (printers, scales, barcode scanners).

## Download

Download the latest release for your platform from the [Releases](https://github.com/limestack/limestack-connector/releases) page.

## Development

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) (v20+)
- Platform-specific dependencies:
  - **macOS**: Xcode Command Line Tools
  - **Windows**: Visual Studio Build Tools, WebView2
  - **Linux**: `webkit2gtk`, `libappindicator`

### Setup

```bash
# Install dependencies
npm install

# Run in development mode
npm run dev
```

### Building

```bash
# Build for current platform
npm run build
```

Installers will be in `src-tauri/target/release/bundle/`

## Architecture

```
Browser (LimeStack) <--WebSocket--> Connector <--> Printer/Scale
                     localhost:9632
```

The connector runs a WebSocket server on `localhost:9632`. The LimeStack web app connects to this server to:
- Enumerate available printers
- Send print jobs (PDF labels)
- Read scale weights (future)

### Security

- Only accepts connections from allowed origins (app.limestack.io, localhost dev)
- Runs entirely on localhost - no external network access
- No data is stored or transmitted externally

### Protocol

See `src-tauri/src/protocol.rs` for message types.

**Client → Connector:**
- `hello` - Authenticate with origin
- `get_printers` - List available printers
- `print` - Send a print job

**Connector → Client:**
- `welcome` - Connection accepted, includes printer list
- `printers` - Printer list response
- `print_result` - Print job result
- `error` - Error message

## Icons

Place icons in `src-tauri/icons/`:
- `icon.png` (512x512) - Main icon
- `icon.icns` - macOS
- `icon.ico` - Windows
- `32x32.png`, `128x128.png`, `128x128@2x.png`

Generate with: `npm run tauri icon` (after placing a 512x512 icon.png)

## License

Proprietary - LimeStack
