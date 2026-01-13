use crate::app::{App, AppResult, InputMode, Keybindings, PageMode};
use crossterm::event::{KeyCode, KeyEvent, MouseEvent, MouseEventKind, MouseButton};
use log::{debug, error};

/// Parses a key string (like "Enter", "k", "Up", "PageDown") into a KeyCode.
/// Returns None if the string is not a valid key.
fn parse_key(key_str: &str) -> Option<KeyCode> {
    let key_str = key_str.trim();
    match key_str.to_lowercase().as_str() {
        // Special keys
        "enter" | "return" => Some(KeyCode::Enter),
        "esc" | "escape" => Some(KeyCode::Esc),
        "backspace" => Some(KeyCode::Backspace),
        "tab" => Some(KeyCode::Tab),
        "up" => Some(KeyCode::Up),
        "down" => Some(KeyCode::Down),
        "left" => Some(KeyCode::Left),
        "right" => Some(KeyCode::Right),
        "pageup" | "pgup" => Some(KeyCode::PageUp),
        "pagedown" | "pgdn" | "pgdown" => Some(KeyCode::PageDown),
        "home" => Some(KeyCode::Home),
        "end" => Some(KeyCode::End),
        "insert" => Some(KeyCode::Insert),
        "delete" | "del" => Some(KeyCode::Delete),
        "space" | " " => Some(KeyCode::Char(' ')),
        // Single character - case sensitive for characters
        s if s.len() == 1 => {
            let c = key_str.chars().next()?;
            Some(KeyCode::Char(c))
        }
        _ => None,
    }
}

/// Checks if a key event matches any of the keys in a comma-separated keybinding string.
/// Example: "k,Up" will match either 'k' or Up arrow.
fn key_matches(key_event: &KeyEvent, keybinding: &str) -> bool {
    for key_str in keybinding.split(',') {
        if let Some(key_code) = parse_key(key_str) {
            if key_event.code == key_code {
                return true;
            }
        }
    }
    false
}

/// Helper to get keybindings from app
fn keys(app: &App) -> &Keybindings {
    &app.config.keybindings
}

/// Handles the key events and updates the state of [`App`].
pub async fn handle_key_events(key_event: KeyEvent, app: &mut App) -> AppResult<()> {
    // Handle help mode across all pages first
    if app.input_mode == InputMode::Help {
        // Help mode uses quit, Esc, and help keys to close
        if key_matches(&key_event, &keys(app).quit)
            || key_event.code == KeyCode::Esc
            || key_matches(&key_event, &keys(app).help)
        {
            app.toggle_help();
        }
        return Ok(());
    }

    // Handle search mode - text input, not customizable
    if app.input_mode == InputMode::Searching {
        match key_event.code {
            KeyCode::Enter => {
                app.confirm_search();
            }
            KeyCode::Esc => {
                app.cancel_search();
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
                app.update_search_filter();
            }
            KeyCode::Backspace => {
                app.search_query.pop();
                app.update_search_filter();
            }
            _ => {}
        }
        return Ok(());
    }

    // Handle vi-style command mode
    if app.input_mode == InputMode::Command {
        match key_event.code {
            KeyCode::Enter => {
                if app.execute_command().is_ok() {
                    // Check if we need to toggle favorites (async operation)
                    if app.error_message == Some("__toggle_favorites__".to_string()) {
                        app.error_message = None;
                        app.toggle_favorites_page().await;
                    }
                }
            }
            KeyCode::Esc => {
                app.cancel_command_mode();
            }
            KeyCode::Char(c) => {
                app.command_buffer.push(c);
            }
            KeyCode::Backspace => {
                app.command_buffer.pop();
            }
            _ => {}
        }
        return Ok(());
    }

    // Handle preview mode
    if app.input_mode == InputMode::Preview {
        let kb = keys(app).clone();
        if key_event.code == KeyCode::Esc
            || key_matches(&key_event, &kb.quit)
            || key_matches(&key_event, &kb.open_preview)
        {
            app.close_preview();
        } else if key_matches(&key_event, &kb.move_up) {
            app.preview_scroll_up();
        } else if key_matches(&key_event, &kb.move_down) {
            app.preview_scroll_down();
        } else if key_matches(&key_event, &kb.page_up) {
            app.preview_page_up();
        } else if key_matches(&key_event, &kb.page_down) {
            app.preview_page_down();
        } else if key_matches(&key_event, &kb.open_in_browser) {
            app.open_selected_feed();
        } else if key_matches(&key_event, &kb.toggle_read) {
            app.toggle_read_status();
        } else if key_matches(&key_event, &kb.toggle_favorite) {
            app.toggle_favorite();
        } else if key_matches(&key_event, &kb.scroll_to_top) {
            app.preview_scroll = 0;
        } else if key_matches(&key_event, &kb.scroll_to_bottom) {
            // Set to max value; the UI will cap it to actual content length
            app.preview_scroll = u16::MAX;
        } else if key_matches(&key_event, &kb.export_article) {
            // Export to clipboard with 's', export to file with 'S'
            app.export_article_to_clipboard();
        } else if key_event.code == KeyCode::Char('S') {
            app.export_article_to_file();
        }
        return Ok(());
    }

    // Clone keybindings to avoid borrow issues
    let kb = keys(app).clone();

    match app.page_mode {
        PageMode::FeedList => {
            // Enter vi-style command mode with ':'
            if key_event.code == KeyCode::Char(':') {
                app.start_command_mode();
            } else if key_matches(&key_event, &kb.quit) {
                app.quit();
            } else if key_event.code == KeyCode::Esc {
                // If there's an active search filter, clear it; otherwise quit
                if app.filtered_indices.is_some() {
                    app.clear_search();
                } else {
                    app.quit();
                }
            } else if key_matches(&key_event, &kb.start_search) {
                app.start_search();
            } else if key_matches(&key_event, &kb.toggle_unread_only) {
                app.toggle_unread_only();
            } else if key_matches(&key_event, &kb.open_preview) {
                app.open_preview();
            } else if key_matches(&key_event, &kb.open_feed_manager) {
                app.clear_search(); // Clear search when entering feed manager
                app.toggle_feed_manager();
            } else if key_matches(&key_event, &kb.open_in_browser) {
                app.open_selected_feed();
            } else if key_matches(&key_event, &kb.move_up) {
                app.select_previous();
                app.ensure_selection_visible();
            } else if key_matches(&key_event, &kb.move_down) {
                app.select_next();
                app.ensure_selection_visible();
            } else if key_matches(&key_event, &kb.select) {
                if let Some(index) = app.selected_index {
                    app.select_feed(index).await?;
                }
            } else if key_matches(&key_event, &kb.toggle_read) {
                app.toggle_read_status();
            } else if key_matches(&key_event, &kb.mark_all_read) {
                app.mark_all_as_read();
            } else if key_matches(&key_event, &kb.page_up) {
                app.page_up();
            } else if key_matches(&key_event, &kb.page_down) {
                app.page_down();
            } else if key_matches(&key_event, &kb.scroll_to_top) {
                app.scroll_to_top();
            } else if key_matches(&key_event, &kb.scroll_to_bottom) {
                app.scroll_to_bottom();
            } else if key_matches(&key_event, &kb.refresh) {
                if let Err(e) = app.refresh_all_feeds().await {
                    error!("Failed to refresh feeds: {}", e);
                    app.error_message = Some(format!("Failed to refresh feeds: {}", e));
                } else {
                    app.last_refresh = Some(std::time::SystemTime::now());
                }
            } else if key_matches(&key_event, &kb.toggle_favorite) {
                app.toggle_favorite();
            } else if key_matches(&key_event, &kb.toggle_favorites_view) {
                app.clear_search(); // Clear search when toggling favorites
                app.toggle_favorites_page().await;
            } else if key_matches(&key_event, &kb.export_article) {
                app.export_article_to_clipboard();
            } else if key_event.code == KeyCode::Char('S') {
                app.export_article_to_file();
            } else if key_matches(&key_event, &kb.help) {
                app.toggle_help();
            }
        }
        PageMode::FeedManager => match app.input_mode {
            InputMode::Normal => {
                // Enter vi-style command mode with ':'
                if key_event.code == KeyCode::Char(':') {
                    app.start_command_mode();
                } else if key_matches(&key_event, &kb.quit) || key_event.code == KeyCode::Esc {
                    app.quit();
                } else if key_matches(&key_event, &kb.open_feed_manager) {
                    debug!("Are we logging?");
                    app.toggle_feed_manager();
                } else if key_matches(&key_event, &kb.add_feed) {
                    app.start_adding();
                } else if key_matches(&key_event, &kb.delete_feed) {
                    app.start_deleting();
                } else if key_matches(&key_event, &kb.set_category) {
                    app.start_setting_category();
                } else if key_matches(&key_event, &kb.refresh) {
                    app.cache_all_feeds().await;
                } else if key_matches(&key_event, &kb.export_clipboard) {
                    app.export_feeds_to_clipboard();
                } else if key_matches(&key_event, &kb.export_opml) {
                    // Export to OPML file
                    if let Err(e) = app.export_opml() {
                        error!("Failed to export OPML: {}", e);
                    }
                } else if key_matches(&key_event, &kb.import_clipboard) {
                    app.start_importing();
                } else if key_matches(&key_event, &kb.import_opml) {
                    // Import from OPML file
                    let opml_path = App::get_opml_path();
                    if opml_path.exists() {
                        if let Err(e) = app.import_opml(&opml_path).await {
                            error!("Failed to import OPML: {}", e);
                            app.error_message = Some(format!("Failed to import OPML: {}", e));
                        }
                    } else {
                        app.error_message =
                            Some(format!("OPML file not found: {}", opml_path.display()));
                    }
                } else if key_matches(&key_event, &kb.select) {
                    if let Some(index) = app.selected_index {
                        app.select_feed(index).await?;
                        app.toggle_feed_manager();
                        if !app.current_feed_content.is_empty() {
                            app.selected_index = Some(0);
                            app.scroll = 0; // Reset scroll position
                        }
                    }
                } else if key_matches(&key_event, &kb.move_up) {
                    app.select_previous();
                    app.ensure_selection_visible();
                } else if key_matches(&key_event, &kb.move_down) {
                    app.select_next();
                    app.ensure_selection_visible();
                } else if key_matches(&key_event, &kb.toggle_read) {
                    app.mark_as_read();
                } else if key_matches(&key_event, &kb.mark_all_read) {
                    app.mark_all_as_read();
                } else if key_matches(&key_event, &kb.page_up) {
                    app.scroll_up();
                } else if key_matches(&key_event, &kb.page_down) {
                    app.scroll_down();
                } else if key_matches(&key_event, &kb.scroll_to_top) {
                    app.scroll_to_top();
                } else if key_matches(&key_event, &kb.scroll_to_bottom) {
                    app.scroll_to_bottom();
                } else if key_matches(&key_event, &kb.help) {
                    app.toggle_help();
                }
            }
            // Text input modes - not customizable (except navigation in Deleting mode)
            InputMode::Adding => match key_event.code {
                KeyCode::Enter => {
                    app.add_feed().await?;
                }
                KeyCode::Esc => {
                    app.cancel_adding();
                }
                KeyCode::Char(c) => {
                    app.input_buffer.push(c);
                }
                KeyCode::Backspace => {
                    app.input_buffer.pop();
                }
                _ => {}
            },
            InputMode::Deleting => {
                if key_event.code == KeyCode::Enter {
                    if let Some(index) = app.selected_index {
                        app.delete_feed(index);
                        app.cancel_deleting();
                    }
                } else if key_event.code == KeyCode::Esc {
                    app.cancel_deleting();
                } else if key_matches(&key_event, &kb.move_up) {
                    app.select_previous();
                    app.ensure_selection_visible();
                } else if key_matches(&key_event, &kb.move_down) {
                    app.select_next();
                    app.ensure_selection_visible();
                }
            }
            InputMode::Importing => match key_event.code {
                KeyCode::Enter => {
                    app.import_feeds().await?;
                }
                KeyCode::Esc => {
                    app.cancel_importing();
                }
                KeyCode::Char(c) => {
                    app.input_buffer.push(c);
                }
                KeyCode::Backspace => {
                    app.input_buffer.pop();
                }
                _ => {}
            },
            InputMode::SettingCategory => match key_event.code {
                KeyCode::Enter => {
                    app.set_category();
                }
                KeyCode::Esc => {
                    app.cancel_setting_category();
                }
                KeyCode::Char(c) => {
                    app.input_buffer.push(c);
                }
                KeyCode::Backspace => {
                    app.input_buffer.pop();
                }
                _ => {}
            },
            _ => {}
        },
        PageMode::Favorites => {
            // Enter vi-style command mode with ':'
            if key_event.code == KeyCode::Char(':') {
                app.start_command_mode();
            } else if key_matches(&key_event, &kb.quit) {
                app.quit();
            } else if key_event.code == KeyCode::Esc {
                // If there's an active search filter, clear it; otherwise quit
                if app.filtered_indices.is_some() {
                    app.clear_search();
                } else {
                    app.quit();
                }
            } else if key_matches(&key_event, &kb.start_search) {
                app.start_search();
            } else if key_matches(&key_event, &kb.toggle_unread_only) {
                app.toggle_unread_only();
            } else if key_matches(&key_event, &kb.open_preview) {
                app.open_preview();
            } else if key_matches(&key_event, &kb.open_in_browser) {
                app.open_selected_feed();
            } else if key_matches(&key_event, &kb.move_up) {
                app.select_previous();
                app.ensure_selection_visible();
            } else if key_matches(&key_event, &kb.move_down) {
                app.select_next();
                app.ensure_selection_visible();
            } else if key_matches(&key_event, &kb.toggle_favorite) {
                app.toggle_favorite();
            } else if key_matches(&key_event, &kb.toggle_favorites_view) {
                app.clear_search(); // Clear search when toggling favorites
                app.toggle_favorites_page().await;
            } else if key_matches(&key_event, &kb.page_up) {
                app.page_up();
            } else if key_matches(&key_event, &kb.page_down) {
                app.page_down();
            } else if key_matches(&key_event, &kb.scroll_to_top) {
                app.scroll_to_top();
            } else if key_matches(&key_event, &kb.scroll_to_bottom) {
                app.scroll_to_bottom();
            } else if key_matches(&key_event, &kb.export_article) {
                app.export_article_to_clipboard();
            } else if key_event.code == KeyCode::Char('S') {
                app.export_article_to_file();
            } else if key_matches(&key_event, &kb.help) {
                app.toggle_help();
            }
        }
    }
    Ok(())
}

/// Handles mouse events for navigation and scrolling
pub async fn handle_mouse_events(mouse_event: MouseEvent, app: &mut App) -> AppResult<()> {
    // Ignore mouse events in text input modes
    match app.input_mode {
        InputMode::Adding | InputMode::Importing | InputMode::Searching
        | InputMode::SettingCategory | InputMode::Command => {
            return Ok(());
        }
        _ => {}
    }

    match mouse_event.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            handle_mouse_click(mouse_event.row, app).await?;
        }
        MouseEventKind::ScrollUp => {
            handle_mouse_scroll_up(app);
        }
        MouseEventKind::ScrollDown => {
            handle_mouse_scroll_down(app);
        }
        _ => {}
    }

    Ok(())
}

/// Handles a mouse click at the given row position
async fn handle_mouse_click(row: u16, app: &mut App) -> AppResult<()> {
    // Layout: 3 lines title bar, then content area, then 3 lines command bar
    // Content area starts at row 3 and ends at terminal_height - 3
    let content_start = 3;
    let content_end = app.terminal_height.saturating_sub(3);

    // Check if click is in the content area
    if row < content_start || row >= content_end {
        return Ok(());
    }

    // Calculate the row within the content area (accounting for the border)
    let content_row = row.saturating_sub(content_start + 1); // +1 for border

    // Handle click based on current mode
    match app.input_mode {
        InputMode::Help => {
            // Clicking anywhere in help mode closes it
            app.toggle_help();
        }
        InputMode::Preview => {
            // Clicking in preview mode does nothing special for now
            // Could potentially implement scroll-to-position later
        }
        InputMode::Normal | InputMode::Deleting | InputMode::FeedManager => {
            match app.page_mode {
                PageMode::FeedList | PageMode::Favorites => {
                    // Each feed item takes 3 lines in the list view
                    let item_height = 3;
                    let clicked_item = (content_row / item_height) as usize;
                    let actual_index = app.scroll as usize + clicked_item;

                    // Check if the click is within the valid item range
                    let item_count = app.visible_item_count();
                    if actual_index < item_count {
                        if Some(actual_index) == app.selected_index {
                            // Double-click behavior: if already selected, open preview
                            app.open_preview();
                        } else {
                            app.selected_index = Some(actual_index);
                            debug!("Mouse selected item at index {}", actual_index);
                        }
                    }
                }
                PageMode::FeedManager => {
                    // In Feed Manager, calculate item index accounting for category headers
                    // Each item takes 1 line, but we have category headers interspersed
                    let feeds_by_category = app.get_feeds_by_category();
                    let mut current_row: u16 = 0;
                    let mut feed_index = 0;
                    let scroll_offset = app.scroll as u16;

                    'outer: for (_, feeds) in &feeds_by_category {
                        // Category header takes 1 line
                        if current_row >= scroll_offset && current_row - scroll_offset == content_row {
                            // Clicked on a category header - do nothing
                            break;
                        }
                        current_row += 1;

                        for _ in feeds {
                            if current_row >= scroll_offset && current_row - scroll_offset == content_row {
                                // Clicked on a feed item
                                if feed_index < app.rss_feeds.len() {
                                    if Some(feed_index) == app.selected_index {
                                        // Double-click behavior: if already selected, select the feed
                                        app.select_feed(feed_index).await?;
                                        app.toggle_feed_manager();
                                        if !app.current_feed_content.is_empty() {
                                            app.selected_index = Some(0);
                                            app.scroll = 0;
                                        }
                                    } else {
                                        app.selected_index = Some(feed_index);
                                        debug!("Mouse selected feed at index {}", feed_index);
                                    }
                                }
                                break 'outer;
                            }
                            current_row += 1;
                            feed_index += 1;
                        }
                    }
                }
            }
        }
        _ => {}
    }

    Ok(())
}

/// Handles mouse scroll up event
fn handle_mouse_scroll_up(app: &mut App) {
    match app.input_mode {
        InputMode::Preview => {
            app.preview_scroll_up();
            app.preview_scroll_up();
            app.preview_scroll_up();
        }
        InputMode::Help => {
            // Could implement help scrolling if needed
        }
        InputMode::Normal | InputMode::Deleting | InputMode::FeedManager => {
            // Scroll up by 3 items (matching page up/down behavior)
            for _ in 0..3 {
                app.select_previous();
            }
        }
        _ => {}
    }
}

/// Handles mouse scroll down event
fn handle_mouse_scroll_down(app: &mut App) {
    match app.input_mode {
        InputMode::Preview => {
            app.preview_scroll_down();
            app.preview_scroll_down();
            app.preview_scroll_down();
        }
        InputMode::Help => {
            // Could implement help scrolling if needed
        }
        InputMode::Normal | InputMode::Deleting | InputMode::FeedManager => {
            // Scroll down by 3 items (matching page up/down behavior)
            for _ in 0..3 {
                app.select_next();
            }
        }
        _ => {}
    }
}
