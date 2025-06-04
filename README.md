# Reedy - A TUI RSS Feed Reader

![Rust CI](https://github.com/dorryspears/reedy/workflows/Rust%20CI/badge.svg)

Reedy is a cross-platform RSS and Atom feed reader that runs entirely in your
terminal. It is built with Rust and the Ratatui framework and focuses on a
clean, keyboard-driven interface.

Feed data is cached locally so you can browse articles even when offline. Your
subscriptions and read status are stored in your operating system's config
directory (for example `~/.config/reedy/feeds.json` on Linux).

## Features

- Subscribe to RSS and Atom feeds
- Offline reading thanks to automatic feed caching
- Mark items as read or unread
- Save articles to a favorites list
- Open items directly in your web browser
- Manage subscriptions through a built-in feed manager
- Clean and responsive terminal UI with full keyboard navigation

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

### Quick Start

1. Launch the application: `reedy`
2. Press `m` to open the feed manager
3. Press `a` to add a feed URL
4. Once added, select a feed and press `Enter` to view its articles
5. Use the keyboard shortcuts below to navigate and manage items

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

## Configuration and Logging

Feed state is saved to your operating system's configuration directory. On
Linux, this defaults to `~/.config/reedy/feeds.json`.

Set the environment variable `REEDY_ENV=DEBUG` before launching the application
to enable debug logging. Logs are written to the data directory
(`~/.local/share/reedy/logs/reedy.log` on Linux).

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for
guidelines on how to propose changes and set up a development environment.

## License

MIT
