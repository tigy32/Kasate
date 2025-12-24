# Kasate

A cross-platform AI chat application built with Tauri 2.0 (Rust backend) and HTML/CSS/JS frontend.

## Features

- **Chat Interface** - ChatGPT-style conversation UI with message history
- **Voice Chat** - HAL9000-inspired voice interface with blue/purple aesthetic
- **Chat History** - Browse and search past conversations
- **Cross-Platform** - Runs on desktop (Windows, macOS, Linux), mobile (iOS, Android), and web

## Tech Stack

- **Backend**: Rust with Tauri 2.0
- **Frontend**: Vanilla HTML, CSS, JavaScript
- **Styling**: Custom CSS with dark theme and gradient accents

## Project Structure

```
Kasate/
├── src-tauri/           # Rust backend
│   ├── src/
│   │   ├── main.rs      # Entry point
│   │   ├── lib.rs       # Tauri commands
│   │   └── chat.rs      # Chat logic
│   ├── Cargo.toml
│   └── tauri.conf.json
├── ui/                  # Frontend
│   ├── index.html
│   ├── css/
│   │   ├── main.css     # Base styles
│   │   ├── chat.css     # Chat screen
│   │   ├── voice.css    # Voice screen (HAL9000 style)
│   │   └── history.css  # History screen
│   └── js/
│       └── app.js       # Application logic
└── README.md
```

## Development

### Prerequisites

- Rust (1.70+)
- Node.js (18+)
- Platform-specific dependencies for Tauri

### Run Development Server

```bash
cd src-tauri
cargo tauri dev
```

### Build for Production

```bash
cd src-tauri
cargo tauri build
```

### Preview UI Without Tauri

Open `ui/index.html` directly in a browser to preview the UI with mock data.

## Screens

### Chat
Standard chat interface with:
- Message input with auto-resize
- Conversation sidebar
- Suggestion buttons for quick prompts

### Voice
HAL9000-inspired interface featuring:
- Animated "eye" with pulsing glow effects
- Blue and purple color scheme
- Audio visualizer bars
- Transcript display

### History
Browse past conversations with:
- Search functionality
- Grouped by date
- Message preview
