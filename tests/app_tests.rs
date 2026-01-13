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

#[test]
fn test_opml_generate() {
    let mut app = App::default();

    // Add feeds with various categories
    app.rss_feeds.push(FeedInfo {
        url: "https://example.com/feed1.xml".to_string(),
        title: "Tech News".to_string(),
        category: Some("Technology".to_string()),
    });
    app.rss_feeds.push(FeedInfo {
        url: "https://example.com/feed2.xml".to_string(),
        title: "Science Daily".to_string(),
        category: Some("Science".to_string()),
    });
    app.rss_feeds.push(FeedInfo {
        url: "https://example.com/feed3.xml".to_string(),
        title: "Uncategorized Feed".to_string(),
        category: None,
    });

    // Generate OPML - using the private method via reflection is not possible,
    // but we can test that export_opml would not panic on valid data
    // For a fuller test, we'd need to make generate_opml public or test via export_opml
    // For now, just verify the feeds are set up correctly
    assert_eq!(app.rss_feeds.len(), 3);
}

#[tokio::test]
async fn test_opml_import_empty_content() {
    let mut app = App::default();

    // Import empty/invalid OPML content
    let result = app.import_opml_content("").await;
    assert!(result.is_ok());

    // Should report no feeds found
    assert!(app.error_message.is_some());
    assert!(app.error_message.as_ref().unwrap().contains("No feeds found"));
}

#[tokio::test]
async fn test_opml_import_basic() {
    let mut app = App::default();

    let opml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="2.0">
  <head><title>Test Feeds</title></head>
  <body>
    <outline type="rss" text="Test Feed 1" xmlUrl="https://example.com/feed1.xml"/>
    <outline type="rss" text="Test Feed 2" title="Test Feed 2 Title" xmlUrl="https://example.com/feed2.xml"/>
  </body>
</opml>"#;

    let result = app.import_opml_content(opml_content).await;
    assert!(result.is_ok());

    // Should have imported 2 feeds
    assert_eq!(app.rss_feeds.len(), 2);
    assert_eq!(app.rss_feeds[0].url, "https://example.com/feed1.xml");
    assert_eq!(app.rss_feeds[0].title, "Test Feed 1");
    assert_eq!(app.rss_feeds[1].url, "https://example.com/feed2.xml");
}

#[tokio::test]
async fn test_opml_import_with_categories() {
    let mut app = App::default();

    let opml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="2.0">
  <head><title>Test Feeds</title></head>
  <body>
    <outline text="Tech">
      <outline type="rss" text="Tech Feed" xmlUrl="https://example.com/tech.xml"/>
    </outline>
    <outline type="rss" text="Uncategorized Feed" xmlUrl="https://example.com/other.xml"/>
  </body>
</opml>"#;

    let result = app.import_opml_content(opml_content).await;
    assert!(result.is_ok());

    // Should have imported 2 feeds
    assert_eq!(app.rss_feeds.len(), 2);

    // First feed should have "Tech" category
    assert_eq!(app.rss_feeds[0].category, Some("Tech".to_string()));
    assert_eq!(app.rss_feeds[0].title, "Tech Feed");

    // Second feed should have no category
    assert_eq!(app.rss_feeds[1].category, None);
    assert_eq!(app.rss_feeds[1].title, "Uncategorized Feed");
}

#[tokio::test]
async fn test_opml_import_skips_duplicates() {
    let mut app = App::default();

    // Pre-add a feed
    app.rss_feeds.push(FeedInfo {
        url: "https://example.com/existing.xml".to_string(),
        title: "Existing Feed".to_string(),
        category: None,
    });

    let opml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="2.0">
  <head><title>Test Feeds</title></head>
  <body>
    <outline type="rss" text="Existing Feed" xmlUrl="https://example.com/existing.xml"/>
    <outline type="rss" text="New Feed" xmlUrl="https://example.com/new.xml"/>
  </body>
</opml>"#;

    let result = app.import_opml_content(opml_content).await;
    assert!(result.is_ok());

    // Should only have 2 feeds (existing + 1 new)
    assert_eq!(app.rss_feeds.len(), 2);

    // Check the import result message mentions duplicates
    assert!(app.error_message.as_ref().unwrap().contains("1 added"));
    assert!(app.error_message.as_ref().unwrap().contains("1 duplicate"));
}

#[test]
fn test_export_opml_empty_feeds() {
    let mut app = App::default();

    // Try to export with no feeds
    let result = app.export_opml();
    assert!(result.is_err());
    assert!(app.error_message.is_some());
    assert!(app.error_message.as_ref().unwrap().contains("No feeds to export"));
}

// Keybindings tests

#[test]
fn test_keybindings_default() {
    use reedy::app::Keybindings;

    let kb = Keybindings::default();

    // Verify default keybindings match expected vim-style keys
    assert_eq!(kb.move_up, "k,Up");
    assert_eq!(kb.move_down, "j,Down");
    assert_eq!(kb.page_up, "PageUp");
    assert_eq!(kb.page_down, "PageDown");
    assert_eq!(kb.scroll_to_top, "g");
    assert_eq!(kb.scroll_to_bottom, "G");
    assert_eq!(kb.select, "Enter");
    assert_eq!(kb.open_in_browser, "o");
    assert_eq!(kb.toggle_read, "r");
    assert_eq!(kb.mark_all_read, "R");
    assert_eq!(kb.toggle_favorite, "f");
    assert_eq!(kb.toggle_favorites_view, "F");
    assert_eq!(kb.refresh, "c");
    assert_eq!(kb.start_search, "/");
    assert_eq!(kb.open_preview, "p");
    assert_eq!(kb.open_feed_manager, "m");
    assert_eq!(kb.add_feed, "a");
    assert_eq!(kb.delete_feed, "d");
    assert_eq!(kb.set_category, "t");
    assert_eq!(kb.export_clipboard, "e");
    assert_eq!(kb.export_opml, "E");
    assert_eq!(kb.import_clipboard, "i");
    assert_eq!(kb.import_opml, "I");
    assert_eq!(kb.help, "?");
    assert_eq!(kb.quit, "q");
}

#[test]
fn test_config_includes_keybindings() {
    use reedy::app::Config;

    let config = Config::default();

    // Config should include default keybindings
    assert_eq!(config.keybindings.move_up, "k,Up");
    assert_eq!(config.keybindings.quit, "q");
}

#[test]
fn test_keybindings_serialization() {
    use reedy::app::Keybindings;

    let kb = Keybindings::default();

    // Serialize to JSON
    let json = serde_json::to_string(&kb).unwrap();

    // Deserialize back
    let kb2: Keybindings = serde_json::from_str(&json).unwrap();

    // Should match original
    assert_eq!(kb, kb2);
}

#[test]
fn test_keybindings_partial_config() {
    use reedy::app::Keybindings;

    // Test that partial JSON deserializes with defaults for missing fields
    let json = r#"{"move_up": "w,Up", "quit": "x"}"#;
    let kb: Keybindings = serde_json::from_str(json).unwrap();

    // Custom values
    assert_eq!(kb.move_up, "w,Up");
    assert_eq!(kb.quit, "x");

    // Defaults for missing values
    assert_eq!(kb.move_down, "j,Down");
    assert_eq!(kb.toggle_favorite, "f");
}

#[test]
fn test_command_mode_start_and_cancel() {
    let mut app = App::default();
    assert_eq!(app.input_mode, InputMode::Normal);
    assert!(app.command_buffer.is_empty());

    // Enter command mode
    app.start_command_mode();
    assert_eq!(app.input_mode, InputMode::Command);
    assert!(app.command_buffer.is_empty());

    // Type a command
    app.command_buffer.push_str("quit");

    // Cancel command mode
    app.cancel_command_mode();
    assert_eq!(app.input_mode, InputMode::Normal);
    assert!(app.command_buffer.is_empty());
}

#[test]
fn test_command_quit() {
    let mut app = App::default();
    assert!(app.running);

    app.command_buffer = "q".to_string();
    let result = app.execute_command();
    assert!(result.is_ok());
    assert!(!app.running);
}

#[test]
fn test_command_quit_long() {
    let mut app = App::default();
    assert!(app.running);

    app.command_buffer = "quit".to_string();
    let result = app.execute_command();
    assert!(result.is_ok());
    assert!(!app.running);
}

#[test]
fn test_command_wq() {
    let mut app = App::default();
    assert!(app.running);

    app.command_buffer = "wq".to_string();
    let result = app.execute_command();
    assert!(result.is_ok());
    assert!(!app.running);
}

#[test]
fn test_command_help() {
    let mut app = App::default();
    assert_eq!(app.input_mode, InputMode::Normal);

    app.command_buffer = "help".to_string();
    let result = app.execute_command();
    assert!(result.is_ok());
    assert_eq!(app.input_mode, InputMode::Help);
}

#[test]
fn test_command_feeds() {
    let mut app = App::default();
    assert_eq!(app.page_mode, PageMode::FeedList);

    app.command_buffer = "feeds".to_string();
    let result = app.execute_command();
    assert!(result.is_ok());
    assert_eq!(app.page_mode, PageMode::FeedManager);
}

#[test]
fn test_command_unknown() {
    let mut app = App::default();

    app.command_buffer = "unknown_command".to_string();
    let result = app.execute_command();
    assert!(result.is_ok()); // Returns Ok(false) for unknown commands
    assert!(app.error_message.is_some());
    assert!(app.error_message.unwrap().contains("Unknown command"));
}

#[test]
fn test_command_empty() {
    let mut app = App::default();
    assert!(app.running);

    app.command_buffer = "".to_string();
    let result = app.execute_command();
    assert!(result.is_ok());
    assert!(app.running); // Should not quit on empty command
}

#[test]
fn test_command_scroll_to_top() {
    let mut app = App::default();
    app.scroll = 10;

    app.command_buffer = "0".to_string();
    let result = app.execute_command();
    assert!(result.is_ok());
    assert_eq!(app.scroll, 0);
}

#[test]
fn test_export_article_no_selection() {
    let mut app = App::default();
    // No article selected
    app.export_article_to_clipboard();
    assert!(app.error_message.is_some());
    assert!(app.error_message.unwrap().contains("No article selected"));
}

#[test]
fn test_export_article_file_no_selection() {
    let mut app = App::default();
    // No article selected
    app.export_article_to_file();
    assert!(app.error_message.is_some());
    assert!(app.error_message.unwrap().contains("No article selected"));
}

#[test]
fn test_keybindings_export_article_default() {
    let keybindings = reedy::app::Keybindings::default();
    assert_eq!(keybindings.export_article, "s");
}

#[test]
fn test_feed_health_default() {
    use reedy::app::{FeedHealth, FeedStatus};

    let health = FeedHealth::default();
    assert_eq!(health.status, FeedStatus::Unknown);
    assert!(health.last_success.is_none());
    assert!(health.last_response_time_ms.is_none());
    assert!(health.last_error.is_none());
    assert_eq!(health.consecutive_failures, 0);
}

#[test]
fn test_feed_health_status_indicator() {
    use reedy::app::{FeedHealth, FeedStatus};

    let mut health = FeedHealth::default();

    health.status = FeedStatus::Healthy;
    assert_eq!(health.status_indicator(), "●");

    health.status = FeedStatus::Slow;
    assert_eq!(health.status_indicator(), "◐");

    health.status = FeedStatus::Broken;
    assert_eq!(health.status_indicator(), "✗");

    health.status = FeedStatus::Unknown;
    assert_eq!(health.status_indicator(), "○");
}

#[test]
fn test_feed_health_status_description() {
    use reedy::app::{FeedHealth, FeedStatus};

    let mut health = FeedHealth::default();

    // Healthy with response time
    health.status = FeedStatus::Healthy;
    health.last_response_time_ms = Some(250);
    assert_eq!(health.status_description(), "OK (250ms)");

    // Healthy without response time
    health.last_response_time_ms = None;
    assert_eq!(health.status_description(), "OK");

    // Slow with response time
    health.status = FeedStatus::Slow;
    health.last_response_time_ms = Some(6000);
    assert_eq!(health.status_description(), "Slow (6000ms)");

    // Broken with error
    health.status = FeedStatus::Broken;
    health.last_error = Some("Connection timeout".to_string());
    assert_eq!(health.status_description(), "Error: Connection timeout");

    // Broken without error
    health.last_error = None;
    assert_eq!(health.status_description(), "Broken");

    // Unknown
    health.status = FeedStatus::Unknown;
    assert_eq!(health.status_description(), "Not checked");
}

#[test]
fn test_get_feed_health_unknown_url() {
    let app = App::default();

    // Unknown URL should return default (Unknown status)
    let health = app.get_feed_health("https://unknown.example.com/feed.xml");
    assert_eq!(health.status, reedy::app::FeedStatus::Unknown);
}

#[test]
fn test_app_default_has_empty_feed_health() {
    let app = App::default();
    assert!(app.feed_health.is_empty());
}

#[test]
fn test_notifications_disabled_by_default() {
    let app = App::default();
    // Notifications should be disabled by default
    assert!(!app.config.notifications_enabled);
}

#[test]
fn test_config_notifications_enabled_field() {
    use reedy::app::Config;

    let config = Config::default();
    // Default config should have notifications disabled
    assert!(!config.notifications_enabled);

    // Test that the config can be serialized/deserialized with notifications enabled
    let config_json = r#"{"notifications_enabled": true}"#;
    let parsed_config: Config = serde_json::from_str(config_json).unwrap();
    assert!(parsed_config.notifications_enabled);
}

#[test]
fn test_mark_read_on_scroll_disabled_by_default() {
    let app = App::default();
    // Mark read on scroll should be disabled by default
    assert!(!app.config.mark_read_on_scroll);
}

#[test]
fn test_config_mark_read_on_scroll_field() {
    use reedy::app::Config;

    let config = Config::default();
    // Default config should have mark_read_on_scroll disabled
    assert!(!config.mark_read_on_scroll);

    // Test that the config can be serialized/deserialized with mark_read_on_scroll enabled
    let config_json = r#"{"mark_read_on_scroll": true}"#;
    let parsed_config: Config = serde_json::from_str(config_json).unwrap();
    assert!(parsed_config.mark_read_on_scroll);
}

#[test]
fn test_select_next_marks_read_when_enabled() {
    let mut app = App::default();
    app.config.mark_read_on_scroll = true;
    app.page_mode = PageMode::FeedList;

    // Add some feed items
    let item1 = FeedItem {
        title: "Item 1".to_string(),
        description: "Description 1".to_string(),
        link: "https://example.com/1".to_string(),
        published: Some(SystemTime::now()),
        id: "item-1".to_string(),
        feed_url: String::new(),
    };
    let item2 = FeedItem {
        title: "Item 2".to_string(),
        description: "Description 2".to_string(),
        link: "https://example.com/2".to_string(),
        published: Some(SystemTime::now()),
        id: "item-2".to_string(),
        feed_url: String::new(),
    };

    app.current_feed_content = vec![item1.clone(), item2.clone()];
    app.selected_index = Some(0);

    // Item 1 should not be read initially
    assert!(!app.is_item_read(&item1));

    // Navigate to next item
    app.select_next();

    // Now item 1 should be marked as read (we scrolled past it)
    assert!(app.is_item_read(&item1));
    // Item 2 should still not be read
    assert!(!app.is_item_read(&item2));
    // Selection should be on item 2
    assert_eq!(app.selected_index, Some(1));
}

#[test]
fn test_select_next_does_not_mark_read_when_disabled() {
    let mut app = App::default();
    app.config.mark_read_on_scroll = false; // Disabled (default)
    app.page_mode = PageMode::FeedList;

    // Add some feed items
    let item1 = FeedItem {
        title: "Item 1".to_string(),
        description: "Description 1".to_string(),
        link: "https://example.com/1".to_string(),
        published: Some(SystemTime::now()),
        id: "item-1".to_string(),
        feed_url: String::new(),
    };
    let item2 = FeedItem {
        title: "Item 2".to_string(),
        description: "Description 2".to_string(),
        link: "https://example.com/2".to_string(),
        published: Some(SystemTime::now()),
        id: "item-2".to_string(),
        feed_url: String::new(),
    };

    app.current_feed_content = vec![item1.clone(), item2];
    app.selected_index = Some(0);

    // Navigate to next item
    app.select_next();

    // Item 1 should NOT be marked as read (feature is disabled)
    assert!(!app.is_item_read(&item1));
    // Selection should be on item 2
    assert_eq!(app.selected_index, Some(1));
}

#[test]
fn test_select_next_does_not_mark_read_in_feed_manager() {
    let mut app = App::default();
    app.config.mark_read_on_scroll = true;
    app.page_mode = PageMode::FeedManager; // Should NOT mark read in FeedManager

    // Add some feeds
    app.rss_feeds.push(FeedInfo {
        url: "https://example.com/feed1".to_string(),
        title: "Feed 1".to_string(),
        category: None,
    });
    app.rss_feeds.push(FeedInfo {
        url: "https://example.com/feed2".to_string(),
        title: "Feed 2".to_string(),
        category: None,
    });

    // Also add feed items (for read tracking test)
    let item1 = FeedItem {
        title: "Item 1".to_string(),
        description: "Description 1".to_string(),
        link: "https://example.com/1".to_string(),
        published: Some(SystemTime::now()),
        id: "item-1".to_string(),
        feed_url: String::new(),
    };
    app.current_feed_content = vec![item1.clone()];

    app.selected_index = Some(0);

    // Navigate to next item in FeedManager
    app.select_next();

    // No items should be marked as read (we're in FeedManager mode)
    assert!(!app.is_item_read(&item1));
}


