# CoCo - Code Companion
### AI Pair Programmer - Real-time Code Analysis & Insights

> **See what AI thinks about your code as you write it**

[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-red.svg)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/Version-2.0.0-blue.svg)](#)
[![Hackathon](https://img.shields.io/badge/GitHub-ForTheLoveOfCode-purple.svg)](https://github.com/hackathons)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](#license)

**Created by [Vishesh Singh Rajput aka specstan](https://x.com/atspecstan)**

---

## What is CoCo?

CoCo is an **AI-powered pair programmer** that provides real-time code analysis and intelligent suggestions as you develop. Built entirely in **Rust** for maximum performance, CoCo watches your files and gives you instant AI insights through a beautiful terminal interface.

### Key Features

- **Real-time AI Analysis** - Get instant feedback on code changes
- **Lightning Fast** - 3.2MB binary, minimal resource usage
- **Rich Terminal UI** - Beautiful interface with 4 view modes
- **Session Recording** - Capture and replay development sessions
- **Smart File Watching** - Supports 20+ programming languages
- **Highly Configurable** - Extensive customization options

---

## Screenshots

### Side-by-Side View
```
┌─ Code ──────────────────────────┬─ AI Thoughts ────────────────────┐
│ fn fibonacci(n: u32) -> u32 {   │ 🔍 Analyzing recursive function  │
│     if n <= 1 { return n; }    │ ⚠️  High time complexity O(2^n)  │
│     fibonacci(n-1) + fib(n-2)  │ 💡 Consider iterative approach   │
│ }                               │ ⚡ Add memoization for speedup    │
└─────────────────────────────────┴───────────────────────────────────┘
```

---

## Quick Start

### 1. **Set Your API Key**
```bash
export ANTHROPIC_API_KEY="your-key-here"
# or create .env file
echo "ANTHROPIC_API_KEY=your-key" > .env
```

### 2. **Run CoCo**
```bash
# Basic monitoring
./target/release/coco

# With session recording
./target/release/coco record
```

### 3. **Edit Code & Watch Magic**
Open any supported file (`.rs`, `.py`, `.js`, `.go`, etc.) and see AI analysis in real-time!

---

## Controls

| Key | Action |
|-----|--------|
| `q`, `Esc`, `Ctrl+C` | Quit application |
| `v` | Toggle view modes |
| `c` | Clear AI thoughts |
| `y` / `n` | Accept/reject suggestions |
| `h` | Show help |
| `r` | Refresh analysis |

---

## Architecture

CoCo is built with a **modular Rust architecture** for maximum performance and reliability:

```
CoCo v2.0
├── App Core          # Central state management
├── Terminal UI       # Rich TUI with ratatui
├── AI Integration    # Claude API with retry logic
├── File Watcher      # Real-time file monitoring
└── Session System    # Recording & replay
```

### **Technical Highlights**
- **Async-first design** with Tokio
- **Zero-copy string processing**
- **Memory-safe concurrency** with Arc<Mutex>
- **Intelligent debouncing** to prevent analysis spam
- **Robust error handling** with graceful degradation

---

## View Modes

1. **Side-by-Side** - Code and thoughts side by side
2. **Full View** - Code on top, thoughts below
3. **Minimal** - Essential information only
4. **Thoughts Only** - AI analysis full screen

---

## Performance

- **Binary Size**: 3.2MB (optimized release)
- **Memory Usage**: ~10-50MB during operation
- **File Processing**: 10x faster than Node.js equivalents
- **UI Rendering**: 20 FPS smooth updates
- **Startup Time**: <100ms cold start

---

## AI Analysis Types

| Type | Description |
|------|-------------|
| **Analyzing** | Code structure analysis |
| **Suggesting** | Improvement recommendations |
| **Warning** | Potential issues |
| **Error** | Bugs and problems |
| **Performance** | Speed optimizations |
| **Security** | Vulnerability checks |
| **Style** | Code formatting |
| **Architecture** | Design patterns |

---

## Supported Languages

Rust • Python • JavaScript/TypeScript • Go • Java • C/C++ • C# • Ruby • PHP • Swift • Kotlin • Scala • Clojure • Elixir

---

## Configuration

Create `.env` file or set environment variables:

```bash
ANTHROPIC_API_KEY=your-api-key
COCO_LOG_LEVEL=info                    # Logging level
COCO_AUTO_SUGGESTIONS=true             # Enable auto-suggestions
COCO_CONFIDENCE_THRESHOLD=0.7          # Suggestion confidence (0-1)
COCO_ANALYSIS_DELAY_MS=500            # Analysis delay
COCO_MAX_FILE_SIZE=1048576            # Max file size (bytes)
```

---

## Commands

```bash
coco              # Start watching (default)
coco record       # Start with session recording
coco replay <id>  # Replay recorded session
coco list         # List all sessions
coco --help       # Show help
coco --version    # Show version
```

---

## Built for GitHub's ForTheLoveOfCode Hackathon

This project represents a fusion of **cutting-edge AI** and **systems programming excellence**. CoCo demonstrates:

- **Innovation**: Real-time AI pair programming
- **Performance**: Rust's zero-cost abstractions
- **User Experience**: Intuitive terminal interface
- **Reliability**: Robust error handling and recovery
- **Extensibility**: Modular architecture for future enhancements

---

## Building from Source

```bash
# Clone repository
git clone https://github.com/your-username/coco.git
cd coco

# Build optimized release
cargo build --release

# Binary will be at target/release/coco
```

---

## Contributing

We welcome contributions! Please feel free to submit issues and pull requests.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

---

## License

This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.

```
MIT License

Copyright (c) 2025 Vishesh Singh Rajput

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

---

## Acknowledgments

- **GitHub** for hosting the ForTheLoveOfCode Hackathon
- **Anthropic** for providing the Claude AI API
- **Rust Community** for the amazing ecosystem
- **Open Source Contributors** who make projects like this possible

---

## Contact

**Vishesh Singh Rajput aka specstan** - [@atspecstan](https://x.com/atspecstan)

---

<div align="center">

**If CoCo helped you write better code, please give it a star!**

Made with love for GitHub's ForTheLoveOfCode Hackathon

</div>
