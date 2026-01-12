# Reedy - Product Requirements Document

## Overview

Reedy is a terminal-based RSS/Atom feed reader built with Rust. It provides a keyboard-driven TUI for subscribing to, reading, and managing RSS feeds with offline caching.

**Version:** 0.1.4
**Author:** Ryan Spears
**License:** MIT

---

## Bugs Identified

### Critical

#### ~~1. Panic on Empty List Navigation~~ FIXED
**Location:** `src/app.rs:285`, `src/app.rs:296`
**Description:** `select_previous()` and `select_next()` can panic when the list is empty.
- When `len` is 0, `(current + 1) % len` causes division by zero panic
- `len - 1` when `len = 0` causes integer underflow

**Impact:** Application crash when navigating with no feeds/items loaded.

**Fix:** Added early return guard checking `if len == 0 { return; }` in both `select_previous()` and `select_next()` functions.

#### 2. ~~Integer Underflow in `truncate_text`~~ FIXED
**Location:** `src/ui.rs:346-361`
**Description:** When `max_width < 3`, the subtraction `(max_width - 3) as usize` will underflow.

**Impact:** Potential panic or unexpected behavior when terminal is very narrow.

**Fix:** Added early return guard for `max_width < 3` that returns truncated text without ellipsis.

---

### High

#### ~~3. Cache Cleared on Every Startup~~ FIXED
**Location:** `src/app.rs:103`
**Description:** `clear_cache_dir()` is called on every startup, which defeats the purpose of caching feeds for offline reading.

**Impact:** Users always wait for feeds to reload; no true offline support.

**Fix:** Removed the `clear_cache_dir()` call from `App::new()`. The cache already has proper 1-hour TTL expiration logic in `load_feed_cache()`, so clearing on startup was unnecessary. Also removed the now-unused `clear_cache_dir()` function.

#### 4. No HTTP Request Timeouts
**Location:** `src/app.rs:371`, `src/app.rs:435`, `src/app.rs:730`, `src/app.rs:780`
**Description:** All `reqwest::get()` calls lack timeout configuration. Slow or unresponsive feeds will hang the application indefinitely.

**Impact:** Application becomes unresponsive if a feed server is slow or unreachable.

#### 5. Favorites View Not Updated on Unfavorite
**Location:** `src/app.rs:860-875`
**Description:** When unfavoriting an item while in Favorites view, the item remains visible until the user manually refreshes or toggles the view.

**Impact:** Confusing UX where unfavorited items persist in the favorites list.

---

### Medium

#### 6. Duplicate Code Between `app.rs` and `rss_manager.rs`
**Location:** `src/app.rs`, `src/rss_manager.rs`
**Description:** Both files contain nearly identical implementations of `FeedItem`, `SavedState`, `CachedFeed`, and many feed management methods. `rss_manager.rs` appears to be unused dead code.

**Impact:** Maintenance burden; confusion about which module to use; ~629 lines of dead code.

#### 7. Module Double Declaration
**Location:** `src/main.rs:18-24`
**Description:** Modules are declared in both `main.rs` and `lib.rs`, which can cause confusion and potential compilation issues.

```rust
// main.rs re-declares modules already in lib.rs
pub mod app;
pub mod event;
pub mod handler;
pub mod rss_manager;
pub mod tui;
pub mod ui;
```

**Impact:** Code organization issues; potential for module resolution conflicts.

#### 8. Hardcoded Page Sizes
**Location:** `src/app.rs:311-313`, `src/app.rs:597-600`
**Description:** Page size for scrolling uses hardcoded values (5 items for feed list, 10 for manager) instead of calculating based on actual terminal height.

**Impact:** Inconsistent pagination; items may overflow or underflow the visible area.

#### 9. Blocking Async Patterns
**Location:** `src/app.rs:113-130`, `src/app.rs:885-902`
**Description:** Using `block_in_place` with `block_on` inside async context is inefficient and can cause deadlocks in certain scenarios.

**Impact:** Performance degradation; potential deadlocks.

---

### Low

#### 10. Debug Statement Left in Code
**Location:** `src/app.rs:413`
**Description:** `debug!("test");` appears to be leftover debug code that serves no purpose.

#### 11. Silent Error Handling
**Location:** `src/app.rs:234-252`, various `unwrap_or_default()` calls
**Description:** Some errors are silently ignored, making debugging difficult. Old format parsing failures don't report to user.

#### 12. Dead Code Warning Suppression
**Location:** `src/event.rs:23`
**Description:** `#[allow(dead_code)]` attribute on `EventHandler` suggests unused fields that should be cleaned up.

---

## Feature Recommendations

### High Priority

#### 1. Search/Filter Functionality
**Description:** Allow users to filter feed items by keyword in title or description.
**Value:** Essential for users with many subscriptions to find specific content.

#### 2. Feed Export/Import via Clipboard
**Description:** Simple feed list export and import using the clipboard.
- **Export (`e` key in Feed Manager):** Copies all feed URLs to clipboard, one URL per line
- **Import (`i` key in Feed Manager):** Opens a text input where users can paste feed URLs (one per line), validates each URL, and adds valid feeds

**Example clipboard format:**
```
https://example.com/feed.xml
https://blog.example.org/rss
https://news.site.com/atom.xml
```

**Value:** Quick and easy way to backup, share, or migrate feed subscriptions without complex file formats.

#### 3. OPML Import/Export (Optional)
**Description:** Support importing and exporting feed lists in OPML format (industry standard).
**Value:** Compatibility with other RSS readers for migration.

#### 4. Feed Title Display
**Description:** Extract and display the actual feed title instead of showing the URL.
**Value:** Much better UX; users can identify feeds at a glance.

#### 5. Unread Count per Feed
**Description:** Show the number of unread items for each feed in the feed manager.
**Value:** Helps users quickly identify which feeds have new content.

#### 6. Configurable HTTP Timeout
**Description:** Add configuration for request timeouts to prevent hangs.
**Value:** Prevents application from becoming unresponsive.

---

### Medium Priority

#### 7. Feed Categories/Tags
**Description:** Allow users to organize feeds into custom categories or tag them.
**Value:** Better organization for users with many subscriptions.

#### 8. Auto-Refresh Interval
**Description:** Automatically refresh feeds at a configurable interval.
**Value:** Users see new content without manual refresh.

#### 9. Article Preview Pane
**Description:** Show full article content in a dedicated pane within the TUI.
**Value:** Read articles without leaving the terminal.

#### 10. Configurable Cache Duration
**Description:** Let users configure how long feed cache remains valid (currently hardcoded to 1 hour).
**Value:** Flexibility for different use cases (low bandwidth vs. always fresh).

#### 11. Theme Customization
**Description:** Support light/dark themes and customizable color schemes.
**Value:** Accessibility and user preference support.

---

### Low Priority

#### 12. Keyboard Shortcuts Customization
**Description:** Allow users to configure their own keybindings via config file.
**Value:** Power users can optimize their workflow.

#### 13. Vi-Style Commands
**Description:** Support command mode with `:q`, `:w`, `:wq` style commands.
**Value:** Familiar to vim users; consistent terminal experience.

#### 14. Mouse Support
**Description:** Enable clicking to select items and scroll.
**Value:** Accessibility for users who prefer mouse navigation.

#### 15. Export Articles
**Description:** Save articles to file (markdown, plain text) or copy to clipboard.
**Value:** Allows saving interesting articles for later reference.

#### 16. Feed Health Indicators
**Description:** Visual indicators showing feed status (healthy, slow, broken, last updated).
**Value:** Helps users identify and remove problematic feeds.

#### 17. Notification Support
**Description:** Desktop notifications for new articles in subscribed feeds.
**Value:** Stay informed without keeping the app open.

#### 18. Mark Items Read on Scroll
**Description:** Optionally auto-mark items as read when scrolling past them.
**Value:** Reduces manual marking; common RSS reader feature.

#### 19. Vim-Style `G` for Bottom
**Description:** Add `G` (shift+g) to scroll to the bottom of the list (complement to `g` for top).
**Value:** Standard vim navigation pattern.

---

## Technical Debt

1. **Remove `rss_manager.rs`** - Dead code that duplicates `app.rs` functionality
2. **Consolidate module declarations** - Remove duplicate declarations from `main.rs`
3. **Add comprehensive test coverage** - Current tests only cover basic state transitions
4. **Add integration tests** - Test actual feed fetching with mock server
5. **Document public API** - Add rustdoc comments to all public functions
6. **Error type consolidation** - Create proper error enum instead of `Box<dyn Error>`

---

## Configuration File (Future)

Consider adding a `~/.config/reedy/config.toml` for:
- Cache duration
- HTTP timeout
- Default keybindings
- Theme/colors
- Auto-refresh interval
- Notification preferences
