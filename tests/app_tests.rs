use reedy::app::{App, FeedItem, InputMode, PageMode};
use std::time::SystemTime;

#[test]
fn test_app_default() {
    let app = App::default();
    assert!(app.running);
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
    assert!(!app.is_item_favorite(&item));

    // Add to favorites manually (bypassing toggle_favorite which requires selected_index)
    app.favorites.insert(item.id.clone());
    assert!(app.is_item_favorite(&item));
}

#[test]
fn test_select_next_empty_list() {
    let mut app = App::default();
    // Set selected_index to Some but with empty lists
    app.selected_index = Some(0);
    app.page_mode = PageMode::FeedList;
    // current_feed_content is empty by default
    assert_eq!(app.current_feed_content.len(), 0);

    // This should not panic - it should just return early
    app.select_next();

    // selected_index should remain unchanged
    assert_eq!(app.selected_index, Some(0));
}

#[test]
fn test_select_previous_empty_list() {
    let mut app = App::default();
    // Set selected_index to Some but with empty lists
    app.selected_index = Some(0);
    app.page_mode = PageMode::FeedList;
    // current_feed_content is empty by default
    assert_eq!(app.current_feed_content.len(), 0);

    // This should not panic - it should just return early
    app.select_previous();

    // selected_index should remain unchanged
    assert_eq!(app.selected_index, Some(0));
}

#[test]
fn test_select_next_empty_feed_manager() {
    let mut app = App::default();
    app.selected_index = Some(0);
    app.page_mode = PageMode::FeedManager;
    // rss_feeds is empty by default
    assert_eq!(app.rss_feeds.len(), 0);

    // This should not panic
    app.select_next();
    assert_eq!(app.selected_index, Some(0));
}

#[test]
fn test_select_previous_empty_feed_manager() {
    let mut app = App::default();
    app.selected_index = Some(0);
    app.page_mode = PageMode::FeedManager;
    // rss_feeds is empty by default
    assert_eq!(app.rss_feeds.len(), 0);

    // This should not panic
    app.select_previous();
    assert_eq!(app.selected_index, Some(0));
}

#[test]
fn test_unfavorite_removes_from_favorites_view() {
    let mut app = App::default();

    // Add some test items
    let item1 = FeedItem {
        title: "Item 1".to_string(),
        description: "Description 1".to_string(),
        link: "https://example.com/1".to_string(),
        published: Some(SystemTime::now()),
        id: "id-1".to_string(),
    };
    let item2 = FeedItem {
        title: "Item 2".to_string(),
        description: "Description 2".to_string(),
        link: "https://example.com/2".to_string(),
        published: Some(SystemTime::now()),
        id: "id-2".to_string(),
    };

    // Add items to current feed content and mark them as favorites
    app.current_feed_content.push(item1.clone());
    app.current_feed_content.push(item2.clone());
    app.favorites.insert(item1.id.clone());
    app.favorites.insert(item2.id.clone());

    // Switch to Favorites view
    app.page_mode = PageMode::Favorites;
    app.selected_index = Some(0);

    // Unfavorite the first item
    app.toggle_favorite();

    // The item should be removed from current_feed_content
    assert_eq!(app.current_feed_content.len(), 1);
    assert_eq!(app.current_feed_content[0].id, "id-2");
    // Selected index should remain at 0 (now pointing to item2)
    assert_eq!(app.selected_index, Some(0));
    // Item 1 should no longer be in favorites
    assert!(!app.favorites.contains(&item1.id));
}

#[test]
fn test_unfavorite_last_item_in_favorites_view() {
    let mut app = App::default();

    // Add a single test item
    let item = FeedItem {
        title: "Only Item".to_string(),
        description: "Description".to_string(),
        link: "https://example.com/only".to_string(),
        published: Some(SystemTime::now()),
        id: "only-id".to_string(),
    };

    // Add item to current feed content and mark as favorite
    app.current_feed_content.push(item.clone());
    app.favorites.insert(item.id.clone());

    // Switch to Favorites view
    app.page_mode = PageMode::Favorites;
    app.selected_index = Some(0);

    // Unfavorite the only item
    app.toggle_favorite();

    // The list should now be empty
    assert_eq!(app.current_feed_content.len(), 0);
    // Selected index should be None since list is empty
    assert_eq!(app.selected_index, None);
}

#[test]
fn test_unfavorite_does_not_remove_in_feedlist_view() {
    let mut app = App::default();

    // Add a test item
    let item = FeedItem {
        title: "Test Item".to_string(),
        description: "Description".to_string(),
        link: "https://example.com/test".to_string(),
        published: Some(SystemTime::now()),
        id: "test-id".to_string(),
    };

    // Add item to current feed content and mark as favorite
    app.current_feed_content.push(item.clone());
    app.favorites.insert(item.id.clone());

    // Stay in FeedList view (not Favorites)
    app.page_mode = PageMode::FeedList;
    app.selected_index = Some(0);

    // Unfavorite the item
    app.toggle_favorite();

    // The item should still be in current_feed_content (only removed in Favorites view)
    assert_eq!(app.current_feed_content.len(), 1);
    // But it should no longer be a favorite
    assert!(!app.favorites.contains(&item.id));
}

#[test]
fn test_items_per_page_dynamic_calculation() {
    let mut app = App::default();

    // Test with default terminal height (24)
    // Content height = 24 - 8 = 16
    // FeedList: 16 / 3 = 5 items
    app.page_mode = PageMode::FeedList;
    assert_eq!(app.items_per_page(), 5);

    // FeedManager: 16 - 1 = 15 items
    app.page_mode = PageMode::FeedManager;
    assert_eq!(app.items_per_page(), 15);

    // Test with larger terminal height (48)
    // Content height = 48 - 8 = 40
    app.terminal_height = 48;

    // FeedList: 40 / 3 = 13 items
    app.page_mode = PageMode::FeedList;
    assert_eq!(app.items_per_page(), 13);

    // FeedManager: 40 - 1 = 39 items
    app.page_mode = PageMode::FeedManager;
    assert_eq!(app.items_per_page(), 39);

    // Test with very small terminal height (10) to verify minimum of 1
    // Content height = 10 - 8 = 2
    app.terminal_height = 10;

    // FeedList: 2 / 3 = 0, but max(0, 1) = 1
    app.page_mode = PageMode::FeedList;
    assert_eq!(app.items_per_page(), 1);

    // FeedManager: 2 - 1 = 1 item
    app.page_mode = PageMode::FeedManager;
    assert_eq!(app.items_per_page(), 1);

    // Test Favorites mode uses same calculation as FeedList
    app.terminal_height = 24;
    app.page_mode = PageMode::Favorites;
    assert_eq!(app.items_per_page(), 5);
}

#[test]
fn test_search_filter_functionality() {
    let mut app = App::default();

    // Add test items
    let items = vec![
        FeedItem {
            title: "Rust Programming".to_string(),
            description: "Learn about Rust programming language".to_string(),
            link: "https://example.com/rust".to_string(),
            published: Some(SystemTime::now()),
            id: "id-1".to_string(),
        },
        FeedItem {
            title: "Python Tutorial".to_string(),
            description: "Getting started with Python".to_string(),
            link: "https://example.com/python".to_string(),
            published: Some(SystemTime::now()),
            id: "id-2".to_string(),
        },
        FeedItem {
            title: "JavaScript Guide".to_string(),
            description: "Modern JavaScript development with Rust tools".to_string(),
            link: "https://example.com/js".to_string(),
            published: Some(SystemTime::now()),
            id: "id-3".to_string(),
        },
    ];

    for item in items {
        app.current_feed_content.push(item);
    }
    app.selected_index = Some(0);

    // Initially, no filter is active
    assert!(app.filtered_indices.is_none());
    assert_eq!(app.visible_item_count(), 3);

    // Start search mode
    app.start_search();
    assert_eq!(app.input_mode, InputMode::Searching);
    assert!(app.search_query.is_empty());

    // Type a search query
    app.search_query = "rust".to_string();
    app.update_search_filter();

    // Should filter to items containing "rust" (case-insensitive)
    assert!(app.filtered_indices.is_some());
    assert_eq!(app.visible_item_count(), 2); // "Rust Programming" and "JavaScript Guide" (contains "Rust tools" in description)

    // Confirm search
    app.confirm_search();
    assert_eq!(app.input_mode, InputMode::Normal);
    // Filter should still be active
    assert!(app.filtered_indices.is_some());
    assert_eq!(app.visible_item_count(), 2);

    // Clear search
    app.clear_search();
    assert!(app.filtered_indices.is_none());
    assert_eq!(app.visible_item_count(), 3);

    // Test cancel search
    app.start_search();
    app.search_query = "python".to_string();
    app.update_search_filter();
    assert_eq!(app.visible_item_count(), 1);

    app.cancel_search();
    assert_eq!(app.input_mode, InputMode::Normal);
    assert!(app.filtered_indices.is_none());
    assert_eq!(app.visible_item_count(), 3);
}

#[test]
fn test_search_with_no_matches() {
    let mut app = App::default();

    // Add test item
    app.current_feed_content.push(FeedItem {
        title: "Rust Programming".to_string(),
        description: "Learn Rust".to_string(),
        link: "https://example.com/rust".to_string(),
        published: Some(SystemTime::now()),
        id: "id-1".to_string(),
    });
    app.selected_index = Some(0);

    // Search for something that doesn't exist
    app.search_query = "xyz123nonexistent".to_string();
    app.update_search_filter();

    // Should have zero visible items
    assert_eq!(app.visible_item_count(), 0);
    assert_eq!(app.selected_index, None);
}

#[test]
fn test_get_actual_index_with_filter() {
    let mut app = App::default();

    // Add 5 items
    for i in 0..5 {
        app.current_feed_content.push(FeedItem {
            title: format!("Item {}", i),
            description: format!("Description {}", i),
            link: format!("https://example.com/{}", i),
            published: Some(SystemTime::now()),
            id: format!("id-{}", i),
        });
    }

    // Without filter, visible index equals actual index
    assert_eq!(app.get_actual_index(0), Some(0));
    assert_eq!(app.get_actual_index(2), Some(2));
    assert_eq!(app.get_actual_index(4), Some(4));
    assert_eq!(app.get_actual_index(5), None); // Out of bounds

    // Set up a filter that shows items 1 and 3 only
    app.filtered_indices = Some(vec![1, 3]);

    // Now visible index 0 -> actual index 1
    // visible index 1 -> actual index 3
    assert_eq!(app.get_actual_index(0), Some(1));
    assert_eq!(app.get_actual_index(1), Some(3));
    assert_eq!(app.get_actual_index(2), None); // Out of bounds in filtered list
}

#[test]
fn test_start_importing_mode() {
    let mut app = App::default();
    assert_eq!(app.input_mode, InputMode::Normal);

    // Start importing
    app.start_importing();
    assert_eq!(app.input_mode, InputMode::Importing);
    assert!(app.import_result.is_none());
    // Note: input_buffer may or may not be empty depending on clipboard state
}

#[test]
fn test_cancel_importing() {
    let mut app = App::default();

    // Start importing and add some text
    app.input_mode = InputMode::Importing;
    app.input_buffer = "https://example.com/feed.xml".to_string();
    app.import_result = Some("test".to_string());

    // Cancel
    app.cancel_importing();

    assert_eq!(app.input_mode, InputMode::Normal);
    assert!(app.input_buffer.is_empty());
    assert!(app.import_result.is_none());
}

#[test]
fn test_export_empty_feeds() {
    let mut app = App::default();
    assert!(app.rss_feeds.is_empty());

    // Export should set an error message
    app.export_feeds_to_clipboard();
    assert_eq!(app.error_message, Some("No feeds to export".to_string()));
}
