# Reedy - A TUI RSS Feed Reader

![Rust CI](https://github.com/dorryspears/reedy/workflows/Rust%20CI/badge.svg)

Reedy is a terminal-based RSS feed reader with a clean interface built with Rust and Ratatui.

## Features

- Read and manage RSS and Atom feeds
- Mark articles as read/unread
- Save favorite articles
- Open articles in your browser
- Clean and responsive terminal UI
- Keyboard-based navigation

## Installation

### Option 1: Install from cargo

```bash
# Install directly from crates.io
cargo install reedy
```

### Option 2: Build from source

```bash
# Clone the repository
git clone https://github.com/USERNAME/reedy.git
cd reedy

# Build the application
cargo build --release

# Run the application
cargo run --release
```

## Usage

### Navigation

- `↑/k`, `↓/j`: Navigate between items
- `PgUp`, `PgDown`: Scroll page up/down
- `g`: Scroll to top of feed
- `Enter`: Read selected feed

### Actions

- `o`: Open selected item in browser
- `r`: Toggle read status of selected item
- `R`: Mark all items as read
- `f`: Toggle favorite status of selected item
- `F`: Toggle favorites view
- `m`: Open feed manager
- `c`: Refresh feed cache
- `?`: Toggle help menu
- `q/Esc`: Quit application

## Development

### Running Tests

```bash
cargo test
```

### Building

```bash
cargo build
```

## License

MIT
