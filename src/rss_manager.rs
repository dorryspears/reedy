use atom_syndication::Feed as AtomFeed;
use base64;
use chrono::DateTime;
use html2text;
use log::{debug, error, info};
use reqwest;
use rss::Channel;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, error, fs, path::PathBuf, time::SystemTime};

pub type RssResult<T> = std::result::Result<T, Box<dyn error::Error>>;

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
pub struct RssManager {
    pub rss_feeds: Vec<String>,
    pub current_feed_content: Vec<FeedItem>,
    save_path: PathBuf,
    read_items: HashSet<String>,
    favorites: HashSet<String>,
}

impl Default for RssManager {
    fn default() -> Self {
        Self {
            rss_feeds: Vec::new(),
            current_feed_content: Vec::new(),
            save_path: Self::get_save_path(),
            read_items: HashSet::new(),
            favorites: HashSet::new(),
        }
    }
}

impl RssManager {
    pub fn new() -> Self {
        let mut manager = Self::default();

        // Clear the cache directory on startup
        Self::clear_cache_dir();

        manager.load_feeds().unwrap_or_else(|e| {
            error!("Failed to load feeds: {}", e);
        });

        // Cache all feeds in the background and load all cached content
        if !manager.rss_feeds.is_empty() {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    let _ = manager.refresh_all_feeds().await;
                    manager.cache_all_feeds().await;

                    // Load and combine all cached feed content
                    let mut all_items = Vec::new();
                    for url in &manager.rss_feeds {
                        if let Some(cached_items) = manager.load_feed_cache(url) {
                            all_items.extend(cached_items);
                        }
                    }

                    // Sort all items by date, newest first
                    all_items.sort_by(|a, b| b.published.cmp(&a.published));
                    manager.current_feed_content = all_items;
                });
            });
        }
        manager
    }

    pub fn get_save_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("reedy");
        fs::create_dir_all(&path).unwrap_or_default();
        path.push("feeds.json");
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

    pub fn toggle_read_status(&mut self, index: usize) -> bool {
        if let Some(item) = self.current_feed_content.get(index) {
            let was_read = self.read_items.contains(&item.id);
            if was_read {
                self.read_items.remove(&item.id);
                debug!("Marked item as unread: {}", item.title);
            } else {
                self.read_items.insert(item.id.clone());
                debug!("Marked item as read: {}", item.title);
            }
            self.save_state().unwrap_or_else(|e| {
                error!("Failed to save read status: {}", e);
            });
            return true;
        }
        false
    }

    fn save_state(&self) -> RssResult<()> {
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

    fn load_feeds(&mut self) -> RssResult<()> {
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

    pub async fn is_valid_rss_feed(url: &str) -> RssResult<bool> {
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

    pub async fn add_feed(&mut self, url: &str) -> RssResult<bool> {
        debug!("Attempting to add feed: {}", url);
        match Self::is_valid_rss_feed(url).await {
            Ok(true) => {
                info!("Successfully validated feed: {}", url);
                self.rss_feeds.push(url.to_string());
                self.save_feeds()?;
                Ok(true)
            }
            Ok(false) => {
                error!("Invalid RSS feed URL: {}", url);
                Ok(false)
            }
            Err(e) => {
                error!("Error validating feed: {}", e);
                Err(e)
            }
        }
    }

    pub fn delete_feed(&mut self, index: usize) -> bool {
        if index < self.rss_feeds.len() {
            self.rss_feeds.remove(index);
            self.current_feed_content.clear();
            if let Err(e) = self.save_feeds() {
                error!("Failed to save feeds after deletion: {}", e);
                return false;
            }
            return true;
        }
        false
    }

    pub async fn select_feed(&mut self, index: usize) -> RssResult<()> {
        if index < self.rss_feeds.len() {
            debug!("Loading feed content from index {}", index);
            let feed_url = self.rss_feeds[index].clone();
            self.load_feed_content(&feed_url).await?;
        }
        Ok(())
    }

    pub async fn load_feed_content(&mut self, url: &str) -> RssResult<()> {
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
                convert_rss_items(channel, url)
            }
            Err(_) => {
                // Try parsing as Atom feed
                match AtomFeed::read_from(&content[..]) {
                    Ok(feed) => convert_atom_items(feed, url),
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
    }

    fn save_feeds(&self) -> RssResult<()> {
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

    pub fn is_item_favorite(&self, item: &FeedItem) -> bool {
        self.favorites.contains(&item.id)
    }

    pub fn toggle_favorite(&mut self, index: usize) -> bool {
        if let Some(item) = self.current_feed_content.get(index) {
            if self.favorites.contains(&item.id) {
                self.favorites.remove(&item.id);
                debug!("Removed item from favorites: {}", item.title);
            } else {
                self.favorites.insert(item.id.clone());
                debug!("Added item to favorites: {}", item.title);
            }
            self.save_state().unwrap_or_else(|e| {
                error!("Failed to save favorites: {}", e);
            });
            return true;
        }
        false
    }

    pub fn mark_as_read(&mut self, index: usize) -> bool {
        if let Some(item) = self.current_feed_content.get(index) {
            if !self.read_items.contains(&item.id) {
                self.read_items.insert(item.id.clone());
                debug!("Marked item as read: {}", item.title);
                self.save_state().unwrap_or_else(|e| {
                    error!("Failed to save read status: {}", e);
                });
                return true;
            }
        }
        false
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

    pub async fn get_favorites(&mut self) -> Vec<FeedItem> {
        // Filter all feeds to show only favorites
        let all_items = self.get_all_feed_items().await;
        all_items
            .into_iter()
            .filter(|item| self.favorites.contains(&item.id))
            .collect()
    }

    pub async fn get_all_feed_items(&mut self) -> Vec<FeedItem> {
        let mut all_items = Vec::new();

        // Try to load from cache first
        for url in &self.rss_feeds {
            if let Some(cached_items) = self.load_feed_cache(url) {
                all_items.extend(cached_items);
            }
        }

        // If no cached items were found, refresh feeds
        if all_items.is_empty() {
            let _ = self.refresh_all_feeds().await;
        }

        // Sort all items by date, newest first
        all_items.sort_by(|a, b| b.published.cmp(&a.published));
        all_items
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

    fn save_feed_cache(&self, url: &str, content: &[FeedItem]) -> RssResult<()> {
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

    /// Refreshes all RSS/Atom feeds by fetching their latest content.
    pub async fn refresh_all_feeds(&mut self) -> RssResult<()> {
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

pub async fn fetch_feed(url: &str) -> RssResult<Vec<FeedItem>> {
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
                id: RssManager::create_item_id(item.title().unwrap_or("No title"), published),
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
                id: RssManager::create_item_id(&entry.title().value, published),
            }
        })
        .collect()
}
