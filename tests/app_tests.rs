use reedy::app::{App, InputMode, PageMode, FeedItem};
use std::time::SystemTime;

#[test]
fn test_app_default() {
    let app = App::default();
    assert_eq!(app.running, true);
    assert_eq!(app.input_mode, InputMode::Normal);
    assert_eq!(app.page_mode, PageMode::FeedList);
    assert_eq!(app.input_buffer, "");
    assert_eq!(app.rss_feeds.len(), 0);
    assert_eq!(app.selected_index, None);
    assert_eq!(app.current_feed_content.len(), 0);
    assert_eq!(app.error_message, None);
    assert_eq!(app.scroll, 0);
}

#[test]
fn test_app_toggle_feed_manager() {
    let mut app = App::default();
    app.page_mode = PageMode::FeedList;
    app.toggle_feed_manager();
    assert_eq!(app.page_mode, PageMode::FeedManager);
    
    app.toggle_feed_manager();
    assert_eq!(app.page_mode, PageMode::FeedList);
}

#[test]
fn test_app_toggle_help() {
    let mut app = App::default();
    assert_eq!(app.input_mode, InputMode::Normal);
    
    app.toggle_help();
    assert_eq!(app.input_mode, InputMode::Help);
    
    app.toggle_help();
    assert_eq!(app.input_mode, InputMode::Normal);
}

#[test]
fn test_app_item_favorite() {
    let mut app = App::default();
    let item = FeedItem {
        title: "Test Title".to_string(),
        description: "Test Description".to_string(),
        link: "https://example.com".to_string(),
        published: Some(SystemTime::now()),
        id: "test-id".to_string(),
    };
    
    // Initially not a favorite
    assert_eq!(app.is_item_favorite(&item), false);
    
    // Add to favorites manually (bypassing toggle_favorite which requires selected_index)
    app.favorites.insert(item.id.clone());
    assert_eq!(app.is_item_favorite(&item), true);
} 