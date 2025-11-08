# 🦀 Rusty Requester

I develop this (vibe-coded, hehe) because I use api client a lot while I used to using Postman, it's so takes a lot of resource especially the online sync I don't even need it and then I use a lot curl on my terminal, but it's hard to manage the requests, so I build this, a **native, lightweight API client** built with Rust - the ultimate alternative to resource-heavy Electron apps like Postman.


![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![macOS](https://img.shields.io/badge/mac%20os-000000?style=for-the-badge&logo=macos&logoColor=F0F0F0)

## ✨ Features

- 🚀 **Truly Native** - Built with Rust and egui, not Electron wrapper
- 💾 **Local Storage Only** - No cloud sync, no telemetry, complete privacy
- 🎨 **Beautiful UI** - Dracula-inspired dark theme with color-coded HTTP methods
- 📁 **Folder Organization** - Organize your API requests in folders
- 🔧 **Full HTTP Support** - GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS
- 📋 **JSON Formatting** - Automatically formats JSON responses
- ⚡ **Lightning Fast** - Native performance, minimal resource usage
- 🍎 **Apple Silicon Optimized** - Built specifically for M1/M2/M3 Macs

## 🎯 Why Rusty Requester?

**Postman is resource-intensive**. It's essentially a Chromium wrapper that can eat up hundreds of MB of RAM and CPU just to make HTTP requests. Rusty Requester is different:

- **~10MB RAM** vs Postman's ~500MB+
- **Native binary** vs Electron bundle
- **Instant startup** vs slow Electron initialization
- **No tracking** vs analytics and telemetry
- **Local-first** vs cloud-dependent

## 🚀 Installation

### Prerequisites

- Rust 1.70+ (Install from [rustup.rs](https://rustup.rs))
- macOS (Apple Silicon or Intel)

### Build from Source
```bash
# Clone the repository
git clone https://github.com/yourusername/rusty-requester
cd rusty-requester

# Build for release (optimized)
cargo build --release

# Run the application
cargo run --release
```

### Build for Apple Silicon (M1/M2/M3)
```bash
cargo build --release --target aarch64-apple-darwin
```

The compiled binary will be in `target/release/rusty-requester` or `target/aarch64-apple-darwin/release/rusty-requester`

## 📖 Usage

### Creating Your First Request

1. **Create a Folder**: Click the green "➕ New Folder" button in the sidebar
2. **Add a Request**: Click "➕ New Request" inside your folder
3. **Configure Request**:
   - Set the HTTP method (GET, POST, etc.)
   - Enter the URL
   - Add headers if needed
   - Add request body for POST/PUT requests
4. **Send**: Click the purple "Send" button
5. **View Response**: See formatted JSON responses in the response panel

### Keyboard Shortcuts

- **Enter** in URL field: Send request
- **Right-click folder**: Rename folder
- **Enter** while renaming: Save new name

### Managing Requests

- **Rename Folder**: Right-click on folder → "✏ Rename"
- **Delete Request**: Click the red "🗑 Delete" button
- **Edit Request**: All changes auto-save

## 🎨 Features in Detail

### HTTP Methods

Color-coded for quick identification:

- 🟢 **GET** - Green
- 🟠 **POST** - Orange  
- 🔵 **PUT** - Cyan
- 🔴 **DELETE** - Pink
- 🟣 **PATCH** - Purple
- ⚪ **HEAD/OPTIONS** - Gray

### Response Viewer

- **Status Codes**: Color-coded (2xx green, 3xx yellow, 4xx/5xx red)
- **Response Time**: Millisecond accuracy
- **JSON Formatting**: Automatic pretty-printing
- **Copy Button**: One-click copy entire response
- **Large Display**: Response takes 60% of screen space

### Data Storage

All data is stored locally at:
- **macOS**: `~/Library/Application Support/rusty-requester/data.json`

Your data never leaves your computer!

## 🏗️ Architecture

Built with modern Rust technologies:

- **egui**: Immediate mode GUI framework
- **reqwest**: HTTP client library
- **tokio**: Async runtime
- **serde**: JSON serialization
- **poll-promise**: Async task management

## 🔧 Configuration

The app stores all collections and requests in a single JSON file. You can:

- Back up your `data.json` file
- Share collections with your team
- Version control your API collections

## 🤝 Contributing

Contributions are welcome! Feel free to:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📝 License

This project is licensed under the MIT License - see the LICENSE file for details.

## 🙏 Acknowledgments

- Built with [egui](https://github.com/emilk/egui) by Emil Ernerfeldt
- Inspired by the need for a lightweight, native API client
- Color scheme inspired by [Dracula Theme](https://draculatheme.com)

## 🐛 Known Issues

- Async response display could be improved with channels
- No request history yet (coming soon!)
- Environment variables not yet supported

## 🗺️ Roadmap

- [ ] Request history
- [ ] Environment variables
- [ ] Import/Export collections (Postman format)
- [ ] Query parameter builder
- [ ] Authentication presets (Bearer, Basic, etc.)
- [ ] Request chaining
- [ ] GraphQL support
- [ ] WebSocket testing
- [ ] Windows and Linux support

## 📬 Contact

Created by [@chud-lori](https://github.com/chud-lori)

---

**Made with 🦀 and ❤️ in Rust**
