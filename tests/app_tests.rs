use reedy::app::{App, FeedInfo, FeedItem, InputMode, PageMode};
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
        feed_url: String::new(),
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
        feed_url: String::new(),
    };
    let item2 = FeedItem {
        title: "Item 2".to_string(),
        description: "Description 2".to_string(),
        link: "https://example.com/2".to_string(),
        published: Some(SystemTime::now()),
        id: "id-2".to_string(),
        feed_url: String::new(),
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
        feed_url: String::new(),
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
        feed_url: String::new(),
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
            feed_url: String::new(),
        },
        FeedItem {
            title: "Python Tutorial".to_string(),
            description: "Getting started with Python".to_string(),
            link: "https://example.com/python".to_string(),
            published: Some(SystemTime::now()),
            id: "id-2".to_string(),
            feed_url: String::new(),
        },
        FeedItem {
            title: "JavaScript Guide".to_string(),
            description: "Modern JavaScript development with Rust tools".to_string(),
            link: "https://example.com/js".to_string(),
            published: Some(SystemTime::now()),
            id: "id-3".to_string(),
            feed_url: String::new(),
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
        feed_url: String::new(),
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
            feed_url: String::new(),
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

#[test]
fn test_category_setting_mode() {
    let mut app = App::default();

    // Add a test feed
    app.rss_feeds.push(FeedInfo {
        url: "https://example.com/feed.xml".to_string(),
        title: "Test Feed".to_string(),
        category: None,
    });
    app.selected_index = Some(0);
    app.page_mode = PageMode::FeedManager;

    // Start category setting mode
    app.start_setting_category();
    assert_eq!(app.input_mode, InputMode::SettingCategory);
    assert!(app.input_buffer.is_empty()); // No existing category

    // Set a category
    app.input_buffer = "Tech".to_string();
    app.set_category();

    assert_eq!(app.input_mode, InputMode::Normal);
    assert!(app.input_buffer.is_empty());
    assert_eq!(app.rss_feeds[0].category, Some("Tech".to_string()));
}

#[test]
fn test_category_prefill_existing() {
    let mut app = App::default();

    // Add a feed with existing category
    app.rss_feeds.push(FeedInfo {
        url: "https://example.com/feed.xml".to_string(),
        title: "Test Feed".to_string(),
        category: Some("News".to_string()),
    });
    app.selected_index = Some(0);
    app.page_mode = PageMode::FeedManager;

    // Start category setting mode - should prefill with existing category
    app.start_setting_category();
    assert_eq!(app.input_mode, InputMode::SettingCategory);
    assert_eq!(app.input_buffer, "News");
}

#[test]
fn test_clear_category() {
    let mut app = App::default();

    // Add a feed with existing category
    app.rss_feeds.push(FeedInfo {
        url: "https://example.com/feed.xml".to_string(),
        title: "Test Feed".to_string(),
        category: Some("Tech".to_string()),
    });
    app.selected_index = Some(0);
    app.page_mode = PageMode::FeedManager;

    // Set empty category to clear it
    app.start_setting_category();
    app.input_buffer.clear();
    app.set_category();

    assert_eq!(app.rss_feeds[0].category, None);
}

#[test]
fn test_cancel_category_setting() {
    let mut app = App::default();

    // Add a feed
    app.rss_feeds.push(FeedInfo {
        url: "https://example.com/feed.xml".to_string(),
        title: "Test Feed".to_string(),
        category: Some("Original".to_string()),
    });
    app.selected_index = Some(0);
    app.page_mode = PageMode::FeedManager;

    // Start and cancel category setting
    app.start_setting_category();
    app.input_buffer = "NewCategory".to_string();
    app.cancel_setting_category();

    assert_eq!(app.input_mode, InputMode::Normal);
    assert!(app.input_buffer.is_empty());
    // Original category should be preserved
    assert_eq!(app.rss_feeds[0].category, Some("Original".to_string()));
}

#[test]
fn test_get_categories() {
    let mut app = App::default();

    // Add feeds with various categories
    app.rss_feeds.push(FeedInfo {
        url: "https://example.com/1.xml".to_string(),
        title: "Feed 1".to_string(),
        category: Some("Tech".to_string()),
    });
    app.rss_feeds.push(FeedInfo {
        url: "https://example.com/2.xml".to_string(),
        title: "Feed 2".to_string(),
        category: Some("News".to_string()),
    });
    app.rss_feeds.push(FeedInfo {
        url: "https://example.com/3.xml".to_string(),
        title: "Feed 3".to_string(),
        category: Some("Tech".to_string()), // Duplicate category
    });
    app.rss_feeds.push(FeedInfo {
        url: "https://example.com/4.xml".to_string(),
        title: "Feed 4".to_string(),
        category: None, // Uncategorized
    });

    let categories = app.get_categories();
    assert_eq!(categories.len(), 2); // Only "News" and "Tech", sorted
    assert_eq!(categories[0], "News");
    assert_eq!(categories[1], "Tech");
}

#[test]
fn test_get_feeds_by_category() {
    let mut app = App::default();

    // Add feeds with various categories
    app.rss_feeds.push(FeedInfo {
        url: "https://example.com/1.xml".to_string(),
        title: "Tech Feed 1".to_string(),
        category: Some("Tech".to_string()),
    });
    app.rss_feeds.push(FeedInfo {
        url: "https://example.com/2.xml".to_string(),
        title: "Uncategorized Feed".to_string(),
        category: None,
    });
    app.rss_feeds.push(FeedInfo {
        url: "https://example.com/3.xml".to_string(),
        title: "News Feed".to_string(),
        category: Some("News".to_string()),
    });
    app.rss_feeds.push(FeedInfo {
        url: "https://example.com/4.xml".to_string(),
        title: "Tech Feed 2".to_string(),
        category: Some("Tech".to_string()),
    });

    let grouped = app.get_feeds_by_category();

    // Should have 3 groups: None (uncategorized), News, Tech
    assert_eq!(grouped.len(), 3);

    // First group should be uncategorized (None comes first)
    assert_eq!(grouped[0].0, None);
    assert_eq!(grouped[0].1.len(), 1);
    assert_eq!(grouped[0].1[0].title, "Uncategorized Feed");

    // Second group should be News
    assert_eq!(grouped[1].0, Some("News".to_string()));
    assert_eq!(grouped[1].1.len(), 1);

    // Third group should be Tech with 2 feeds
    assert_eq!(grouped[2].0, Some("Tech".to_string()));
    assert_eq!(grouped[2].1.len(), 2);
}

#[test]
fn test_scroll_to_bottom() {
    let mut app = App::default();

    // Add 10 feed items
    for i in 0..10 {
        app.current_feed_content.push(FeedItem {
            title: format!("Item {}", i),
            description: format!("Description {}", i),
            link: format!("https://example.com/{}", i),
            published: Some(SystemTime::now()),
            id: format!("id-{}", i),
            feed_url: String::new(),
        });
    }

    // Set selected to first item
    app.selected_index = Some(0);
    app.scroll = 0;
    app.page_mode = PageMode::FeedList;
    app.terminal_height = 30; // Set reasonable terminal height

    // Scroll to bottom
    app.scroll_to_bottom();

    // Should select the last item (index 9)
    assert_eq!(app.selected_index, Some(9));
}

#[test]
fn test_scroll_to_bottom_empty_list() {
    let mut app = App::default();

    // No items, no selection
    app.page_mode = PageMode::FeedList;

    // Should not panic on empty list
    app.scroll_to_bottom();

    // Selected index should remain None
    assert_eq!(app.selected_index, None);
}

#[test]
fn test_scroll_to_bottom_feed_manager() {
    let mut app = App::default();

    // Add 5 feeds
    for i in 0..5 {
        app.rss_feeds.push(FeedInfo {
            url: format!("https://example.com/{}.xml", i),
            title: format!("Feed {}", i),
            category: None,
        });
    }

    app.selected_index = Some(0);
    app.scroll = 0;
    app.page_mode = PageMode::FeedManager;
    app.terminal_height = 30;

    // Scroll to bottom
    app.scroll_to_bottom();

    // Should select the last feed (index 4)
    assert_eq!(app.selected_index, Some(4));
}

#[test]
fn test_scroll_to_bottom_with_filter() {
    let mut app = App::default();

    // Add 10 feed items
    for i in 0..10 {
        app.current_feed_content.push(FeedItem {
            title: format!("Item {}", i),
            description: format!("Description {}", i),
            link: format!("https://example.com/{}", i),
            published: Some(SystemTime::now()),
            id: format!("id-{}", i),
            feed_url: String::new(),
        });
    }

    // Set up a filter that only shows items 2, 5, 7
    app.filtered_indices = Some(vec![2, 5, 7]);
    app.selected_index = Some(0); // First visible item
    app.scroll = 0;
    app.page_mode = PageMode::FeedList;
    app.terminal_height = 30;

    // Scroll to bottom
    app.scroll_to_bottom();

    // Should select the last visible item (index 2 in visible list, which is actual index 7)
    assert_eq!(app.selected_index, Some(2)); // Visible index, not actual
}
