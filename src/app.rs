use atom_syndication::Feed as AtomFeed;
use base64;
use chrono::DateTime;
use crossterm::terminal;
use html2text;
use log::{debug, error, info, warn};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use reqwest;
use rss::Channel;
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Write};
use std::{
    collections::HashMap, collections::HashSet, error, fs, path::PathBuf, time::Duration,
    time::SystemTime,
};

/// Default HTTP request timeout in seconds
const DEFAULT_HTTP_TIMEOUT_SECS: u64 = 30;

/// Default auto-refresh interval in minutes (0 = disabled)
const DEFAULT_AUTO_REFRESH_MINS: u64 = 0;

/// Default cache duration in minutes (60 = 1 hour)
const DEFAULT_CACHE_DURATION_MINS: u64 = 60;

/// Default notifications enabled setting (false = disabled)
const DEFAULT_NOTIFICATIONS_ENABLED: bool = false;

/// Default mark read on scroll setting (false = disabled)
const DEFAULT_MARK_READ_ON_SCROLL: bool = false;

/// Copies text to clipboard using OSC 52 escape sequence.
/// This works over SSH and through tmux, unlike native clipboard APIs.
/// Returns Ok(()) on success, Err with message on failure.
pub fn copy_to_clipboard_osc52(text: &str) -> Result<(), String> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;

    let encoded = STANDARD.encode(text);
    // OSC 52 sequence: \x1b]52;c;<base64>\x07
    // 'c' = clipboard selection
    let osc52_seq = format!("\x1b]52;c;{}\x07", encoded);

    // Write directly to stdout
    let mut stdout = std::io::stdout();
    stdout
        .write_all(osc52_seq.as_bytes())
        .map_err(|e| format!("Failed to write OSC 52: {}", e))?;
    stdout
        .flush()
        .map_err(|e| format!("Failed to flush stdout: {}", e))?;

    debug!("Copied {} bytes to clipboard via OSC 52", text.len());
    Ok(())
}

/// Color theme for the application UI
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Theme {
    /// Primary accent color (titles, headers)
    #[serde(default = "default_primary")]
    pub primary: String,
    /// Secondary accent color (selected items, section headers)
    #[serde(default = "default_secondary")]
    pub secondary: String,
    /// Default text color
    #[serde(default = "default_text")]
    pub text: String,
    /// Muted text color (read items, inactive elements)
    #[serde(default = "default_muted")]
    pub muted: String,
    /// Error message color
    #[serde(default = "default_error")]
    pub error: String,
    /// Highlight color (unread counts, links)
    #[serde(default = "default_highlight")]
    pub highlight: String,
    /// Description/secondary text color
    #[serde(default = "default_description")]
    pub description: String,
    /// Category header color
    #[serde(default = "default_category")]
    pub category: String,
}

fn default_primary() -> String {
    "green".to_string()
}

fn default_secondary() -> String {
    "yellow".to_string()
}

fn default_text() -> String {
    "white".to_string()
}

fn default_muted() -> String {
    "dark_gray".to_string()
}

fn default_error() -> String {
    "red".to_string()
}

fn default_highlight() -> String {
    "cyan".to_string()
}

fn default_description() -> String {
    "gray".to_string()
}

fn default_category() -> String {
    "magenta".to_string()
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary: default_primary(),
            secondary: default_secondary(),
            text: default_text(),
            muted: default_muted(),
            error: default_error(),
            highlight: default_highlight(),
            description: default_description(),
            category: default_category(),
        }
    }
}

impl Theme {
    /// Returns a light theme suitable for light terminal backgrounds
    pub fn light() -> Self {
        Self {
            primary: "blue".to_string(),
            secondary: "magenta".to_string(),
            text: "black".to_string(),
            muted: "dark_gray".to_string(),
            error: "red".to_string(),
            highlight: "blue".to_string(),
            description: "dark_gray".to_string(),
            category: "magenta".to_string(),
        }
    }
}

/// Customizable keyboard shortcuts
/// Each field contains a comma-separated list of keys that trigger the action.
/// Supported formats: single characters (e.g., "j"), special keys (e.g., "Up", "Enter", "Esc"),
/// and multiple keys (e.g., "j,Down" for vim-style + arrow key navigation).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Keybindings {
    // Navigation
    #[serde(default = "default_move_up")]
    pub move_up: String,
    #[serde(default = "default_move_down")]
    pub move_down: String,
    #[serde(default = "default_page_up")]
    pub page_up: String,
    #[serde(default = "default_page_down")]
    pub page_down: String,
    #[serde(default = "default_scroll_to_top")]
    pub scroll_to_top: String,
    #[serde(default = "default_scroll_to_bottom")]
    pub scroll_to_bottom: String,

    // Actions
    #[serde(default = "default_select")]
    pub select: String,
    #[serde(default = "default_open_in_browser")]
    pub open_in_browser: String,
    #[serde(default = "default_copy_link")]
    pub copy_link: String,
    #[serde(default = "default_toggle_read")]
    pub toggle_read: String,
    #[serde(default = "default_mark_all_read")]
    pub mark_all_read: String,
    #[serde(default = "default_toggle_favorite")]
    pub toggle_favorite: String,
    #[serde(default = "default_toggle_favorites_view")]
    pub toggle_favorites_view: String,
    #[serde(default = "default_refresh")]
    pub refresh: String,

    // Search & Filter
    #[serde(default = "default_start_search")]
    pub start_search: String,
    #[serde(default = "default_toggle_unread_only")]
    pub toggle_unread_only: String,

    // Preview
    #[serde(default = "default_open_preview")]
    pub open_preview: String,

    // Feed Manager
    #[serde(default = "default_open_feed_manager")]
    pub open_feed_manager: String,
    #[serde(default = "default_add_feed")]
    pub add_feed: String,
    #[serde(default = "default_delete_feed")]
    pub delete_feed: String,
    #[serde(default = "default_set_category")]
    pub set_category: String,
    #[serde(default = "default_export_clipboard")]
    pub export_clipboard: String,
    #[serde(default = "default_export_opml")]
    pub export_opml: String,
    #[serde(default = "default_import_clipboard")]
    pub import_clipboard: String,
    #[serde(default = "default_import_opml")]
    pub import_opml: String,

    // UI
    #[serde(default = "default_help")]
    pub help: String,
    #[serde(default = "default_quit")]
    pub quit: String,

    // Export
    #[serde(default = "default_export_article")]
    pub export_article: String,
}

// Default keybinding functions
fn default_move_up() -> String {
    "k,Up".to_string()
}
fn default_move_down() -> String {
    "j,Down".to_string()
}
fn default_page_up() -> String {
    "PageUp".to_string()
}
fn default_page_down() -> String {
    "PageDown".to_string()
}
fn default_scroll_to_top() -> String {
    "g".to_string()
}
fn default_scroll_to_bottom() -> String {
    "G".to_string()
}
fn default_select() -> String {
    "Enter".to_string()
}
fn default_open_in_browser() -> String {
    "o".to_string()
}
fn default_copy_link() -> String {
    "O".to_string()
}
fn default_toggle_read() -> String {
    "r".to_string()
}
fn default_mark_all_read() -> String {
    "R".to_string()
}
fn default_toggle_favorite() -> String {
    "f".to_string()
}
fn default_toggle_favorites_view() -> String {
    "F".to_string()
}
fn default_refresh() -> String {
    "c".to_string()
}
fn default_start_search() -> String {
    "/".to_string()
}
fn default_toggle_unread_only() -> String {
    "u".to_string()
}
fn default_open_preview() -> String {
    "p".to_string()
}
fn default_open_feed_manager() -> String {
    "m".to_string()
}
fn default_add_feed() -> String {
    "a".to_string()
}
fn default_delete_feed() -> String {
    "d".to_string()
}
fn default_set_category() -> String {
    "t".to_string()
}
fn default_export_clipboard() -> String {
    "e".to_string()
}
fn default_export_opml() -> String {
    "E".to_string()
}
fn default_import_clipboard() -> String {
    "i".to_string()
}
fn default_import_opml() -> String {
    "I".to_string()
}
fn default_help() -> String {
    "?".to_string()
}
fn default_quit() -> String {
    "q".to_string()
}
fn default_export_article() -> String {
    "s".to_string()
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            move_up: default_move_up(),
            move_down: default_move_down(),
            page_up: default_page_up(),
            page_down: default_page_down(),
            scroll_to_top: default_scroll_to_top(),
            scroll_to_bottom: default_scroll_to_bottom(),
            select: default_select(),
            open_in_browser: default_open_in_browser(),
            copy_link: default_copy_link(),
            toggle_read: default_toggle_read(),
            mark_all_read: default_mark_all_read(),
            toggle_favorite: default_toggle_favorite(),
            toggle_favorites_view: default_toggle_favorites_view(),
            refresh: default_refresh(),
            start_search: default_start_search(),
            toggle_unread_only: default_toggle_unread_only(),
            open_preview: default_open_preview(),
            open_feed_manager: default_open_feed_manager(),
            add_feed: default_add_feed(),
            delete_feed: default_delete_feed(),
            set_category: default_set_category(),
            export_clipboard: default_export_clipboard(),
            export_opml: default_export_opml(),
            import_clipboard: default_import_clipboard(),
            import_opml: default_import_opml(),
            help: default_help(),
            quit: default_quit(),
            export_article: default_export_article(),
        }
    }
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// HTTP request timeout in seconds (default: 30)
    #[serde(default = "default_http_timeout")]
    pub http_timeout_secs: u64,
    /// Auto-refresh interval in minutes (default: 0 = disabled)
    #[serde(default = "default_auto_refresh")]
    pub auto_refresh_mins: u64,
    /// Cache duration in minutes (default: 60 = 1 hour)
    #[serde(default = "default_cache_duration")]
    pub cache_duration_mins: u64,
    /// Desktop notifications for new articles (default: false)
    #[serde(default = "default_notifications_enabled")]
    pub notifications_enabled: bool,
    /// Auto-mark items as read when scrolling past them (default: false)
    #[serde(default = "default_mark_read_on_scroll")]
    pub mark_read_on_scroll: bool,
    /// Color theme (default: dark theme)
    #[serde(default)]
    pub theme: Theme,
    /// Custom keyboard shortcuts (default: vim-style bindings)
    #[serde(default)]
    pub keybindings: Keybindings,
}

fn default_http_timeout() -> u64 {
    DEFAULT_HTTP_TIMEOUT_SECS
}

fn default_auto_refresh() -> u64 {
    DEFAULT_AUTO_REFRESH_MINS
}

fn default_cache_duration() -> u64 {
    DEFAULT_CACHE_DURATION_MINS
}

fn default_notifications_enabled() -> bool {
    DEFAULT_NOTIFICATIONS_ENABLED
}

fn default_mark_read_on_scroll() -> bool {
    DEFAULT_MARK_READ_ON_SCROLL
}

impl Default for Config {
    fn default() -> Self {
        Self {
            http_timeout_secs: DEFAULT_HTTP_TIMEOUT_SECS,
            auto_refresh_mins: DEFAULT_AUTO_REFRESH_MINS,
            cache_duration_mins: DEFAULT_CACHE_DURATION_MINS,
            notifications_enabled: DEFAULT_NOTIFICATIONS_ENABLED,
            mark_read_on_scroll: DEFAULT_MARK_READ_ON_SCROLL,
            theme: Theme::default(),
            keybindings: Keybindings::default(),
        }
    }
}

/// Creates a reqwest client with a configured timeout to prevent hanging on slow/unresponsive feeds
fn create_http_client(timeout_secs: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

#[derive(Debug, PartialEq)]
pub enum InputMode {
    Normal,
    Adding,
    Deleting,
    FeedManager,
    Help,
    Searching,
    Importing,
    SettingCategory,
    Preview,
    Command,
}

#[derive(Debug, PartialEq)]
pub enum PageMode {
    FeedList,
    FeedManager,
    Favorites,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedItem {
    pub title: String,
    pub description: String,
    pub link: String,
    pub published: Option<SystemTime>,
    pub id: String,
    #[serde(default)]
    pub feed_url: String,
}

/// Represents a feed subscription with its URL, title, and optional category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedInfo {
    pub url: String,
    pub title: String,
    #[serde(default)]
    pub category: Option<String>,
}

/// Feed health status
#[derive(Debug, Clone, PartialEq)]
pub enum FeedStatus {
    /// Feed is healthy and responding normally
    Healthy,
    /// Feed responded but took longer than expected (>5 seconds)
    Slow,
    /// Feed failed to load or parse
    Broken,
    /// Feed has not been checked yet
    Unknown,
}

/// Tracks the health status of a feed
#[derive(Debug, Clone)]
pub struct FeedHealth {
    /// Current status of the feed
    pub status: FeedStatus,
    /// Time of the last successful fetch
    pub last_success: Option<SystemTime>,
    /// Response time of the last fetch attempt (in milliseconds)
    pub last_response_time_ms: Option<u64>,
    /// Error message from the last failed attempt
    pub last_error: Option<String>,
    /// Number of consecutive failures
    pub consecutive_failures: u32,
}

impl Default for FeedHealth {
    fn default() -> Self {
        Self {
            status: FeedStatus::Unknown,
            last_success: None,
            last_response_time_ms: None,
            last_error: None,
            consecutive_failures: 0,
        }
    }
}

impl FeedHealth {
    /// Returns a display string for the health status
    pub fn status_indicator(&self) -> &'static str {
        match self.status {
            FeedStatus::Healthy => "●", // Green dot
            FeedStatus::Slow => "◐",    // Half-filled circle (slow)
            FeedStatus::Broken => "✗",  // X mark (broken)
            FeedStatus::Unknown => "○", // Empty circle (unknown)
        }
    }

    /// Returns a human-readable status description
    pub fn status_description(&self) -> String {
        match self.status {
            FeedStatus::Healthy => {
                if let Some(ms) = self.last_response_time_ms {
                    format!("OK ({}ms)", ms)
                } else {
                    "OK".to_string()
                }
            }
            FeedStatus::Slow => {
                if let Some(ms) = self.last_response_time_ms {
                    format!("Slow ({}ms)", ms)
                } else {
                    "Slow".to_string()
                }
            }
            FeedStatus::Broken => {
                if let Some(ref err) = self.last_error {
                    format!("Error: {}", err)
                } else {
                    "Broken".to_string()
                }
            }
            FeedStatus::Unknown => "Not checked".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SavedState {
    feeds: Vec<FeedInfo>,
    read_items: HashSet<String>,
    favorites: HashSet<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedFeed {
    url: String,
    content: Vec<FeedItem>,
    last_updated: SystemTime,
}

#[derive(Debug)]
pub struct App {
    pub running: bool,
    pub input_mode: InputMode,
    pub page_mode: PageMode,
    pub input_buffer: String,
    pub rss_feeds: Vec<FeedInfo>,
    pub selected_index: Option<usize>,
    pub current_feed_content: Vec<FeedItem>,
    pub error_message: Option<String>,
    /// Status message shown at bottom (for success notifications)
    pub status_message: Option<String>,
    save_path: PathBuf,
    read_items: HashSet<String>,
    pub favorites: HashSet<String>,
    pub scroll: u16,
    pub terminal_width: u16,
    pub terminal_height: u16,
    pub search_query: String,
    pub filtered_indices: Option<Vec<usize>>,
    pub import_result: Option<String>,
    pub config: Config,
    /// Timestamp of the last feed refresh
    pub last_refresh: Option<SystemTime>,
    /// Flag indicating an auto-refresh is pending (set by tick, consumed by main loop)
    pub auto_refresh_pending: bool,
    /// Scroll position for the article preview pane
    pub preview_scroll: u16,
    /// Buffer for vi-style command mode (e.g., :q, :w, :wq)
    pub command_buffer: String,
    /// Health status for each feed (keyed by URL)
    pub feed_health: HashMap<String, FeedHealth>,
    /// Item IDs that have already been seen (for notification tracking)
    seen_items: HashSet<String>,
    /// Filter to show only unread items
    pub show_unread_only: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            running: true,
            input_mode: InputMode::Normal,
            page_mode: PageMode::FeedList,
            input_buffer: String::new(),
            rss_feeds: Vec::new(),
            selected_index: None,
            current_feed_content: Vec::new(),
            error_message: None,
            status_message: None,
            save_path: Self::get_save_path(),
            read_items: HashSet::new(),
            favorites: HashSet::new(),
            scroll: 0,
            terminal_width: 80,
            terminal_height: 24,
            search_query: String::new(),
            filtered_indices: None,
            import_result: None,
            config: Config::default(),
            last_refresh: None,
            auto_refresh_pending: false,
            preview_scroll: 0,
            command_buffer: String::new(),
            feed_health: HashMap::new(),
            seen_items: HashSet::new(),
            show_unread_only: false,
        }
    }
}

impl App {
    pub async fn new() -> Self {
        let mut app = Self {
            config: Self::load_config(),
            ..Default::default()
        };

        // Get initial terminal size
        if let Ok((width, height)) = terminal::size() {
            app.terminal_width = width;
            app.terminal_height = height;
        }

        app.load_feeds().unwrap_or_else(|e| {
            error!("Failed to load feeds: {}", e);
            app.error_message = Some(format!("Failed to load feeds: {}", e));
        });

        // Cache all feeds and load all cached content
        if !app.rss_feeds.is_empty() {
            app.selected_index = Some(0);

            // Load cached content first to populate seen_items (prevents notifications on startup)
            let mut all_items = Vec::new();
            for feed in &app.rss_feeds {
                if let Some(cached_items) = app.load_feed_cache(&feed.url) {
                    all_items.extend(cached_items);
                }
            }
            // Populate seen_items with cached item IDs to avoid startup notifications
            for item in &all_items {
                app.seen_items.insert(item.id.clone());
            }
            app.current_feed_content = all_items;

            // Now refresh feeds (notifications will only fire for truly new items)
            // This will use cache if valid, only fetching expired feeds
            let _ = app.refresh_all_feeds().await;

            // Reload and combine all cached feed content
            let mut all_items = Vec::new();
            for feed in &app.rss_feeds {
                if let Some(cached_items) = app.load_feed_cache(&feed.url) {
                    all_items.extend(cached_items);
                }
            }

            // Sort all items by date, newest first
            all_items.sort_by(|a, b| b.published.cmp(&a.published));
            app.current_feed_content = all_items;
        }

        // Record the initial refresh time
        app.last_refresh = Some(SystemTime::now());

        app
    }

    /// Handles the tick event of the terminal.
    /// Checks if auto-refresh is due and sets the auto_refresh_pending flag.
    pub fn tick(&mut self) {
        // Skip auto-refresh check if disabled or no feeds
        if self.config.auto_refresh_mins == 0 || self.rss_feeds.is_empty() {
            return;
        }

        // Skip if we're not in a view that should auto-refresh (e.g., not in help or input modes)
        if self.input_mode != InputMode::Normal {
            return;
        }

        // Check if it's time for an auto-refresh
        if let Some(last_refresh) = self.last_refresh {
            if let Ok(elapsed) = last_refresh.elapsed() {
                let refresh_interval = Duration::from_secs(self.config.auto_refresh_mins * 60);
                if elapsed >= refresh_interval {
                    debug!(
                        "Auto-refresh triggered after {} minutes",
                        self.config.auto_refresh_mins
                    );
                    self.auto_refresh_pending = true;
                }
            }
        }
    }

    /// Performs the auto-refresh if pending. Called from the main loop.
    pub async fn perform_auto_refresh(&mut self) {
        if !self.auto_refresh_pending {
            return;
        }

        self.auto_refresh_pending = false;

        // Only auto-refresh in FeedList or Favorites mode
        match self.page_mode {
            PageMode::FeedList => {
                info!("Auto-refreshing feeds...");
                if let Err(e) = self.refresh_all_feeds().await {
                    error!("Auto-refresh failed: {}", e);
                } else {
                    self.last_refresh = Some(SystemTime::now());
                }
            }
            PageMode::Favorites => {
                // For favorites, refresh all feeds then re-filter to favorites
                info!("Auto-refreshing feeds (favorites view)...");
                if let Err(e) = self.refresh_all_feeds().await {
                    error!("Auto-refresh failed: {}", e);
                } else {
                    // Re-filter to show only favorites
                    let favorites: Vec<FeedItem> = self
                        .current_feed_content
                        .iter()
                        .filter(|item| self.favorites.contains(&item.id))
                        .cloned()
                        .collect();
                    self.current_feed_content = favorites;
                    // Reset selection state after content change
                    self.selected_index = if self.current_feed_content.is_empty() {
                        None
                    } else {
                        Some(0)
                    };
                    self.filtered_indices = None;
                    self.scroll = 0;
                    self.last_refresh = Some(SystemTime::now());
                }
            }
            PageMode::FeedManager => {
                // Don't auto-refresh in feed manager mode
                self.last_refresh = Some(SystemTime::now());
            }
        }
    }

    /// Returns the time until the next auto-refresh, or None if auto-refresh is disabled.
    pub fn time_until_next_refresh(&self) -> Option<Duration> {
        if self.config.auto_refresh_mins == 0 {
            return None;
        }

        let refresh_interval = Duration::from_secs(self.config.auto_refresh_mins * 60);
        if let Some(last_refresh) = self.last_refresh {
            if let Ok(elapsed) = last_refresh.elapsed() {
                if elapsed < refresh_interval {
                    return Some(refresh_interval - elapsed);
                }
            }
        }

        Some(Duration::from_secs(0))
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn get_save_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("reedy");
        if let Err(e) = fs::create_dir_all(&path) {
            error!("Failed to create config directory {:?}: {}", path, e);
        }
        path.push("feeds.json");
        path
    }

    pub fn get_log_path() -> PathBuf {
        let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("reedy");
        path.push("logs");
        if let Err(e) = fs::create_dir_all(&path) {
            eprintln!("Failed to create log directory {:?}: {}", path, e);
        }
        path.push("reedy.log");
        path
    }

    pub fn get_config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("reedy");
        if let Err(e) = fs::create_dir_all(&path) {
            error!("Failed to create config directory {:?}: {}", path, e);
        }
        path.push("config.json");
        path
    }

    pub fn load_config() -> Config {
        let config_path = Self::get_config_path();
        if config_path.exists() {
            match fs::read_to_string(&config_path) {
                Ok(contents) => match serde_json::from_str::<Config>(&contents) {
                    Ok(config) => return config,
                    Err(e) => {
                        warn!(
                            "Failed to parse config file {}: {}. Using defaults.",
                            config_path.display(),
                            e
                        );
                    }
                },
                Err(e) => {
                    warn!(
                        "Failed to read config file {}: {}. Using defaults.",
                        config_path.display(),
                        e
                    );
                }
            }
        }
        // Return default config if file doesn't exist or can't be parsed
        Config::default()
    }

    pub fn save_config(&self) -> AppResult<()> {
        let config_path = Self::get_config_path();
        let json = serde_json::to_string_pretty(&self.config)?;
        fs::write(config_path, json)?;
        Ok(())
    }

    fn create_item_id(title: &str, published: Option<SystemTime>, feed_url: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        feed_url.hash(&mut hasher);
        let url_hash = hasher.finish();

        let title_slug = title
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric(), "_");

        if let Some(time) = published {
            let nanos = time
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            format!("{}_{:x}_{}", title_slug, url_hash, nanos)
        } else {
            format!("{}_{:x}", title_slug, url_hash)
        }
    }

    pub fn is_item_read(&self, item: &FeedItem) -> bool {
        self.read_items.contains(&item.id)
    }

    /// Returns the count of unread items for a given feed URL.
    /// Uses cached feed content to determine the count.
    pub fn count_unread_for_feed(&self, url: &str) -> usize {
        if let Some(items) = self.load_feed_cache(url) {
            items.iter().filter(|item| !self.is_item_read(item)).count()
        } else {
            0
        }
    }

    /// Returns the total count of items for a given feed URL.
    /// Uses cached feed content to determine the count.
    pub fn count_total_for_feed(&self, url: &str) -> usize {
        if let Some(items) = self.load_feed_cache(url) {
            items.len()
        } else {
            0
        }
    }

    /// Returns the health status for a given feed URL.
    /// Returns a default Unknown status if the feed hasn't been checked yet.
    pub fn get_feed_health(&self, url: &str) -> FeedHealth {
        self.feed_health.get(url).cloned().unwrap_or_default()
    }

    pub fn toggle_read_status(&mut self) {
        if let Some(visible_index) = self.selected_index {
            if let Some(actual_index) = self.get_actual_index(visible_index) {
                if let Some(item) = self.current_feed_content.get(actual_index) {
                    if self.read_items.contains(&item.id) {
                        self.read_items.remove(&item.id);
                        debug!("Marked item as unread: {}", item.title);
                    } else {
                        self.read_items.insert(item.id.clone());
                        debug!("Marked item as read: {}", item.title);
                    }
                    self.save_state().unwrap_or_else(|e| {
                        error!("Failed to save read status: {}", e);
                    });
                }
            }
        }
    }

    /// Marks the currently selected item as read if mark_read_on_scroll is enabled.
    /// This is called when navigating away from an item (scrolling to the next one).
    /// Does not save state immediately to avoid excessive disk writes during rapid scrolling;
    /// state will be saved on quit or next explicit save action.
    fn mark_current_as_read_on_scroll(&mut self) {
        if !self.config.mark_read_on_scroll {
            return;
        }

        // Only mark read in FeedList or Favorites mode
        if self.page_mode != PageMode::FeedList && self.page_mode != PageMode::Favorites {
            return;
        }

        if let Some(visible_index) = self.selected_index {
            if let Some(actual_index) = self.get_actual_index(visible_index) {
                if let Some(item) = self.current_feed_content.get(actual_index) {
                    if !self.read_items.contains(&item.id) {
                        self.read_items.insert(item.id.clone());
                        debug!("Auto-marked item as read on scroll: {}", item.title);
                    }
                }
            }
        }
    }

    fn save_state(&self) -> AppResult<()> {
        let saved = SavedState {
            feeds: self.rss_feeds.clone(),
            read_items: self.read_items.clone(),
            favorites: self.favorites.clone(),
        };
        let content = serde_json::to_string_pretty(&saved)?;
        fs::write(&self.save_path, content)?;
        debug!(
            "Saved {} feeds, {} read items, and {} favorites to {}",
            self.rss_feeds.len(),
            self.read_items.len(),
            self.favorites.len(),
            self.save_path.display()
        );
        Ok(())
    }

    fn load_feeds(&mut self) -> AppResult<()> {
        if self.save_path.exists() {
            let content = match fs::read_to_string(&self.save_path) {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to read feeds file: {}. Clearing corrupted data.", e);
                    if let Err(e) = fs::remove_file(&self.save_path) {
                        error!("Failed to remove corrupted file: {}", e);
                    }
                    self.error_message = Some(
                        "Feeds data was corrupted and has been cleared. Starting fresh."
                            .to_string(),
                    );
                    return Ok(());
                }
            };

            // Try to parse with new format first (Vec<FeedInfo>)
            match serde_json::from_str::<SavedState>(&content) {
                Ok(saved) => {
                    self.rss_feeds = saved.feeds;
                    self.read_items = saved.read_items;
                    self.favorites = saved.favorites;
                    debug!(
                        "Loaded {} feeds and {} favorites from {}",
                        self.rss_feeds.len(),
                        self.favorites.len(),
                        self.save_path.display()
                    );
                }
                Err(_) => {
                    // Try parsing middle format (with favorites, but Vec<String> for feeds)
                    #[derive(Debug, Serialize, Deserialize)]
                    struct MiddleSavedState {
                        feeds: Vec<String>,
                        read_items: HashSet<String>,
                        favorites: HashSet<String>,
                    }

                    if let Ok(middle_saved) = serde_json::from_str::<MiddleSavedState>(&content) {
                        // Convert Vec<String> to Vec<FeedInfo> using URL as title
                        self.rss_feeds = middle_saved
                            .feeds
                            .into_iter()
                            .map(|url| FeedInfo {
                                title: url.clone(),
                                url,
                                category: None,
                            })
                            .collect();
                        self.read_items = middle_saved.read_items;
                        self.favorites = middle_saved.favorites;
                        debug!(
                            "Loaded {} feeds from middle format state file {}",
                            self.rss_feeds.len(),
                            self.save_path.display()
                        );
                    } else {
                        // Try parsing oldest format (without favorites)
                        #[derive(Debug, Serialize, Deserialize)]
                        struct OldSavedState {
                            feeds: Vec<String>,
                            read_items: HashSet<String>,
                        }

                        if let Ok(old_saved) = serde_json::from_str::<OldSavedState>(&content) {
                            // Convert Vec<String> to Vec<FeedInfo> using URL as title
                            self.rss_feeds = old_saved
                                .feeds
                                .into_iter()
                                .map(|url| FeedInfo {
                                    title: url.clone(),
                                    url,
                                    category: None,
                                })
                                .collect();
                            self.read_items = old_saved.read_items;
                            self.favorites = HashSet::new(); // Initialize empty favorites
                            debug!(
                                "Loaded {} feeds from old format state file {}",
                                self.rss_feeds.len(),
                                self.save_path.display()
                            );
                        } else {
                            // All parsing attempts failed - clear the corrupted file and start fresh
                            error!(
                                "Failed to parse feeds file: {}. Clearing corrupted data.",
                                self.save_path.display()
                            );
                            if let Err(e) = fs::remove_file(&self.save_path) {
                                error!("Failed to remove corrupted file: {}", e);
                            }
                            self.error_message = Some(
                                "Feeds data was corrupted and has been cleared. Starting fresh."
                                    .to_string(),
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn toggle_feed_manager(&mut self) {
        match self.page_mode {
            PageMode::FeedList => {
                self.page_mode = PageMode::FeedManager;
                self.selected_index = Some(0);
                self.scroll = 0; // Reset scroll position
            }
            PageMode::FeedManager => {
                self.page_mode = PageMode::FeedList;
                // Reset selection and trigger refresh
                self.selected_index = Some(0);
                self.scroll = 0; // Reset scroll position
            }
            PageMode::Favorites => {
                self.page_mode = PageMode::FeedManager;
                self.selected_index = Some(0);
                self.scroll = 0; // Reset scroll position
            }
        }
    }

    pub fn select_previous(&mut self) {
        self.status_message = None; // Clear status on navigation
        if let Some(current) = self.selected_index {
            let len = match self.page_mode {
                PageMode::FeedList | PageMode::Favorites => self.visible_item_count(),
                PageMode::FeedManager => self.rss_feeds.len(),
            };
            if len == 0 {
                return;
            }
            self.selected_index = Some(if current > 0 { current - 1 } else { len - 1 });
            self.ensure_selection_visible();
        }
    }

    pub fn select_next(&mut self) {
        self.status_message = None; // Clear status on navigation
        if let Some(current) = self.selected_index {
            let len = match self.page_mode {
                PageMode::FeedList | PageMode::Favorites => self.visible_item_count(),
                PageMode::FeedManager => self.rss_feeds.len(),
            };
            if len == 0 {
                return;
            }
            // Mark current item as read before moving to the next (if enabled)
            self.mark_current_as_read_on_scroll();
            self.selected_index = Some((current + 1) % len);
            self.ensure_selection_visible();
        }
    }

    /// Calculates the number of visible items based on terminal height and page mode.
    /// This accounts for UI chrome (title bar, command bar, borders, etc.)
    pub fn items_per_page(&self) -> usize {
        // Terminal layout: 3 lines title + 3 lines command bar = 6 lines of chrome
        // Content area has 2 lines for borders
        let content_height = self.terminal_height.saturating_sub(8) as usize;

        match self.page_mode {
            // FeedList/Favorites: each item takes 3 lines (title, description snippet, metadata)
            PageMode::FeedList | PageMode::Favorites => (content_height / 3).max(1),
            // FeedManager: each item takes 1 line, minus 1 for status line
            PageMode::FeedManager => content_height.saturating_sub(1).max(1),
        }
    }

    /// Ensures that the currently selected item is visible in the view
    pub fn ensure_selection_visible(&mut self) {
        if let Some(index) = self.selected_index {
            // Make sure selection is not above the current scroll position
            if (index as u16) < self.scroll {
                self.scroll = index as u16;
            }

            // Calculate the number of visible items in the current view
            let items_per_page = self.items_per_page();

            // Make sure selection is not below the visible area
            if index >= (self.scroll as usize + items_per_page) {
                self.scroll = (index - items_per_page + 1) as u16;
            }
        }
    }

    pub fn start_adding(&mut self) {
        self.input_mode = InputMode::Adding;
        self.input_buffer.clear();
    }

    pub fn cancel_adding(&mut self) {
        self.input_mode = InputMode::Normal;
        self.input_buffer.clear();
        self.clear_error();
    }

    pub fn toggle_help(&mut self) {
        match self.input_mode {
            InputMode::Help => self.input_mode = InputMode::Normal,
            _ => self.input_mode = InputMode::Help,
        }
    }

    pub fn start_deleting(&mut self) {
        if !self.rss_feeds.is_empty() {
            self.input_mode = InputMode::Deleting;
            self.selected_index = Some(0);
        }
    }

    pub fn cancel_deleting(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    pub fn delete_feed(&mut self, index: usize) {
        if index < self.rss_feeds.len() {
            self.rss_feeds.remove(index);
            self.selected_index = None;
            self.current_feed_content.clear();
            if let Err(e) = self.save_feeds() {
                error!("Failed to save feeds after deletion: {}", e);
                self.error_message = Some("Failed to save feeds".to_string());
            }
        }
    }

    /// Exports all feed URLs to the clipboard using OSC 52, one URL per line
    pub fn export_feeds_to_clipboard(&mut self) {
        if self.rss_feeds.is_empty() {
            self.error_message = Some("No feeds to export".to_string());
            return;
        }

        let feed_list: String = self
            .rss_feeds
            .iter()
            .map(|f| f.url.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        match copy_to_clipboard_osc52(&feed_list) {
            Ok(()) => {
                info!("Exported {} feeds to clipboard", self.rss_feeds.len());
                self.status_message = Some(format!(
                    "Exported {} feeds to clipboard",
                    self.rss_feeds.len()
                ));
            }
            Err(e) => {
                error!("Failed to copy to clipboard: {}", e);
                self.error_message = Some(format!("Failed to copy to clipboard: {}", e));
            }
        }
    }

    /// Starts the import mode
    pub fn start_importing(&mut self) {
        self.input_mode = InputMode::Importing;
        self.input_buffer.clear();
        self.import_result = None;

        // Try to pre-fill from clipboard
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                self.input_buffer = text;
            }
        }
    }

    /// Cancels the import mode
    pub fn cancel_importing(&mut self) {
        self.input_mode = InputMode::Normal;
        self.input_buffer.clear();
        self.import_result = None;
        self.clear_error();
    }

    /// Exports feeds to OPML format and saves to a file
    /// Returns the path where the file was saved
    pub fn export_opml(&mut self) -> AppResult<PathBuf> {
        if self.rss_feeds.is_empty() {
            self.error_message = Some("No feeds to export".to_string());
            return Err("No feeds to export".into());
        }

        let opml_content = self.generate_opml()?;

        // Save to the config directory
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("reedy");
        fs::create_dir_all(&path)?;
        path.push("feeds.opml");

        fs::write(&path, opml_content)?;

        info!(
            "Exported {} feeds to OPML: {}",
            self.rss_feeds.len(),
            path.display()
        );
        self.error_message = Some(format!(
            "Exported {} feeds to {}",
            self.rss_feeds.len(),
            path.display()
        ));

        Ok(path)
    }

    /// Generates OPML XML content from current feeds
    fn generate_opml(&self) -> AppResult<String> {
        let mut writer = Writer::new(Cursor::new(Vec::new()));

        // XML declaration
        writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

        // OPML root element
        let mut opml = BytesStart::new("opml");
        opml.push_attribute(("version", "2.0"));
        writer.write_event(Event::Start(opml))?;

        // Head section
        writer.write_event(Event::Start(BytesStart::new("head")))?;
        writer.write_event(Event::Start(BytesStart::new("title")))?;
        writer.write_event(Event::Text(BytesText::new("Reedy RSS Feeds")))?;
        writer.write_event(Event::End(BytesEnd::new("title")))?;
        writer.write_event(Event::End(BytesEnd::new("head")))?;

        // Body section
        writer.write_event(Event::Start(BytesStart::new("body")))?;

        // Group feeds by category
        let feeds_by_category = self.get_feeds_by_category();

        for (category, feeds) in feeds_by_category {
            match category {
                Some(cat_name) => {
                    // Create a category outline
                    let mut cat_outline = BytesStart::new("outline");
                    cat_outline.push_attribute(("text", cat_name.as_str()));
                    cat_outline.push_attribute(("title", cat_name.as_str()));
                    writer.write_event(Event::Start(cat_outline))?;

                    // Write feeds in this category
                    for feed in feeds {
                        let mut outline = BytesStart::new("outline");
                        outline.push_attribute(("type", "rss"));
                        outline.push_attribute(("text", feed.title.as_str()));
                        outline.push_attribute(("title", feed.title.as_str()));
                        outline.push_attribute(("xmlUrl", feed.url.as_str()));
                        writer.write_event(Event::Empty(outline))?;
                    }

                    writer.write_event(Event::End(BytesEnd::new("outline")))?;
                }
                None => {
                    // Write uncategorized feeds at the top level
                    for feed in feeds {
                        let mut outline = BytesStart::new("outline");
                        outline.push_attribute(("type", "rss"));
                        outline.push_attribute(("text", feed.title.as_str()));
                        outline.push_attribute(("title", feed.title.as_str()));
                        outline.push_attribute(("xmlUrl", feed.url.as_str()));
                        writer.write_event(Event::Empty(outline))?;
                    }
                }
            }
        }

        writer.write_event(Event::End(BytesEnd::new("body")))?;
        writer.write_event(Event::End(BytesEnd::new("opml")))?;

        let result = writer.into_inner().into_inner();
        Ok(String::from_utf8(result)?)
    }

    /// Imports feeds from an OPML file
    pub async fn import_opml(&mut self, path: &PathBuf) -> AppResult<()> {
        let content = fs::read_to_string(path)?;
        self.parse_and_import_opml(&content).await
    }

    /// Imports feeds from OPML content string
    pub async fn import_opml_content(&mut self, content: &str) -> AppResult<()> {
        self.parse_and_import_opml(content).await
    }

    /// Parses OPML content and imports feeds
    async fn parse_and_import_opml(&mut self, content: &str) -> AppResult<()> {
        let mut reader = Reader::from_str(content);
        reader.config_mut().trim_text(true);

        let mut added = 0;
        let mut skipped_duplicate = 0;
        let mut category_stack: Vec<String> = Vec::new();

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"outline" => {
                    // This is a start tag (has children) - could be a category or a feed
                    let mut xml_url: Option<String> = None;
                    let mut title: Option<String> = None;

                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"xmlUrl" | b"xmlurl" => {
                                xml_url = Some(String::from_utf8_lossy(&attr.value).to_string());
                            }
                            b"text" | b"title" => {
                                if title.is_none() {
                                    title = Some(String::from_utf8_lossy(&attr.value).to_string());
                                }
                            }
                            _ => {}
                        }
                    }

                    if xml_url.is_some() {
                        // It's a feed with a start tag (unusual but valid)
                        let url = xml_url.unwrap();
                        if self.rss_feeds.iter().any(|f| f.url == url) {
                            skipped_duplicate += 1;
                        } else {
                            let feed_title = title
                                .filter(|t| !t.is_empty())
                                .unwrap_or_else(|| url.clone());
                            let category = category_stack.last().cloned();
                            info!("Adding feed from OPML: {} ({})", feed_title, url);
                            self.rss_feeds.push(FeedInfo {
                                url,
                                title: feed_title,
                                category,
                            });
                            added += 1;
                        }
                    } else if let Some(cat_name) = title {
                        // It's a category - push to stack
                        category_stack.push(cat_name);
                    }
                }
                Ok(Event::Empty(ref e)) if e.name().as_ref() == b"outline" => {
                    // Self-closing tag - this is a feed
                    let mut xml_url: Option<String> = None;
                    let mut title: Option<String> = None;

                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"xmlUrl" | b"xmlurl" => {
                                xml_url = Some(String::from_utf8_lossy(&attr.value).to_string());
                            }
                            b"text" | b"title" => {
                                if title.is_none() {
                                    title = Some(String::from_utf8_lossy(&attr.value).to_string());
                                }
                            }
                            _ => {}
                        }
                    }

                    if let Some(url) = xml_url {
                        // Skip duplicates
                        if self.rss_feeds.iter().any(|f| f.url == url) {
                            skipped_duplicate += 1;
                            continue;
                        }

                        // Use the title from OPML or use the URL as fallback
                        let feed_title = title
                            .filter(|t| !t.is_empty())
                            .unwrap_or_else(|| url.clone());
                        let category = category_stack.last().cloned();

                        info!("Adding feed from OPML: {} ({})", feed_title, url);
                        self.rss_feeds.push(FeedInfo {
                            url,
                            title: feed_title,
                            category,
                        });
                        added += 1;
                    }
                }
                Ok(Event::End(ref e)) if e.name().as_ref() == b"outline" => {
                    // Exiting an outline - pop category if we have one
                    category_stack.pop();
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    error!("Error parsing OPML: {}", e);
                    return Err(format!("Error parsing OPML: {}", e).into());
                }
                _ => {}
            }
        }

        // Save if we added any feeds
        if added > 0 {
            self.save_feeds()?;
        }

        // Build result message
        let mut result_parts = Vec::new();
        if added > 0 {
            result_parts.push(format!("{} added", added));
        }
        if skipped_duplicate > 0 {
            result_parts.push(format!("{} duplicate", skipped_duplicate));
        }

        let result_msg = if result_parts.is_empty() {
            "OPML Import: No feeds found".to_string()
        } else {
            format!("OPML Import: {}", result_parts.join(", "))
        };
        self.import_result = Some(result_msg.clone());
        self.error_message = Some(result_msg);

        Ok(())
    }

    /// Gets the default OPML file path for import
    pub fn get_opml_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("reedy");
        path.push("feeds.opml");
        path
    }

    /// Starts category setting mode for the currently selected feed
    pub fn start_setting_category(&mut self) {
        if self.selected_index.is_some() && !self.rss_feeds.is_empty() {
            self.input_mode = InputMode::SettingCategory;
            // Pre-fill with current category if it exists
            if let Some(index) = self.selected_index {
                if let Some(feed) = self.rss_feeds.get(index) {
                    self.input_buffer = feed.category.clone().unwrap_or_default();
                }
            }
        }
    }

    /// Cancels category setting mode
    pub fn cancel_setting_category(&mut self) {
        self.input_mode = InputMode::Normal;
        self.input_buffer.clear();
        self.clear_error();
    }

    /// Sets the category for the currently selected feed
    pub fn set_category(&mut self) {
        if let Some(index) = self.selected_index {
            if let Some(feed) = self.rss_feeds.get_mut(index) {
                let category = self.input_buffer.trim().to_string();
                if category.is_empty() {
                    feed.category = None;
                    info!("Cleared category for feed: {}", feed.title);
                } else {
                    feed.category = Some(category.clone());
                    info!("Set category '{}' for feed: {}", category, feed.title);
                }
                if let Err(e) = self.save_feeds() {
                    error!("Failed to save feeds after setting category: {}", e);
                    self.error_message = Some("Failed to save category".to_string());
                }
            }
        }
        self.input_mode = InputMode::Normal;
        self.input_buffer.clear();
    }

    /// Returns a sorted list of unique categories used by feeds
    pub fn get_categories(&self) -> Vec<String> {
        let mut categories: Vec<String> = self
            .rss_feeds
            .iter()
            .filter_map(|f| f.category.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        categories.sort();
        categories
    }

    /// Returns feeds grouped by category. Uncategorized feeds are grouped under None.
    pub fn get_feeds_by_category(&self) -> Vec<(Option<String>, Vec<&FeedInfo>)> {
        use std::collections::BTreeMap;

        let mut grouped: BTreeMap<Option<String>, Vec<&FeedInfo>> = BTreeMap::new();

        for feed in &self.rss_feeds {
            grouped.entry(feed.category.clone()).or_default().push(feed);
        }

        // Convert to Vec and sort: None (uncategorized) first, then alphabetically by category
        let mut result: Vec<(Option<String>, Vec<&FeedInfo>)> = grouped.into_iter().collect();
        result.sort_by(|a, b| match (&a.0, &b.0) {
            (None, None) => std::cmp::Ordering::Equal,
            (None, Some(_)) => std::cmp::Ordering::Less,
            (Some(_), None) => std::cmp::Ordering::Greater,
            (Some(a), Some(b)) => a.cmp(b),
        });
        result
    }

    /// Imports feeds from the input buffer (one URL per line)
    pub async fn import_feeds(&mut self) -> AppResult<()> {
        let urls: Vec<String> = self
            .input_buffer
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if urls.is_empty() {
            self.error_message = Some("No URLs to import".to_string());
            return Ok(());
        }

        let mut added = 0;
        let mut skipped_duplicate = 0;
        let mut skipped_invalid = 0;

        for url in urls {
            // Skip duplicates
            if self.rss_feeds.iter().any(|f| f.url == url) {
                skipped_duplicate += 1;
                continue;
            }

            // Validate the URL and get title
            match Self::validate_and_get_feed_title(&url, self.config.http_timeout_secs).await {
                Ok(Some(title)) => {
                    info!("Successfully validated and added feed: {} ({})", title, url);
                    self.rss_feeds.push(FeedInfo {
                        url,
                        title,
                        category: None,
                    });
                    added += 1;
                }
                Ok(None) => {
                    debug!("Invalid RSS feed URL during import: {}", url);
                    skipped_invalid += 1;
                }
                Err(e) => {
                    debug!("Error validating feed during import: {} - {}", url, e);
                    skipped_invalid += 1;
                }
            }
        }

        // Save if we added any feeds
        if added > 0 {
            self.save_feeds()?;
        }

        // Build result message
        let mut result_parts = Vec::new();
        if added > 0 {
            result_parts.push(format!("{} added", added));
        }
        if skipped_duplicate > 0 {
            result_parts.push(format!("{} duplicate", skipped_duplicate));
        }
        if skipped_invalid > 0 {
            result_parts.push(format!("{} invalid", skipped_invalid));
        }

        let result_msg = format!("Import: {}", result_parts.join(", "));
        self.import_result = Some(result_msg.clone());
        self.error_message = Some(result_msg);

        self.input_mode = InputMode::Normal;
        self.input_buffer.clear();

        Ok(())
    }

    /// Validates a URL as a valid RSS/Atom feed and returns its title if valid.
    /// Returns Ok(Some(title)) if valid, Ok(None) if invalid, or Err on failure.
    pub async fn validate_and_get_feed_title(
        url: &str,
        timeout_secs: u64,
    ) -> AppResult<Option<String>> {
        // First validate URL format
        let url = reqwest::Url::parse(url)?;
        if url.scheme() != "http" && url.scheme() != "https" {
            return Ok(None);
        }

        // Try to fetch and parse the feed
        let client = create_http_client(timeout_secs);
        match client.get(url.as_str()).send().await {
            Ok(response) => {
                // Check for HTTP errors before trying to parse
                if !response.status().is_success() {
                    error!(
                        "HTTP error {} when fetching feed: {}",
                        response.status(),
                        url
                    );
                    return Ok(None);
                }

                let bytes = response.bytes().await?;
                // Try RSS first
                if let Ok(channel) = Channel::read_from(&bytes[..]) {
                    let title = channel.title().to_string();
                    return Ok(Some(if title.is_empty() {
                        url.to_string()
                    } else {
                        title
                    }));
                }
                // Try Atom if RSS fails
                if let Ok(feed) = AtomFeed::read_from(&bytes[..]) {
                    let title = feed.title().value.clone();
                    return Ok(Some(if title.is_empty() {
                        url.to_string()
                    } else {
                        title
                    }));
                }
                Ok(None)
            }
            Err(_) => Ok(None),
        }
    }

    pub async fn add_feed(&mut self) -> AppResult<()> {
        debug!("Attempting to add feed: {}", self.input_buffer);
        match Self::validate_and_get_feed_title(&self.input_buffer, self.config.http_timeout_secs)
            .await
        {
            Ok(Some(title)) => {
                info!(
                    "Successfully validated feed: {} ({})",
                    title, self.input_buffer
                );
                self.rss_feeds.push(FeedInfo {
                    url: self.input_buffer.clone(),
                    title,
                    category: None,
                });
                self.save_feeds()?;
                self.input_buffer.clear();
                self.input_mode = InputMode::Normal;
                Ok(())
            }
            Ok(None) => {
                error!("Invalid RSS feed URL: {}", self.input_buffer);
                self.error_message = Some("Invalid RSS feed URL".to_string());
                Ok(())
            }
            Err(e) => {
                error!("Error validating feed: {}", e);
                self.error_message = Some(format!("Error: {}", e));
                Ok(())
            }
        }
    }

    pub async fn select_feed(&mut self, index: usize) -> AppResult<()> {
        if index < self.rss_feeds.len() {
            debug!("Loading feed content from index {}", index);
            self.selected_index = Some(index);
            self.load_feed_content().await?;
        }
        Ok(())
    }

    pub async fn load_feed_content(&mut self) -> AppResult<()> {
        if let Some(index) = self.selected_index {
            if let Some(feed_info) = self.rss_feeds.get(index) {
                let url = &feed_info.url;
                let feed_title = &feed_info.title;
                debug!("Checking cache for URL: {}", url);

                // Try to load from cache first
                if let Some(cached_content) = self.load_feed_cache(url) {
                    debug!("Using cached content for {}", url);
                    self.current_feed_content = cached_content;
                    return Ok(());
                }

                debug!("Fetching feed content from URL: {}", url);
                let client = create_http_client(self.config.http_timeout_secs);
                let response = client.get(url.as_str()).send().await?;

                // Check for HTTP errors
                if !response.status().is_success() {
                    self.error_message = Some(format!(
                        "HTTP error {}: {}",
                        response.status(),
                        response
                            .status()
                            .canonical_reason()
                            .unwrap_or("Unknown error")
                    ));
                    return Ok(());
                }

                let content = response.bytes().await?;

                let feed_title_clone = feed_title.clone();
                let mut feed_items: Vec<FeedItem> = match Channel::read_from(&content[..]) {
                    Ok(channel) => {
                        // Handle RSS feed
                        channel
                            .items()
                            .iter()
                            .map(|item| {
                                let description = item
                                    .description()
                                    .unwrap_or("No description")
                                    .replace(|c| ['\n', '\r'].contains(&c), " ");
                                let clean_description =
                                    html2text::from_read(description.as_bytes(), 80);

                                let published = item.pub_date().and_then(|date| {
                                    DateTime::parse_from_rfc2822(date).ok().map(|dt| dt.into())
                                });

                                FeedItem {
                                    title: format!(
                                        "{} | {}",
                                        item.title().unwrap_or("No title"),
                                        feed_title_clone
                                    ),
                                    description: clean_description,
                                    link: item.link().unwrap_or("").to_string(),
                                    published,
                                    id: Self::create_item_id(
                                        item.title().unwrap_or("No title"),
                                        published,
                                        url,
                                    ),
                                    feed_url: url.clone(),
                                }
                            })
                            .collect()
                    }
                    Err(_) => {
                        // Try parsing as Atom feed
                        match AtomFeed::read_from(&content[..]) {
                            Ok(feed) => feed
                                .entries()
                                .iter()
                                .map(|entry| {
                                    let description = entry
                                        .content()
                                        .and_then(|c| c.value.clone())
                                        .or_else(|| entry.summary().map(|s| s.value.clone()))
                                        .unwrap_or_else(|| "No description".to_string());
                                    let clean_description =
                                        html2text::from_read(description.as_bytes(), 80);

                                    let published = entry
                                        .published()
                                        .or_else(|| Some(entry.updated()))
                                        .map(|date| date.to_owned().into());

                                    FeedItem {
                                        title: format!(
                                            "{} | {}",
                                            entry.title().value,
                                            feed_title_clone
                                        ),
                                        description: clean_description,
                                        link: entry
                                            .links()
                                            .first()
                                            .map(|l| l.href().to_string())
                                            .unwrap_or_default(),
                                        published,
                                        id: Self::create_item_id(
                                            &entry.title().value,
                                            published,
                                            url,
                                        ),
                                        feed_url: url.clone(),
                                    }
                                })
                                .collect(),
                            Err(e) => {
                                error!("Failed to parse feed as either RSS or Atom: {}", e);
                                return Err(Box::new(e));
                            }
                        }
                    }
                };

                // Sort by date, newest first
                feed_items.sort_by(|a, b| b.published.cmp(&a.published));

                // Save to cache
                if let Err(e) = self.save_feed_cache(url, &feed_items) {
                    error!("Failed to cache feed content: {}", e);
                }

                self.current_feed_content = feed_items;
                Ok(())
            } else {
                debug!("No feed URL found at index {}", index);
                Ok(())
            }
        } else {
            debug!("No feed selected");
            Ok(())
        }
    }

    fn save_feeds(&self) -> AppResult<()> {
        let saved = SavedState {
            feeds: self.rss_feeds.clone(),
            read_items: self.read_items.clone(),
            favorites: self.favorites.clone(),
        };
        let content = serde_json::to_string_pretty(&saved)?;
        fs::write(&self.save_path, content)?;
        debug!(
            "Saved {} feeds, {} read items, and {} favorites to {}",
            self.rss_feeds.len(),
            self.read_items.len(),
            self.favorites.len(),
            self.save_path.display()
        );
        Ok(())
    }

    pub fn open_selected_feed(&self) {
        if let Some(visible_index) = self.selected_index {
            if let Some(actual_index) = self.get_actual_index(visible_index) {
                if let Some(item) = self.current_feed_content.get(actual_index) {
                    if !item.link.is_empty() {
                        let _ = open::that(&item.link);
                    }
                }
            }
        }
    }

    /// Copies the selected item's link to the clipboard using OSC 52
    pub fn copy_selected_link(&mut self) {
        if let Some(visible_index) = self.selected_index {
            if let Some(actual_index) = self.get_actual_index(visible_index) {
                if let Some(item) = self.current_feed_content.get(actual_index) {
                    if !item.link.is_empty() {
                        match copy_to_clipboard_osc52(&item.link) {
                            Ok(()) => {
                                self.status_message = Some("Link copied!".to_string());
                                debug!("Copied link to clipboard: {}", item.link);
                            }
                            Err(e) => {
                                error!("Failed to copy to clipboard: {}", e);
                                self.error_message = Some(format!("Failed to copy: {}", e));
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    /// Clears both status and error messages
    pub fn clear_messages(&mut self) {
        self.status_message = None;
        self.error_message = None;
    }

    pub fn scroll_up(&mut self) {
        if self.scroll > 0 {
            self.scroll = self.scroll.saturating_sub(1);
        }
    }

    pub fn scroll_down(&mut self) {
        let max_scroll = match self.page_mode {
            PageMode::FeedList | PageMode::Favorites => {
                if self.current_feed_content.is_empty() {
                    0
                } else {
                    self.current_feed_content.len().saturating_sub(1)
                }
            }
            PageMode::FeedManager => {
                if self.rss_feeds.is_empty() {
                    0
                } else {
                    self.rss_feeds.len().saturating_sub(1)
                }
            }
        };

        if (self.scroll as usize) < max_scroll {
            self.scroll += 1;
        }
    }

    pub fn page_up(&mut self) {
        let page_size = self.items_per_page() as u16;

        // Scroll up by page size
        self.scroll = self.scroll.saturating_sub(page_size);

        // Update selection to follow scrolling
        if let Some(index) = self.selected_index {
            if (index as u16) >= self.scroll + page_size {
                self.selected_index = Some(self.scroll as usize);
            }
        }
    }

    pub fn page_down(&mut self) {
        let page_size = self.items_per_page();

        // Get the appropriate list length based on page mode
        let list_len = match self.page_mode {
            PageMode::FeedList | PageMode::Favorites => self.current_feed_content.len(),
            PageMode::FeedManager => self.rss_feeds.len(),
        };

        // Calculate maximum possible scroll value
        let max_scroll = if list_len == 0 {
            0
        } else {
            list_len
                .saturating_sub(1)
                .saturating_sub(page_size.saturating_sub(1))
        };

        // Calculate new scroll position, capped at maximum scroll
        let new_scroll = (self.scroll as usize + page_size).min(max_scroll);

        self.scroll = new_scroll as u16;

        // If the selected index is now above the visible area, update it
        if let Some(index) = self.selected_index {
            if index < new_scroll {
                self.selected_index = Some(new_scroll);
            }
        }
    }

    /// Scrolls to the top of the feed and selects the first item
    pub fn scroll_to_top(&mut self) {
        self.scroll = 0;

        // Select the first item if there are any items
        match self.page_mode {
            PageMode::FeedList | PageMode::Favorites => {
                if !self.current_feed_content.is_empty() {
                    self.selected_index = Some(0);
                }
            }
            PageMode::FeedManager => {
                if !self.rss_feeds.is_empty() {
                    self.selected_index = Some(0);
                }
            }
        }
    }

    /// Scrolls to the bottom of the feed and selects the last item
    pub fn scroll_to_bottom(&mut self) {
        let len = match self.page_mode {
            PageMode::FeedList | PageMode::Favorites => self.visible_item_count(),
            PageMode::FeedManager => self.rss_feeds.len(),
        };

        if len == 0 {
            return;
        }

        // Select the last item
        self.selected_index = Some(len - 1);

        // Ensure the selection is visible by scrolling to show it
        self.ensure_selection_visible();
    }

    /// Opens the article preview pane for the currently selected item
    pub fn open_preview(&mut self) {
        if let Some(visible_index) = self.selected_index {
            if self.get_actual_index(visible_index).is_some() {
                self.input_mode = InputMode::Preview;
                self.preview_scroll = 0;
            }
        }
    }

    /// Closes the article preview pane
    pub fn close_preview(&mut self) {
        self.input_mode = InputMode::Normal;
        self.preview_scroll = 0;
    }

    /// Scrolls the preview pane up by one line
    pub fn preview_scroll_up(&mut self) {
        self.preview_scroll = self.preview_scroll.saturating_sub(1);
    }

    /// Scrolls the preview pane down by one line
    pub fn preview_scroll_down(&mut self) {
        self.preview_scroll = self.preview_scroll.saturating_add(1);
    }

    /// Scrolls the preview pane up by a page
    pub fn preview_page_up(&mut self) {
        let page_size = self.terminal_height.saturating_sub(10);
        self.preview_scroll = self.preview_scroll.saturating_sub(page_size);
    }

    /// Scrolls the preview pane down by a page
    pub fn preview_page_down(&mut self) {
        let page_size = self.terminal_height.saturating_sub(10);
        self.preview_scroll = self.preview_scroll.saturating_add(page_size);
    }

    /// Gets the currently selected feed item for preview
    pub fn get_preview_item(&self) -> Option<&FeedItem> {
        if let Some(visible_index) = self.selected_index {
            if let Some(actual_index) = self.get_actual_index(visible_index) {
                return self.current_feed_content.get(actual_index);
            }
        }
        None
    }

    /// Formats a feed item as markdown for export
    fn format_article_markdown(&self, item: &FeedItem) -> String {
        let mut output = String::new();

        // Title
        output.push_str(&format!("# {}\n\n", item.title));

        // Metadata
        if let Some(published) = item.published {
            if let Ok(duration) = published.duration_since(SystemTime::UNIX_EPOCH) {
                let secs = duration.as_secs() as i64;
                if let Some(dt) = chrono::DateTime::from_timestamp(secs, 0) {
                    output.push_str(&format!("**Date:** {}\n\n", dt.format("%Y-%m-%d %H:%M")));
                }
            }
        }

        if !item.link.is_empty() {
            output.push_str(&format!("**Link:** {}\n\n", item.link));
        }

        // Status
        let read_status = if self.read_items.contains(&item.id) {
            "Read"
        } else {
            "Unread"
        };
        let fav_status = if self.favorites.contains(&item.id) {
            "★ Favorited"
        } else {
            ""
        };
        if !fav_status.is_empty() {
            output.push_str(&format!("**Status:** {} | {}\n\n", read_status, fav_status));
        } else {
            output.push_str(&format!("**Status:** {}\n\n", read_status));
        }

        output.push_str("---\n\n");

        // Content - convert HTML to plain text
        let plain_text = html2text::from_read(item.description.as_bytes(), 80);
        output.push_str(&plain_text);

        output
    }

    /// Exports the currently selected article to clipboard using OSC 52
    pub fn export_article_to_clipboard(&mut self) {
        let item = if let Some(item) = self.get_preview_item() {
            item.clone()
        } else {
            self.error_message = Some("No article selected".to_string());
            return;
        };

        let content = self.format_article_markdown(&item);

        match copy_to_clipboard_osc52(&content) {
            Ok(()) => {
                info!("Exported article to clipboard: {}", item.title);
                self.status_message = Some("Article copied!".to_string());
            }
            Err(e) => {
                error!("Failed to copy to clipboard: {}", e);
                self.error_message = Some(format!("Failed to copy: {}", e));
            }
        }
    }

    /// Exports the currently selected article to a file
    pub fn export_article_to_file(&mut self) {
        let item = if let Some(item) = self.get_preview_item() {
            item.clone()
        } else {
            self.error_message = Some("No article selected".to_string());
            return;
        };

        let content = self.format_article_markdown(&item);

        // Create a safe filename from the title
        let safe_title: String = item
            .title
            .chars()
            .take(50)
            .map(|c| {
                if c.is_alphanumeric() || c == ' ' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>()
            .trim()
            .replace(' ', "_");

        // Get export directory
        let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("reedy");
        path.push("exports");
        if let Err(e) = fs::create_dir_all(&path) {
            error!("Failed to create export directory: {}", e);
            self.error_message = Some(format!("Failed to create export directory: {}", e));
            return;
        }

        // Add timestamp to filename to avoid collisions
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        path.push(format!("{}_{}.md", safe_title, timestamp));

        match fs::write(&path, content) {
            Ok(()) => {
                info!("Exported article to: {}", path.display());
                self.error_message = Some(format!("Saved to: {}", path.display()));
            }
            Err(e) => {
                error!("Failed to write file: {}", e);
                self.error_message = Some(format!("Failed to save article: {}", e));
            }
        }
    }

    /// Enters vi-style command mode (triggered by ':')
    pub fn start_command_mode(&mut self) {
        self.input_mode = InputMode::Command;
        self.command_buffer.clear();
    }

    /// Cancels command mode without executing
    pub fn cancel_command_mode(&mut self) {
        self.input_mode = InputMode::Normal;
        self.command_buffer.clear();
    }

    /// Executes the current command buffer and returns to normal mode.
    /// Returns Ok(true) if the command was executed successfully,
    /// Ok(false) if the command was not recognized,
    /// or an error if execution failed.
    pub fn execute_command(&mut self) -> AppResult<bool> {
        let command = self.command_buffer.trim().to_lowercase();
        self.input_mode = InputMode::Normal;
        self.command_buffer.clear();

        match command.as_str() {
            // Quit commands
            "q" | "quit" => {
                self.quit();
                Ok(true)
            }
            // Write/save commands
            "w" | "write" | "save" => {
                self.save_state()?;
                self.error_message = Some("State saved".to_string());
                Ok(true)
            }
            // Write and quit
            "wq" | "x" => {
                self.save_state()?;
                self.quit();
                Ok(true)
            }
            // Force quit (without save - but we always save state anyway)
            "q!" => {
                self.quit();
                Ok(true)
            }
            // Refresh feeds
            "refresh" | "r" => {
                // Set a flag to indicate refresh is needed (actual refresh is async)
                self.auto_refresh_pending = true;
                Ok(true)
            }
            // Help
            "help" | "h" => {
                self.toggle_help();
                Ok(true)
            }
            // Open feed manager
            "feeds" | "manage" => {
                self.toggle_feed_manager();
                Ok(true)
            }
            // Toggle favorites view
            "favorites" | "fav" => {
                // Return false to indicate async action needed
                // The handler will call toggle_favorites_page().await
                self.error_message = Some("__toggle_favorites__".to_string());
                Ok(true)
            }
            // Mark all as read
            "read" | "markread" => {
                self.mark_all_as_read();
                Ok(true)
            }
            // Scroll to top
            "0" | "top" | "gg" => {
                self.scroll_to_top();
                Ok(true)
            }
            // Scroll to bottom
            "$" | "bottom" => {
                self.scroll_to_bottom();
                Ok(true)
            }
            // Empty command - just cancel
            "" => Ok(true),
            // Unknown command
            _ => {
                self.error_message = Some(format!("Unknown command: {}", command));
                Ok(false)
            }
        }
    }

    /// Starts search mode
    pub fn start_search(&mut self) {
        self.input_mode = InputMode::Searching;
        self.search_query.clear();
        self.filtered_indices = None;
    }

    /// Cancels search mode and clears the filter
    pub fn cancel_search(&mut self) {
        self.input_mode = InputMode::Normal;
        self.search_query.clear();
        self.filtered_indices = None;
        self.scroll = 0;
        // Reset selection to first item if available
        if !self.current_feed_content.is_empty() {
            self.selected_index = Some(0);
        }
    }

    /// Confirms the search and stays in filtered mode
    pub fn confirm_search(&mut self) {
        self.input_mode = InputMode::Normal;
        // Keep the filter active, selection remains on current filtered item
    }

    /// Updates the search filter based on the current query
    pub fn update_search_filter(&mut self) {
        self.apply_filters();
    }

    /// Applies all active filters (search query and unread-only)
    fn apply_filters(&mut self) {
        let has_search = !self.search_query.is_empty();
        let has_unread_filter = self.show_unread_only;

        // If no filters active, clear filtered_indices
        if !has_search && !has_unread_filter {
            self.filtered_indices = None;
            self.scroll = 0;
            if !self.current_feed_content.is_empty() {
                self.selected_index = Some(0);
            }
            return;
        }

        let query_lower = self.search_query.to_lowercase();
        let filtered: Vec<usize> = self
            .current_feed_content
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                // Apply search filter if active
                let matches_search = !has_search
                    || item.title.to_lowercase().contains(&query_lower)
                    || item.description.to_lowercase().contains(&query_lower);

                // Apply unread filter if active
                let matches_unread = !has_unread_filter || !self.read_items.contains(&item.id);

                matches_search && matches_unread
            })
            .map(|(i, _)| i)
            .collect();

        self.scroll = 0;
        if filtered.is_empty() {
            self.selected_index = None;
        } else {
            self.selected_index = Some(0);
        }
        self.filtered_indices = Some(filtered);
    }

    /// Toggles the unread-only filter
    pub fn toggle_unread_only(&mut self) {
        self.show_unread_only = !self.show_unread_only;
        self.apply_filters();
        debug!(
            "Toggled unread-only filter: {}",
            if self.show_unread_only { "ON" } else { "OFF" }
        );
    }

    /// Clears all filters (search and unread-only) when pressing Esc
    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.show_unread_only = false;
        self.apply_filters();
    }

    /// Returns the items to display based on the current filter
    pub fn get_visible_items(&self) -> Vec<(usize, &FeedItem)> {
        match &self.filtered_indices {
            Some(indices) => indices
                .iter()
                .map(|&i| (i, &self.current_feed_content[i]))
                .collect(),
            None => self.current_feed_content.iter().enumerate().collect(),
        }
    }

    /// Returns the number of visible items (filtered or all)
    pub fn visible_item_count(&self) -> usize {
        match &self.filtered_indices {
            Some(indices) => indices.len(),
            None => self.current_feed_content.len(),
        }
    }

    /// Gets the actual index in current_feed_content for a visible index
    pub fn get_actual_index(&self, visible_index: usize) -> Option<usize> {
        match &self.filtered_indices {
            Some(indices) => indices.get(visible_index).copied(),
            None => {
                if visible_index < self.current_feed_content.len() {
                    Some(visible_index)
                } else {
                    None
                }
            }
        }
    }

    fn get_cache_dir() -> PathBuf {
        let mut path = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("reedy");
        path.push("feed_cache");
        if let Err(e) = fs::create_dir_all(&path) {
            error!("Failed to create cache directory {:?}: {}", path, e);
        }
        path
    }

    fn get_cache_path(url: &str) -> PathBuf {
        let mut path = Self::get_cache_dir();
        // Create a filename from the URL (sanitized)
        let filename = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, url);
        path.push(filename);
        path.set_extension("json");
        path
    }

    fn save_feed_cache(&self, url: &str, content: &[FeedItem]) -> AppResult<()> {
        let cache = CachedFeed {
            url: url.to_string(),
            content: content.to_vec(),
            last_updated: SystemTime::now(),
        };
        let cache_path = Self::get_cache_path(url);
        let content = serde_json::to_string_pretty(&cache)?;
        fs::write(cache_path, content)?;
        Ok(())
    }

    fn load_feed_cache(&self, url: &str) -> Option<Vec<FeedItem>> {
        let cache_path = Self::get_cache_path(url);
        if let Ok(content) = fs::read_to_string(&cache_path) {
            match serde_json::from_str::<CachedFeed>(&content) {
                Ok(cache) => {
                    // Check if cache is within the configured duration
                    let cache_duration_secs = self.config.cache_duration_mins * 60;
                    if let Ok(duration) = cache.last_updated.elapsed() {
                        if duration.as_secs() < cache_duration_secs {
                            return Some(cache.content);
                        }
                    }
                }
                Err(e) => {
                    // Cache file is corrupted, delete it
                    error!(
                        "Failed to parse cache file for {}: {}. Removing corrupted cache.",
                        url, e
                    );
                    match fs::remove_file(&cache_path) {
                        Ok(_) => info!("Removed corrupted cache file for {}", url),
                        Err(e) => error!("Failed to remove corrupted cache file: {}", e),
                    }
                }
            }
        }
        None
    }

    /// Caches content from all configured RSS/Atom feeds.
    ///
    /// This method iterates through all feed URLs and:
    /// - Checks if a valid cache already exists for each feed
    /// - Skips feeds that are already cached
    /// - Fetches and parses new content for uncached feeds
    /// - Attempts to parse feeds as both RSS and Atom formats
    /// - Stores the parsed content in the local cache
    ///
    /// The cached content includes feed items with their titles, descriptions,
    /// links, and publication dates. Cache entries are stored in the application's
    /// cache directory with base64-encoded URLs as filenames.
    ///
    /// # Errors
    ///
    /// While this method doesn't return errors, it logs error messages when:
    /// - Network requests fail
    /// - Feed parsing fails
    /// - Cache operations fail
    pub async fn cache_all_feeds(&mut self) {
        for feed_info in self.rss_feeds.clone() {
            debug!("Checking cache for URL: {}", feed_info.url);

            // Skip if already cached
            if self.load_feed_cache(&feed_info.url).is_some() {
                debug!("Using existing cache for {}", feed_info.url);
                continue;
            }

            debug!("Fetching feed content from URL: {}", feed_info.url);
            let client = create_http_client(self.config.http_timeout_secs);
            match client.get(&feed_info.url).send().await {
                Ok(response) => {
                    // Check for HTTP errors
                    if !response.status().is_success() {
                        error!(
                            "HTTP error {} when fetching {}: {}",
                            response.status(),
                            feed_info.url,
                            response.status().canonical_reason().unwrap_or("Unknown")
                        );
                        continue;
                    }

                    if let Ok(content) = response.bytes().await {
                        // Try RSS first
                        let feed_items = match Channel::read_from(&content[..]) {
                            Ok(channel) => {
                                convert_rss_items(channel, &feed_info.title, &feed_info.url)
                            }
                            Err(_) => {
                                // Try Atom if RSS fails
                                match AtomFeed::read_from(&content[..]) {
                                    Ok(feed) => {
                                        convert_atom_items(feed, &feed_info.title, &feed_info.url)
                                    }
                                    Err(e) => {
                                        error!("Failed to parse feed as either RSS or Atom: {}", e);
                                        continue;
                                    }
                                }
                            }
                        };

                        if let Err(e) = self.save_feed_cache(&feed_info.url, &feed_items) {
                            error!("Failed to cache feed content for {}: {}", feed_info.url, e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to fetch feed {}: {}", feed_info.url, e);
                }
            }
        }
    }

    /// Refreshes all RSS/Atom feeds by fetching their latest content.
    ///
    /// This method:
    /// - Fetches the latest content from all configured feed URLs
    /// - Parses both RSS and Atom feed formats
    /// - Caches the fetched content for each feed
    /// - Combines all feed items into a single sorted list
    /// - Updates the application's current feed content
    /// - Tracks feed health status (healthy, slow, broken)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Network requests fail
    /// - Feed parsing fails
    /// - Cache operations fail
    pub async fn refresh_all_feeds(&mut self) -> AppResult<()> {
        self.refresh_all_feeds_impl(false).await
    }

    pub async fn force_refresh_all_feeds(&mut self) -> AppResult<()> {
        self.refresh_all_feeds_impl(true).await
    }

    async fn refresh_all_feeds_impl(&mut self, force: bool) -> AppResult<()> {
        use std::time::Instant;

        let mut all_items = Vec::new();

        let client = create_http_client(self.config.http_timeout_secs);

        // Clone feed info to avoid borrow issues
        let feeds: Vec<FeedInfo> = self.rss_feeds.clone();

        for feed_info in feeds {
            // Check cache first unless forcing refresh
            if !force {
                if let Some(cached_items) = self.load_feed_cache(&feed_info.url) {
                    debug!("Using cached content for {}", feed_info.url);
                    all_items.extend(cached_items);
                    continue;
                }
            }

            debug!("Fetching feed: {}", feed_info.url);

            // Record start time for health tracking
            let start_time = Instant::now();

            match client.get(&feed_info.url).send().await {
                Ok(response) => {
                    let response_time_ms = start_time.elapsed().as_millis() as u64;

                    // Check for HTTP errors
                    if !response.status().is_success() {
                        error!(
                            "HTTP error {} when refreshing {}: {}",
                            response.status(),
                            feed_info.url,
                            response.status().canonical_reason().unwrap_or("Unknown")
                        );
                        let existing = self.feed_health.get(&feed_info.url);
                        let consecutive = existing.map(|h| h.consecutive_failures + 1).unwrap_or(1);
                        self.feed_health.insert(
                            feed_info.url.clone(),
                            FeedHealth {
                                status: FeedStatus::Broken,
                                last_success: existing.and_then(|h| h.last_success),
                                last_error: Some(format!("HTTP {}", response.status())),
                                last_response_time_ms: Some(response_time_ms),
                                consecutive_failures: consecutive,
                            },
                        );
                        continue;
                    }

                    match response.bytes().await {
                        Ok(content) => {
                            // Try RSS first
                            let parse_result = match Channel::read_from(&content[..]) {
                                Ok(channel) => Some(convert_rss_items(
                                    channel,
                                    &feed_info.title,
                                    &feed_info.url,
                                )),
                                Err(_) => {
                                    // Try Atom if RSS fails
                                    match AtomFeed::read_from(&content[..]) {
                                        Ok(feed) => Some(convert_atom_items(
                                            feed,
                                            &feed_info.title,
                                            &feed_info.url,
                                        )),
                                        Err(_) => None,
                                    }
                                }
                            };

                            match parse_result {
                                Some(feed_items) => {
                                    // Save to cache
                                    if let Err(e) =
                                        self.save_feed_cache(&feed_info.url, &feed_items)
                                    {
                                        error!(
                                            "Failed to cache feed content for {}: {}",
                                            feed_info.url, e
                                        );
                                    }
                                    all_items.extend(feed_items);

                                    // Update health status - slow if > 5000ms, healthy otherwise
                                    let status = if response_time_ms > 5000 {
                                        FeedStatus::Slow
                                    } else {
                                        FeedStatus::Healthy
                                    };

                                    self.feed_health.insert(
                                        feed_info.url.clone(),
                                        FeedHealth {
                                            status,
                                            last_success: Some(SystemTime::now()),
                                            last_response_time_ms: Some(response_time_ms),
                                            last_error: None,
                                            consecutive_failures: 0,
                                        },
                                    );
                                }
                                None => {
                                    error!(
                                        "Failed to parse feed as either RSS or Atom: {}",
                                        feed_info.url
                                    );

                                    // Update health status as broken (parse error)
                                    let health =
                                        self.feed_health.entry(feed_info.url.clone()).or_default();
                                    health.status = FeedStatus::Broken;
                                    health.last_error = Some("Failed to parse feed".to_string());
                                    health.last_response_time_ms = Some(response_time_ms);
                                    health.consecutive_failures += 1;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to read response body for {}: {}", feed_info.url, e);

                            // Update health status as broken
                            let health = self.feed_health.entry(feed_info.url.clone()).or_default();
                            health.status = FeedStatus::Broken;
                            health.last_error = Some(format!("Read error: {}", e));
                            health.consecutive_failures += 1;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to fetch feed {}: {}", feed_info.url, e);

                    // Update health status as broken (network error)
                    let health = self.feed_health.entry(feed_info.url.clone()).or_default();
                    health.status = FeedStatus::Broken;
                    health.last_error = Some(format!("{}", e));
                    health.consecutive_failures += 1;
                }
            }
        }

        // Sort all items by date, newest first
        all_items.sort_by(|a, b| b.published.cmp(&a.published));

        // Check for new items and send notifications if enabled
        if self.config.notifications_enabled {
            let new_items: Vec<&FeedItem> = all_items
                .iter()
                .filter(|item| !self.seen_items.contains(&item.id))
                .collect();

            if !new_items.is_empty() {
                self.send_new_articles_notification(&new_items);
            }
        }

        // Update seen items with all current item IDs
        for item in &all_items {
            self.seen_items.insert(item.id.clone());
        }

        // Update the current feed content
        self.current_feed_content = all_items;

        Ok(())
    }

    /// Sends a desktop notification for new articles
    fn send_new_articles_notification(&self, new_items: &[&FeedItem]) {
        use notify_rust::Notification;

        let count = new_items.len();
        let summary = if count == 1 {
            "1 new article".to_string()
        } else {
            format!("{} new articles", count)
        };

        // Build body with up to 3 article titles
        let body: String = new_items
            .iter()
            .take(3)
            .map(|item| format!("• {}", item.title))
            .collect::<Vec<_>>()
            .join("\n");

        let body_with_more = if count > 3 {
            format!("{}\n...and {} more", body, count - 3)
        } else {
            body
        };

        if let Err(e) = Notification::new()
            .summary(&summary)
            .body(&body_with_more)
            .appname("Reedy")
            .timeout(5000)
            .show()
        {
            error!("Failed to send notification: {}", e);
        } else {
            info!("Sent notification for {} new article(s)", count);
        }
    }

    pub fn mark_as_read(&mut self) {
        if let Some(visible_index) = self.selected_index {
            if let Some(actual_index) = self.get_actual_index(visible_index) {
                if let Some(item) = self.current_feed_content.get(actual_index) {
                    if !self.read_items.contains(&item.id) {
                        self.read_items.insert(item.id.clone());
                        debug!("Marked item as read: {}", item.title);
                        self.save_state().unwrap_or_else(|e| {
                            error!("Failed to save read status: {}", e);
                        });
                    }
                }
            }
        }
    }

    pub fn mark_all_as_read(&mut self) {
        // Get items to mark - either filtered items or all items
        let items_to_mark: Vec<String> = match &self.filtered_indices {
            Some(indices) => indices
                .iter()
                .filter_map(|&i| self.current_feed_content.get(i))
                .filter(|item| !self.read_items.contains(&item.id))
                .map(|item| item.id.clone())
                .collect(),
            None => self
                .current_feed_content
                .iter()
                .filter(|item| !self.read_items.contains(&item.id))
                .map(|item| item.id.clone())
                .collect(),
        };

        for id in items_to_mark {
            self.read_items.insert(id.clone());
            debug!("Marked item as read: {}", id);
        }
        self.save_state().unwrap_or_else(|e| {
            error!("Failed to save read status: {}", e);
        });
    }

    pub fn is_item_favorite(&self, item: &FeedItem) -> bool {
        self.favorites.contains(&item.id)
    }

    pub fn toggle_favorite(&mut self) {
        if let Some(visible_index) = self.selected_index {
            if let Some(actual_index) = self.get_actual_index(visible_index) {
                if let Some(item) = self.current_feed_content.get(actual_index) {
                    let was_favorite = self.favorites.contains(&item.id);
                    if was_favorite {
                        self.favorites.remove(&item.id);
                        debug!("Removed item from favorites: {}", item.title);
                    } else {
                        self.favorites.insert(item.id.clone());
                        debug!("Added item to favorites: {}", item.title);
                    }
                    self.save_state().unwrap_or_else(|e| {
                        error!("Failed to save favorites: {}", e);
                    });

                    // If we're in Favorites view and just unfavorited an item, remove it from the list
                    if was_favorite && self.page_mode == PageMode::Favorites {
                        self.current_feed_content.remove(actual_index);
                        // Rebuild filters to handle all active filter combinations correctly
                        self.apply_filters();
                        // Clamp selected index to valid range
                        let visible_count = self.visible_item_count();
                        if visible_count == 0 {
                            self.selected_index = None;
                        } else if visible_index >= visible_count {
                            self.selected_index = Some(visible_count - 1);
                        }
                    }
                }
            }
        }
    }

    pub async fn toggle_favorites_page(&mut self) {
        match self.page_mode {
            PageMode::Favorites => {
                self.page_mode = PageMode::FeedList;
                // Reset scroll position
                self.scroll = 0;

                // Reload feeds using cache if valid
                let _ = self.refresh_all_feeds().await;
                self.selected_index = if self.current_feed_content.is_empty() {
                    None
                } else {
                    Some(0)
                };
            }
            _ => {
                self.page_mode = PageMode::Favorites;
                // Reset scroll position
                self.scroll = 0;

                // Filter current feed content to show only favorites
                let favorites: Vec<FeedItem> = self
                    .current_feed_content
                    .iter()
                    .filter(|item| self.favorites.contains(&item.id))
                    .cloned()
                    .collect();
                self.current_feed_content = favorites;
                self.selected_index = if self.current_feed_content.is_empty() {
                    None
                } else {
                    Some(0)
                };
            }
        }
    }
}

pub async fn fetch_feed(url: &str, timeout_secs: Option<u64>) -> AppResult<Vec<FeedItem>> {
    debug!("Fetching feed from URL: {}", url);
    let client = create_http_client(timeout_secs.unwrap_or(DEFAULT_HTTP_TIMEOUT_SECS));
    let resp = client.get(url).send().await?;

    // Check for HTTP errors
    if !resp.status().is_success() {
        return Err(format!(
            "HTTP error {}: {}",
            resp.status(),
            resp.status().canonical_reason().unwrap_or("Unknown")
        )
        .into());
    }

    let response = resp.bytes().await?;

    // Try parsing as RSS first
    match Channel::read_from(&response[..]) {
        Ok(channel) => {
            debug!("Successfully parsed RSS feed");
            Ok(convert_rss_items(channel, url, url))
        }
        Err(_) => {
            // Try parsing as Atom
            debug!("RSS parsing failed, attempting Atom format");
            match AtomFeed::read_from(&response[..]) {
                Ok(feed) => {
                    debug!("Successfully parsed Atom feed");
                    Ok(convert_atom_items(feed, url, url))
                }
                Err(e) => {
                    error!("Failed to parse feed as either RSS or Atom: {}", e);
                    Err(Box::new(e))
                }
            }
        }
    }
}

fn convert_rss_items(channel: Channel, feed_title: &str, feed_url: &str) -> Vec<FeedItem> {
    channel
        .items()
        .iter()
        .map(|item| {
            let description = item
                .description()
                .unwrap_or("No description")
                .replace(|c| ['\n', '\r'].contains(&c), " ");
            let clean_description = html2text::from_read(description.as_bytes(), 80);

            let published = item
                .pub_date()
                .and_then(|date| DateTime::parse_from_rfc2822(date).ok().map(|dt| dt.into()));

            FeedItem {
                title: format!("{} | {}", item.title().unwrap_or("No title"), feed_title),
                description: clean_description,
                link: item.link().unwrap_or("").to_string(),
                published,
                id: App::create_item_id(item.title().unwrap_or("No title"), published, feed_url),
                feed_url: feed_url.to_string(),
            }
        })
        .collect()
}

fn convert_atom_items(feed: AtomFeed, feed_title: &str, feed_url: &str) -> Vec<FeedItem> {
    feed.entries()
        .iter()
        .map(|entry| {
            let description = entry
                .content()
                .and_then(|c| c.value.clone())
                .or_else(|| entry.summary().map(|s| s.value.clone()))
                .unwrap_or_else(|| "No description".to_string());
            let clean_description = html2text::from_read(description.as_bytes(), 80);

            let published = entry
                .published()
                .or_else(|| Some(entry.updated()))
                .map(|date| date.to_owned().into());

            FeedItem {
                title: format!("{} | {}", entry.title().value, feed_title),
                description: clean_description,
                link: entry
                    .links()
                    .first()
                    .map(|l| l.href().to_string())
                    .unwrap_or_default(),
                published,
                id: App::create_item_id(&entry.title().value, published, feed_url),
                feed_url: feed_url.to_string(),
            }
        })
        .collect()
}
