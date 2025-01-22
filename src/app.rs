use atom_syndication::Feed as AtomFeed;
use base64;
use chrono::DateTime;
use html2text;
use log::{debug, error, info};
use reqwest;
use rss::Channel;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, error, fs, path::PathBuf, time::SystemTime};

pub type AppResult<T> = std::result::Result<T, Box<dyn error::Error>>;

#[derive(Debug, PartialEq)]
pub enum InputMode {
    Normal,
    Adding,
    Deleting,
    FeedManager,
}

#[derive(Debug, PartialEq)]
pub enum PageMode {
    FeedList,
    FeedManager,
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
    pub scroll: u16,
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
            scroll: 0,
        }
    }
}

impl App {
    pub fn new() -> Self {
        let mut app = Self::default();

        // Clear the cache directory on startup
        Self::clear_cache_dir();

        app.load_feeds().unwrap_or_else(|e| {
            error!("Failed to load feeds: {}", e);
            app.error_message = Some(format!("Failed to load feeds: {}", e));
        });

        // Cache all feeds in the background
        if !app.rss_feeds.is_empty() {
            app.selected_index = Some(0);
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    app.cache_all_feeds().await;
                    if let Err(e) = app.load_feed_content().await {
                        error!("Failed to load initial feed content: {}", e);
                        app.error_message = Some(format!("Failed to load feed content: {}", e));
                    }
                });
            });
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
        };
        let content = serde_json::to_string_pretty(&saved)?;
        fs::write(&self.save_path, content)?;
        debug!(
            "Saved {} feeds and {} read items to {}",
            self.rss_feeds.len(),
            self.read_items.len(),
            self.save_path.display()
        );
        Ok(())
    }

    fn load_feeds(&mut self) -> AppResult<()> {
        if self.save_path.exists() {
            let content = fs::read_to_string(&self.save_path)?;
            let saved: SavedState = serde_json::from_str(&content)?;
            self.rss_feeds = saved.feeds;
            self.read_items = saved.read_items;
            debug!(
                "Loaded {} feeds from {}",
                self.rss_feeds.len(),
                self.save_path.display()
            );
        }
        Ok(())
    }

    pub fn toggle_feed_manager(&mut self) {
        match self.page_mode {
            PageMode::FeedList => {
                self.page_mode = PageMode::FeedManager;
                self.selected_index = Some(0);
            }
            PageMode::FeedManager => {
                self.page_mode = PageMode::FeedList;
                // Reset selection and trigger refresh
                self.selected_index = Some(0);
                // Using block_in_place because we can't use .await directly
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        if let Err(e) = self.refresh_all_feeds().await {
                            error!("Failed to refresh feeds: {}", e);
                            self.error_message = Some(format!("Failed to refresh feeds: {}", e));
                        }
                    });
                });
            }
        }
    }

    pub fn select_previous(&mut self) {
        if let Some(current) = self.selected_index {
            let len = match self.page_mode {
                PageMode::FeedList => self.current_feed_content.len(),
                PageMode::FeedManager => self.rss_feeds.len(),
            };
            self.selected_index = Some(if current > 0 { current - 1 } else { len - 1 });
        }
    }

    pub fn select_next(&mut self) {
        if let Some(current) = self.selected_index {
            let len = match self.page_mode {
                PageMode::FeedList => self.current_feed_content.len(),
                PageMode::FeedManager => self.rss_feeds.len(),
            };
            self.selected_index = Some((current + 1) % len);
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
        match reqwest::get(url.as_str()).await {
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
                let response = reqwest::get(url).await?;
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
        };
        let content = serde_json::to_string_pretty(&saved)?;
        fs::write(&self.save_path, content)?;
        debug!(
            "Saved {} feeds and {} read items to {}",
            self.rss_feeds.len(),
            self.read_items.len(),
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
            self.scroll -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        let max_scroll = match self.page_mode {
            PageMode::FeedList => self.current_feed_content.len(),
            PageMode::FeedManager => self.rss_feeds.len(),
        };
        if (self.scroll as usize) < max_scroll.saturating_sub(1) {
            self.scroll += 1;
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

    pub async fn cache_all_feeds(&mut self) {
        for url in self.rss_feeds.clone() {
            debug!("Checking cache for URL: {}", url);

            // Skip if already cached
            if self.load_feed_cache(&url).is_some() {
                debug!("Using existing cache for {}", url);
                continue;
            }

            debug!("Fetching feed content from URL: {}", url);
            match reqwest::get(&url).await {
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

    pub async fn refresh_all_feeds(&mut self) -> AppResult<()> {
        let mut all_items = Vec::new();

        for url in &self.rss_feeds {
            debug!("Refreshing feed: {}", url);
            match reqwest::get(url).await {
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

    fn clear_cache_dir() {
        let cache_dir = Self::get_cache_dir();
        if cache_dir.exists() {
            if let Err(e) = fs::remove_dir_all(&cache_dir) {
                error!("Failed to clear cache directory: {}", e);
            }
            if let Err(e) = fs::create_dir_all(&cache_dir) {
                error!("Failed to recreate cache directory: {}", e);
            }
        }
    }
}

pub async fn fetch_feed(url: &str) -> AppResult<Vec<FeedItem>> {
    debug!("Fetching feed from URL: {}", url);
    let response = reqwest::get(url).await?.bytes().await?;

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
