use atom_syndication::Feed as AtomFeed;
use base64;
use chrono::DateTime;
use crossterm::terminal;
use html2text;
use log::{debug, error, info};
use reqwest;
use rss::Channel;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, error, fs, path::PathBuf, time::Duration, time::SystemTime};

/// Default HTTP request timeout in seconds
const HTTP_TIMEOUT_SECS: u64 = 30;

/// Creates a reqwest client with a configured timeout to prevent hanging on slow/unresponsive feeds
fn create_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
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
}

#[derive(Debug, Serialize, Deserialize)]
struct SavedState {
    feeds: Vec<String>,
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
    pub rss_feeds: Vec<String>,
    pub selected_index: Option<usize>,
    pub current_feed_content: Vec<FeedItem>,
    pub error_message: Option<String>,
    save_path: PathBuf,
    read_items: HashSet<String>,
    pub favorites: HashSet<String>,
    pub scroll: u16,
    pub terminal_width: u16,
    pub terminal_height: u16,
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
            save_path: Self::get_save_path(),
            read_items: HashSet::new(),
            favorites: HashSet::new(),
            scroll: 0,
            terminal_width: 80,
            terminal_height: 24,
        }
    }
}

impl App {
    pub async fn new() -> Self {
        let mut app = Self::default();

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
            let _ = app.refresh_all_feeds().await;
            app.cache_all_feeds().await;

            // Load and combine all cached feed content
            let mut all_items = Vec::new();
            for url in &app.rss_feeds {
                if let Some(cached_items) = app.load_feed_cache(url) {
                    all_items.extend(cached_items);
                }
            }

            // Sort all items by date, newest first
            all_items.sort_by(|a, b| b.published.cmp(&a.published));
            app.current_feed_content = all_items;
        }
        app
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn get_save_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("reedy");
        fs::create_dir_all(&path).unwrap_or_default();
        path.push("feeds.json");
        path
    }

    pub fn get_log_path() -> PathBuf {
        let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("reedy");
        path.push("logs");
        fs::create_dir_all(&path).unwrap_or_default();
        path.push("reedy.log");
        path
    }

    fn create_item_id(title: &str, published: Option<SystemTime>) -> String {
        if let Some(time) = published {
            format!(
                "{}_{}",
                title
                    .to_lowercase()
                    .replace(|c: char| !c.is_alphanumeric(), "_"),
                time.duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            )
        } else {
            title
                .to_lowercase()
                .replace(|c: char| !c.is_alphanumeric(), "_")
        }
    }

    pub fn is_item_read(&self, item: &FeedItem) -> bool {
        self.read_items.contains(&item.id)
    }

    pub fn toggle_read_status(&mut self) {
        if let Some(index) = self.selected_index {
            if let Some(item) = self.current_feed_content.get(index) {
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
            let content = fs::read_to_string(&self.save_path)?;

            // Try to parse with new format first
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
                    // Try parsing old format (without favorites)
                    #[derive(Debug, Serialize, Deserialize)]
                    struct OldSavedState {
                        feeds: Vec<String>,
                        read_items: HashSet<String>,
                    }

                    if let Ok(old_saved) = serde_json::from_str::<OldSavedState>(&content) {
                        self.rss_feeds = old_saved.feeds;
                        self.read_items = old_saved.read_items;
                        self.favorites = HashSet::new(); // Initialize empty favorites
                        debug!(
                            "Loaded {} feeds from old format state file {}",
                            self.rss_feeds.len(),
                            self.save_path.display()
                        );
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
        if let Some(current) = self.selected_index {
            let len = match self.page_mode {
                PageMode::FeedList | PageMode::Favorites => self.current_feed_content.len(),
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
        if let Some(current) = self.selected_index {
            let len = match self.page_mode {
                PageMode::FeedList | PageMode::Favorites => self.current_feed_content.len(),
                PageMode::FeedManager => self.rss_feeds.len(),
            };
            if len == 0 {
                return;
            }
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

    pub async fn is_valid_rss_feed(url: &str) -> AppResult<bool> {
        // First validate URL format
        let url = reqwest::Url::parse(url)?;
        if url.scheme() != "http" && url.scheme() != "https" {
            return Ok(false);
        }

        // Try to fetch and parse the feed
        let client = create_http_client();
        match client.get(url.as_str()).send().await {
            Ok(response) => {
                let bytes = response.bytes().await?;
                // Try RSS first
                if Channel::read_from(&bytes[..]).is_ok() {
                    return Ok(true);
                }
                // Try Atom if RSS fails
                if AtomFeed::read_from(&bytes[..]).is_ok() {
                    return Ok(true);
                }
                Ok(false)
            }
            Err(_) => Ok(false),
        }
    }

    pub async fn add_feed(&mut self) -> AppResult<()> {
        debug!("Attempting to add feed: {}", self.input_buffer);
        match Self::is_valid_rss_feed(&self.input_buffer).await {
            Ok(true) => {
                info!("Successfully validated feed: {}", self.input_buffer);
                self.rss_feeds.push(self.input_buffer.clone());
                self.save_feeds()?;
                self.input_buffer.clear();
                self.input_mode = InputMode::Normal;
                Ok(())
            }
            Ok(false) => {
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
        debug!("test");
        if index < self.rss_feeds.len() {
            debug!("Loading feed content from index {}", index);
            self.selected_index = Some(index);
            self.load_feed_content().await?;
        }
        Ok(())
    }

    pub async fn load_feed_content(&mut self) -> AppResult<()> {
        if let Some(index) = self.selected_index {
            if let Some(url) = self.rss_feeds.get(index) {
                debug!("Checking cache for URL: {}", url);

                // Try to load from cache first
                if let Some(cached_content) = self.load_feed_cache(url) {
                    debug!("Using cached content for {}", url);
                    self.current_feed_content = cached_content;
                    return Ok(());
                }

                debug!("Fetching feed content from URL: {}", url);
                let client = create_http_client();
                let response = client.get(url).send().await?;
                let content = response.bytes().await?;

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
                                        url
                                    ),
                                    description: clean_description,
                                    link: item.link().unwrap_or("").to_string(),
                                    published,
                                    id: Self::create_item_id(
                                        item.title().unwrap_or("No title"),
                                        published,
                                    ),
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
                                        title: format!("{} | {}", entry.title().value, url),
                                        description: clean_description,
                                        link: entry
                                            .links()
                                            .first()
                                            .map(|l| l.href().to_string())
                                            .unwrap_or_default(),
                                        published,
                                        id: Self::create_item_id(&entry.title().value, published),
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
        if let Some(index) = self.selected_index {
            if let Some(item) = self.current_feed_content.get(index) {
                if !item.link.is_empty() {
                    let _ = open::that(&item.link);
                }
            }
        }
    }

    pub fn clear_error(&mut self) {
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
            list_len.saturating_sub(1).saturating_sub(page_size.saturating_sub(1))
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

    fn get_cache_dir() -> PathBuf {
        let mut path = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("reedy");
        path.push("feed_cache");
        fs::create_dir_all(&path).unwrap_or_default();
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
        if let Ok(content) = fs::read_to_string(cache_path) {
            if let Ok(cache) = serde_json::from_str::<CachedFeed>(&content) {
                // Check if cache is less than 1 hour old
                if let Ok(duration) = cache.last_updated.elapsed() {
                    if duration.as_secs() < 3600 {
                        return Some(cache.content);
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
        for url in self.rss_feeds.clone() {
            debug!("Checking cache for URL: {}", url);

            // Skip if already cached
            if self.load_feed_cache(&url).is_some() {
                debug!("Using existing cache for {}", url);
                continue;
            }

            debug!("Fetching feed content from URL: {}", url);
            let client = create_http_client();
            match client.get(&url).send().await {
                Ok(response) => {
                    if let Ok(content) = response.bytes().await {
                        // Try RSS first
                        let feed_items = match Channel::read_from(&content[..]) {
                            Ok(channel) => convert_rss_items(channel, &url),
                            Err(_) => {
                                // Try Atom if RSS fails
                                match AtomFeed::read_from(&content[..]) {
                                    Ok(feed) => convert_atom_items(feed, &url),
                                    Err(e) => {
                                        error!("Failed to parse feed as either RSS or Atom: {}", e);
                                        continue;
                                    }
                                }
                            }
                        };

                        if let Err(e) = self.save_feed_cache(&url, &feed_items) {
                            error!("Failed to cache feed content for {}: {}", url, e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to fetch feed {}: {}", url, e);
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
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Network requests fail
    /// - Feed parsing fails
    /// - Cache operations fail
    pub async fn refresh_all_feeds(&mut self) -> AppResult<()> {
        let mut all_items = Vec::new();

        let client = create_http_client();
        for url in &self.rss_feeds {
            debug!("Refreshing feed: {}", url);
            match client.get(url).send().await {
                Ok(response) => {
                    let content = response.bytes().await?;
                    // Try RSS first
                    let feed_items = match Channel::read_from(&content[..]) {
                        Ok(channel) => convert_rss_items(channel, url),
                        Err(_) => {
                            // Try Atom if RSS fails
                            match AtomFeed::read_from(&content[..]) {
                                Ok(feed) => convert_atom_items(feed, url),
                                Err(_e) => {
                                    error!("Failed to parse feed as either RSS or Atom: {}", url);
                                    continue;
                                }
                            }
                        }
                    };
                    // Save to cache
                    if let Err(e) = self.save_feed_cache(url, &feed_items) {
                        error!("Failed to cache feed content for {}: {}", url, e);
                    }
                    all_items.extend(feed_items);
                }
                Err(e) => {
                    error!("Failed to fetch feed {}: {}", url, e);
                }
            }
        }

        // Sort all items by date, newest first
        all_items.sort_by(|a, b| b.published.cmp(&a.published));

        // Update the current feed content
        self.current_feed_content = all_items;

        Ok(())
    }

    pub fn mark_as_read(&mut self) {
        if let Some(index) = self.selected_index {
            if let Some(item) = self.current_feed_content.get(index) {
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

    pub fn mark_all_as_read(&mut self) {
        for item in &self.current_feed_content {
            if !self.read_items.contains(&item.id) {
                self.read_items.insert(item.id.clone());
                debug!("Marked item as read: {}", item.title);
            }
        }
        self.save_state().unwrap_or_else(|e| {
            error!("Failed to save read status: {}", e);
        });
    }

    pub fn is_item_favorite(&self, item: &FeedItem) -> bool {
        self.favorites.contains(&item.id)
    }

    pub fn toggle_favorite(&mut self) {
        if let Some(index) = self.selected_index {
            if let Some(item) = self.current_feed_content.get(index) {
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
                    self.current_feed_content.remove(index);
                    // Adjust selected index
                    if self.current_feed_content.is_empty() {
                        self.selected_index = None;
                    } else if index >= self.current_feed_content.len() {
                        self.selected_index = Some(self.current_feed_content.len() - 1);
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

                // Reset selection and reload all feeds like at startup
                let _ = self.refresh_all_feeds().await;
                self.cache_all_feeds().await;

                // Load and combine all cached feed content
                let mut all_items = Vec::new();
                for url in &self.rss_feeds {
                    if let Some(cached_items) = self.load_feed_cache(url) {
                        all_items.extend(cached_items);
                    }
                }

                // Sort all items by date, newest first
                all_items.sort_by(|a, b| b.published.cmp(&a.published));
                self.current_feed_content = all_items;
                self.selected_index = Some(0);
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

pub async fn fetch_feed(url: &str) -> AppResult<Vec<FeedItem>> {
    debug!("Fetching feed from URL: {}", url);
    let client = create_http_client();
    let response = client.get(url).send().await?.bytes().await?;

    // Try parsing as RSS first
    match Channel::read_from(&response[..]) {
        Ok(channel) => {
            debug!("Successfully parsed RSS feed");
            Ok(convert_rss_items(channel, url))
        }
        Err(_) => {
            // Try parsing as Atom
            debug!("RSS parsing failed, attempting Atom format");
            match AtomFeed::read_from(&response[..]) {
                Ok(feed) => {
                    debug!("Successfully parsed Atom feed");
                    Ok(convert_atom_items(feed, url))
                }
                Err(e) => {
                    error!("Failed to parse feed as either RSS or Atom: {}", e);
                    Err(Box::new(e))
                }
            }
        }
    }
}

fn convert_rss_items(channel: Channel, feed_url: &str) -> Vec<FeedItem> {
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
                title: format!("{} | {}", item.title().unwrap_or("No title"), feed_url),
                description: clean_description,
                link: item.link().unwrap_or("").to_string(),
                published,
                id: App::create_item_id(item.title().unwrap_or("No title"), published),
            }
        })
        .collect()
}

fn convert_atom_items(feed: AtomFeed, feed_url: &str) -> Vec<FeedItem> {
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
                title: format!("{} | {}", entry.title().value, feed_url),
                description: clean_description,
                link: entry
                    .links()
                    .first()
                    .map(|l| l.href().to_string())
                    .unwrap_or_default(),
                published,
                id: App::create_item_id(&entry.title().value, published),
            }
        })
        .collect()
}
