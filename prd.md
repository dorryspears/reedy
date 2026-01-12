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

#### ~~4. No HTTP Request Timeouts~~ FIXED
**Location:** `src/app.rs:371`, `src/app.rs:435`, `src/app.rs:730`, `src/app.rs:780`
**Description:** All `reqwest::get()` calls lack timeout configuration. Slow or unresponsive feeds will hang the application indefinitely.

**Impact:** Application becomes unresponsive if a feed server is slow or unreachable.

**Fix:** Added a `create_http_client()` helper function that creates a `reqwest::Client` with a 30-second timeout. All HTTP requests now use this client instead of the bare `reqwest::get()` function.

#### ~~5. Favorites View Not Updated on Unfavorite~~ FIXED
**Location:** `src/app.rs:866-893`
**Description:** When unfavoriting an item while in Favorites view, the item remains visible until the user manually refreshes or toggles the view.

**Impact:** Confusing UX where unfavorited items persist in the favorites list.

**Fix:** Modified `toggle_favorite()` to check if in Favorites view after unfavoriting. If so, the item is immediately removed from `current_feed_content` and the `selected_index` is adjusted appropriately (moved to previous item if at end, or set to None if list becomes empty).

---

### Medium

#### ~~6. Duplicate Code Between `app.rs` and `rss_manager.rs`~~ FIXED
**Location:** `src/app.rs`, `src/rss_manager.rs`
**Description:** Both files contain nearly identical implementations of `FeedItem`, `SavedState`, `CachedFeed`, and many feed management methods. `rss_manager.rs` appears to be unused dead code.

**Impact:** Maintenance burden; confusion about which module to use; ~629 lines of dead code.

**Fix:** Deleted `src/rss_manager.rs` (629 lines of dead code) and removed the module declarations from both `src/main.rs` and `src/lib.rs`.

#### ~~7. Module Double Declaration~~ FIXED
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

**Fix:** Removed the duplicate module declarations from `main.rs`. The modules are already properly declared in `lib.rs` and exported via the `reedy` crate, which `main.rs` imports. This eliminates the redundancy and potential for confusion.

#### ~~8. Hardcoded Page Sizes~~ FIXED
**Location:** `src/app.rs:311-313`, `src/app.rs:597-600`
**Description:** Page size for scrolling uses hardcoded values (5 items for feed list, 10 for manager) instead of calculating based on actual terminal height.

**Impact:** Inconsistent pagination; items may overflow or underflow the visible area.

**Fix:** Added `items_per_page()` helper method that dynamically calculates visible items based on terminal height and page mode. Updated `ensure_selection_visible()`, `page_up()`, and `page_down()` to use this helper instead of hardcoded values.

#### ~~9. Blocking Async Patterns~~ FIXED
**Location:** `src/app.rs:113-130`, `src/app.rs:885-902`
**Description:** Using `block_in_place` with `block_on` inside async context is inefficient and can cause deadlocks in certain scenarios.

**Impact:** Performance degradation; potential deadlocks.

**Fix:** Converted `App::new()` and `toggle_favorites_page()` from sync to async functions. Removed all `tokio::task::block_in_place` and `block_on` calls from `app.rs` and `handler.rs`. Async operations now flow naturally through the async runtime without blocking.

---

### Low

#### ~~10. Debug Statement Left in Code~~ FIXED
**Location:** `src/app.rs:413`
**Description:** `debug!("test");` appears to be leftover debug code that serves no purpose.

**Fix:** Removed the useless `debug!("test");` statement from `select_feed()`. The function already has a proper debug statement logging meaningful context.

#### ~~11. Silent Error Handling~~ FIXED
**Location:** `src/app.rs:234-252`, various `unwrap_or_default()` calls
**Description:** Some errors are silently ignored, making debugging difficult. Old format parsing failures don't report to user.

**Fix:** Added error reporting for critical silent failures:
1. `load_feeds()`: Now displays an error message to the user when the feeds file exists but cannot be parsed by any format (new, middle, or old). This alerts users to potential file corruption.
2. `load_config()`: Now logs warnings when config file exists but cannot be read or parsed. Uses default values but informs users via log output.

#### ~~12. Dead Code Warning Suppression~~ FIXED
**Location:** `src/event.rs:23`
**Description:** `#[allow(dead_code)]` attribute on `EventHandler` suggests unused fields that should be cleaned up.

**Fix:** Renamed the unused-but-necessary fields `sender` and `handler` to `_sender` and `_handler` respectively. This follows Rust conventions for intentionally unused fields that must remain in scope (the sender prevents channel closure; the handler keeps the tokio task alive). Removed the `#[allow(dead_code)]` attribute and updated comments to explain why these fields exist.

---

## Feature Recommendations

### High Priority

#### ~~1. Search/Filter Functionality~~ DONE
**Description:** Allow users to filter feed items by keyword in title or description.
**Value:** Essential for users with many subscriptions to find specific content.

**Implementation:**
- Press `/` to start search mode in FeedList or Favorites view
- Type a search query to filter items by title or description (case-insensitive)
- Press `Enter` to confirm and keep filter active while navigating
- Press `Esc` to cancel search (clears filter)
- When filter is active, press `Esc` to clear filter (instead of quit)
- Filter indicator shows in title bar: `[Filter: "query"]`
- All actions (open, read, favorite) work correctly with filtered items

#### ~~2. Feed Export/Import via Clipboard~~ DONE
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

**Implementation:**
- Press `e` in Feed Manager to export all feed URLs to clipboard
- Press `i` in Feed Manager to start import mode (clipboard content auto-pasted)
- Press `Enter` to validate and import feeds, `Esc` to cancel
- Import validates each URL as valid RSS/Atom feed
- Skips duplicates and invalid URLs with informative message

#### ~~3. OPML Import/Export~~ DONE
**Description:** Support importing and exporting feed lists in OPML format (industry standard).
**Value:** Compatibility with other RSS readers for migration.

**Implementation:**
- Press `E` (shift+e) in Feed Manager to export feeds to OPML file (`~/.config/reedy/feeds.opml`)
- Press `I` (shift+i) in Feed Manager to import feeds from OPML file (`~/.config/reedy/feeds.opml`)
- OPML export preserves feed categories as nested outline elements
- OPML import recognizes category structure and assigns categories to imported feeds
- Skips duplicate URLs already in the feed list
- Uses feed titles from OPML, falls back to URL if title is empty
- Complements existing clipboard export (`e`) and import (`i`) for simpler operations
- Help menu updated with OPML documentation

#### ~~4. Feed Title Display~~ DONE
**Description:** Extract and display the actual feed title instead of showing the URL.
**Value:** Much better UX; users can identify feeds at a glance.

**Implementation:**
- Added `FeedInfo` struct with `url` and `title` fields to store feed subscriptions
- When adding a feed (via manual entry or import), the feed title is extracted from the RSS/Atom `<title>` element
- Feed Manager now displays feed titles instead of raw URLs
- Feed items in the list view show "Item Title | Feed Title" instead of "Item Title | URL"
- Backwards compatible: migrates old saved state files that only stored URLs (uses URL as title initially)

#### ~~5. Unread Count per Feed~~ DONE
**Description:** Show the number of unread items for each feed in the feed manager.
**Value:** Helps users quickly identify which feeds have new content.

**Implementation:**
- Added `feed_url` field to `FeedItem` struct to track which feed each item belongs to
- Added `count_unread_for_feed()` and `count_total_for_feed()` methods to `App`
- Feed Manager now displays unread/total count next to each feed title: "Feed Title (3/10)"
- Unread counts are highlighted in cyan when there are unread items
- Uses `#[serde(default)]` for backwards compatibility with existing cached feed data

#### ~~6. Configurable HTTP Timeout~~ DONE
**Description:** Add configuration for request timeouts to prevent hangs.
**Value:** Prevents application from becoming unresponsive.

**Implementation:**
- Added `Config` struct with `http_timeout_secs` field (default: 30 seconds)
- Configuration stored in `~/.config/reedy/config.json`
- Config is loaded on startup and used for all HTTP requests
- If config file doesn't exist, default values are used
- `fetch_feed()` public API accepts optional timeout parameter for flexibility

---

### Medium Priority

#### ~~7. Feed Categories/Tags~~ DONE
**Description:** Allow users to organize feeds into custom categories or tag them.
**Value:** Better organization for users with many subscriptions.

**Implementation:**
- Added optional `category` field to `FeedInfo` struct with `#[serde(default)]` for backwards compatibility
- Press `t` in Feed Manager to set a category for the selected feed
- Categories are displayed as section headers in Feed Manager, grouping feeds visually
- Uncategorized feeds appear first, followed by categorized feeds sorted alphabetically by category
- Enter empty text to remove a category from a feed
- Category is persisted in the saved state file

#### ~~8. Auto-Refresh Interval~~ DONE
**Description:** Automatically refresh feeds at a configurable interval.
**Value:** Users see new content without manual refresh.

**Implementation:**
- Added `auto_refresh_mins` field to `Config` struct (default: 0 = disabled)
- Configuration stored in `~/.config/reedy/config.json`
- `tick()` method checks if refresh interval has elapsed and sets `auto_refresh_pending` flag
- `perform_auto_refresh()` async method executes the refresh when triggered
- Auto-refresh works in FeedList and Favorites view modes
- Title bar displays countdown timer when auto-refresh is enabled: `[Auto: M:SS]`
- Manual refresh (via `c` key) resets the auto-refresh timer
- Auto-refresh is skipped during input modes (adding, searching, etc.)

#### ~~9. Article Preview Pane~~ DONE
**Description:** Show full article content in a dedicated pane within the TUI.
**Value:** Read articles without leaving the terminal.

**Implementation:**
- Press `p` in FeedList or Favorites view to open the article preview pane
- Preview displays: article title, feed name, date, link, read/favorite status, and full description
- Navigation: `↑/k` scroll up, `↓/j` scroll down, `PgUp/PgDn` page scroll, `g` scroll to top
- Actions in preview: `o` open in browser, `r` toggle read, `f` toggle favorite
- Press `Esc`, `q`, or `p` to close preview and return to feed list
- Content is word-wrapped to fit terminal width
- Scroll position indicator shows current line and total lines when content overflows

#### ~~10. Configurable Cache Duration~~ DONE
**Description:** Let users configure how long feed cache remains valid (previously hardcoded to 1 hour).
**Value:** Flexibility for different use cases (low bandwidth vs. always fresh).

**Implementation:**
- Added `cache_duration_mins` field to `Config` struct (default: 60 minutes)
- Configuration stored in `~/.config/reedy/config.json`
- `load_feed_cache()` now uses the configured duration instead of hardcoded 3600 seconds
- Backwards compatible: uses `#[serde(default)]` for missing field in existing config files

#### ~~11. Theme Customization~~ DONE
**Description:** Support light/dark themes and customizable color schemes.
**Value:** Accessibility and user preference support.

**Implementation:**
- Added `Theme` struct with 8 customizable color fields: `primary`, `secondary`, `text`, `muted`, `error`, `highlight`, `description`, `category`
- Theme is stored in the `Config` struct and persisted in `~/.config/reedy/config.json`
- Colors can be specified as named colors (e.g., "green", "light_blue", "dark_gray") or hex codes (e.g., "#ff5500")
- Added `parse_color()` function to convert color strings to ratatui `Color` values
- Includes a built-in `Theme::light()` preset for light terminal backgrounds
- All UI rendering functions now use theme colors instead of hardcoded values
- Backwards compatible: uses `#[serde(default)]` for all theme fields

---

### Low Priority

#### ~~12. Keyboard Shortcuts Customization~~ DONE
**Description:** Allow users to configure their own keybindings via config file.
**Value:** Power users can optimize their workflow.

**Implementation:**
- Added `Keybindings` struct with 24 customizable key actions
- Keybindings stored in `~/.config/reedy/config.json` under the `keybindings` field
- Each keybinding supports multiple keys via comma separation (e.g., "k,Up" for both 'k' and Up arrow)
- Key parser supports special keys: Enter, Esc, Up, Down, Left, Right, PageUp, PageDown, Home, End, Tab, Space, Backspace, Delete
- All navigation, action, and UI keys are customizable except text input modes
- Help menu dynamically displays configured keybindings
- Full backwards compatibility: missing keybinding fields use vim-style defaults

#### ~~13. Vi-Style Commands~~ DONE
**Description:** Support command mode with `:q`, `:w`, `:wq` style commands.
**Value:** Familiar to vim users; consistent terminal experience.

**Implementation:**
- Press `:` in FeedList, FeedManager, or Favorites view to enter command mode
- Type a command and press `Enter` to execute, or `Esc` to cancel
- Supported commands:
  - `:q` or `:quit` - Quit the application
  - `:w` or `:write` or `:save` - Save application state
  - `:wq` or `:x` - Save and quit
  - `:q!` - Force quit without explicit save
  - `:help` or `:h` - Toggle help menu
  - `:feeds` or `:manage` - Open feed manager
  - `:favorites` or `:fav` - Toggle favorites view
  - `:read` or `:markread` - Mark all items as read
  - `:refresh` or `:r` - Refresh feeds
  - `:0` or `:top` or `:gg` - Scroll to top
  - `:$` or `:bottom` - Scroll to bottom
- Command mode displays `:command█` in command bar
- Help menus updated with vi-style command documentation

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

#### ~~19. Vim-Style `G` for Bottom~~ DONE
**Description:** Add `G` (shift+g) to scroll to the bottom of the list (complement to `g` for top).
**Value:** Standard vim navigation pattern.

**Implementation:**
- Press `G` (shift+g) in FeedList, Favorites, or FeedManager to scroll to the bottom and select the last item
- Works with filtered results (respects the visible item count when a search filter is active)
- In article preview mode, `G` scrolls to the end of the article content
- Complements the existing `g` command for scrolling to the top
- Help menu updated with `G` documentation for all modes

---

## Technical Debt

1. ~~**Remove `rss_manager.rs`**~~ - ✓ DONE - Dead code removed
2. ~~**Consolidate module declarations**~~ - ✓ DONE - Duplicate declarations removed from `main.rs`
3. **Add comprehensive test coverage** - Current tests only cover basic state transitions
4. **Add integration tests** - Test actual feed fetching with mock server
5. **Document public API** - Add rustdoc comments to all public functions
6. **Error type consolidation** - Create proper error enum instead of `Box<dyn Error>`

---

## Configuration File

The application uses `~/.config/reedy/config.json` for user configuration:

**Currently Implemented:**
- ~~Cache duration~~ ✓ (`cache_duration_mins`)
- ~~HTTP timeout~~ ✓ (`http_timeout_secs`)
- ~~Default keybindings~~ ✓ (`keybindings` object with 24 customizable keys)
- ~~Theme/colors~~ ✓ (`theme` object with 8 color fields)
- ~~Auto-refresh interval~~ ✓ (`auto_refresh_mins`)

**Future:**
- Notification preferences
