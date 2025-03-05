# Reedy

Reedy is a terminal-based RSS/Atom feed aggregator built in Rust with a text user interface (TUI).

## Features

- Subscribe to RSS and Atom feeds
- View feed content with rich text formatting in the terminal
- Manage feed subscriptions (add, delete)
- Mark items as read/unread
- Save favorite articles
- Open articles in external browser
- Cache feed content for offline reading
- Filter by favorites

## Architecture

The application consists of several key components:

- **App (app.rs)**: Core application state and business logic
- **TUI (tui.rs)**: Terminal interface management
- **Event (event.rs)**: Event handling system
- **Handler (handler.rs)**: Keyboard input processing
- **UI (ui.rs)**: UI rendering logic
- **Main (main.rs)**: Entry point and initialization

## Technology Stack

- **ratatui/crossterm**: Terminal UI framework
- **tokio**: Asynchronous runtime
- **rss/atom_syndication**: Feed parsing libraries
- **reqwest**: HTTP client for fetching feeds
- **serde**: Data serialization/deserialization
- **html2text**: HTML to terminal text conversion

## Workflow

1. App fetches and parses RSS/Atom feeds
2. Content is cached locally for offline access
3. Terminal UI displays feed items in a navigable interface
4. User interacts through keyboard shortcuts
5. Items can be marked as read/favorite or opened in browser