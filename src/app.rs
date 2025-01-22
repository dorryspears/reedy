use std::{error, fs, path::PathBuf};
use log::{debug, error, info};
use rss::Channel;
use serde::{Deserialize, Serialize};
use reqwest;

/// Application result type.
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

#[derive(Debug, Clone)]
pub struct FeedItem {
    pub title: String,
    pub description: String,
    pub link: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SavedFeeds {
    feeds: Vec<String>,
}

/// Application.
#[derive(Debug)]
pub struct App {
    /// Is the application running?
    pub running: bool,
    pub input_mode: InputMode,
    pub page_mode: PageMode,
    pub input_buffer: String,
    pub rss_feeds: Vec<String>,
    pub selected_index: Option<usize>,
    pub current_feed_content: Vec<FeedItem>,
    pub error_message: Option<String>,
    save_path: PathBuf,
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
        }
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new() -> Self {
        let mut app = Self::default();
        app.load_feeds().unwrap_or_else(|e| {
            error!("Failed to load feeds: {}", e);
            app.error_message = Some(format!("Failed to load feeds: {}", e));
        });
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

    fn load_feeds(&mut self) -> AppResult<()> {
        if self.save_path.exists() {
            let content = fs::read_to_string(&self.save_path)?;
            let saved: SavedFeeds = serde_json::from_str(&content)?;
            self.rss_feeds = saved.feeds;
        }
        Ok(())
    }

    pub fn toggle_feed_manager(&mut self) {
        match self.page_mode {
            PageMode::FeedList => {
                self.page_mode = PageMode::FeedManager;
                self.input_mode = InputMode::Normal;
                self.selected_index = if !self.rss_feeds.is_empty() { Some(0) } else { None };
            }
            PageMode::FeedManager => {
                self.page_mode = PageMode::FeedList;
                self.input_mode = InputMode::Normal;
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
                match Channel::read_from(&bytes[..]) {
                    Ok(_) => Ok(true),
                    Err(_) => Ok(false),
                }
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
                debug!("Fetching feed content from URL: {}", url);
                let response = reqwest::get(url).await?;
                let content = response.bytes().await?;
                
                match Channel::read_from(&content[..]) {
                    Ok(channel) => {
                        info!("Successfully loaded feed: {}", url);
                        self.current_feed_content = channel.items().iter().map(|item| FeedItem {
                            title: item.title().unwrap_or("No title").to_string(),
                            description: item.description().unwrap_or("No description").to_string(),
                            link: item.link().unwrap_or("").to_string(),
                        }).collect();
                        debug!("Loaded {} items from feed", self.current_feed_content.len());
                    }
                    Err(e) => {
                        error!("Failed to parse feed {}: {}", url, e);
                        return Err(Box::new(e));
                    }
                }
            }
        }
        Ok(())
    }

    fn save_feeds(&self) -> AppResult<()> {
        let saved = SavedFeeds {
            feeds: self.rss_feeds.clone(),
        };
        let content = serde_json::to_string_pretty(&saved)?;
        fs::write(&self.save_path, content)?;
        debug!("Saved {} feeds to {}", self.rss_feeds.len(), self.save_path.display());
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
}
