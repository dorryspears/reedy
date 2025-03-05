use crate::app::{App, AppResult, InputMode, PageMode};
use crossterm::event::{KeyCode, KeyEvent};
use log::{debug, error};

/// Handles the key events and updates the state of [`App`].
pub async fn handle_key_events(key_event: KeyEvent, app: &mut App) -> AppResult<()> {
    // Handle help mode across all pages first
    if app.input_mode == InputMode::Help {
        match key_event.code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('?') => {
                app.toggle_help();
                return Ok(());
            }
            _ => return Ok(()),
        }
    }

    match app.page_mode {
        PageMode::FeedList => match key_event.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                app.quit();
            }
            KeyCode::Char('m') => {
                app.toggle_feed_manager();
            }
            KeyCode::Char('o') => {
                app.open_selected_feed();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.select_previous();
                // Using our centralized method to ensure selection is visible
                app.ensure_selection_visible();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.select_next();
                // Using our centralized method to ensure selection is visible
                app.ensure_selection_visible();
            }
            KeyCode::Enter => {
                if let Some(index) = app.selected_index {
                    app.select_feed(index).await?;
                }
            }
            KeyCode::Char('r') => {
                app.toggle_read_status();
            }
            KeyCode::Char('R') => {
                app.mark_all_as_read();
            }
            KeyCode::PageUp => {
                app.page_up();
            }
            KeyCode::PageDown => {
                app.page_down();
            }
            KeyCode::Char('g') => {
                app.scroll_to_top();
            }
            KeyCode::Char('c') => {
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        if let Err(e) = app.refresh_all_feeds().await {
                            error!("Failed to refresh feeds: {}", e);
                            app.error_message = Some(format!("Failed to refresh feeds: {}", e));
                        }
                    });
                });
            }
            KeyCode::Char('f') => {
                app.toggle_favorite();
            }
            KeyCode::Char('F') => {
                app.toggle_favorites_page();
            }
            KeyCode::Char('?') => {
                app.toggle_help();
            }
            _ => {}
        },
        PageMode::FeedManager => match app.input_mode {
            InputMode::Normal => match key_event.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    app.quit();
                }
                KeyCode::Char('m') => {
                    debug!("Are we logging?");
                    app.toggle_feed_manager();
                }
                KeyCode::Char('a') => {
                    app.start_adding();
                }
                KeyCode::Char('d') => {
                    app.start_deleting();
                }
                KeyCode::Char('c') => {
                    tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async {
                            app.cache_all_feeds().await;
                        });
                    });
                }
                KeyCode::Enter => {
                    if let Some(index) = app.selected_index {
                        app.select_feed(index).await?;
                        app.toggle_feed_manager();
                        if !app.current_feed_content.is_empty() {
                            app.selected_index = Some(0);
                            app.scroll = 0; // Reset scroll position
                        }
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    app.select_previous();
                    // Ensure selected item is visible
                    app.ensure_selection_visible();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.select_next();
                    // Ensure selected item is visible
                    app.ensure_selection_visible();
                },
                KeyCode::Char('r') => {
                    app.mark_as_read();
                }
                KeyCode::Char('R') => {
                    app.mark_all_as_read();
                }
                KeyCode::PageUp => {
                    app.scroll_up();
                }
                KeyCode::PageDown => {
                    app.scroll_down();
                }
                KeyCode::Char('g') => {
                    app.scroll_to_top();
                }
                KeyCode::Char('?') => {
                    app.toggle_help();
                }
                _ => {}
            },
            InputMode::Adding => match key_event.code {
                KeyCode::Enter => {
                    app.add_feed().await?;
                }
                KeyCode::Char('q') | KeyCode::Esc => {
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
            InputMode::Deleting => match key_event.code {
                KeyCode::Enter => {
                    if let Some(index) = app.selected_index {
                        app.delete_feed(index);
                        app.cancel_deleting();
                    }
                }
                KeyCode::Esc => {
                    app.cancel_deleting();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    app.select_previous();
                    app.ensure_selection_visible();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.select_next();
                    app.ensure_selection_visible();
                }
                _ => {}
            },
            _ => {}
        },
        PageMode::Favorites => match key_event.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                app.quit();
            }
            KeyCode::Char('o') => {
                app.open_selected_feed();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.select_previous();
                // Using our centralized method to ensure selection is visible
                app.ensure_selection_visible();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.select_next();
                // Using our centralized method to ensure selection is visible
                app.ensure_selection_visible();
            }
            KeyCode::Char('f') => {
                app.toggle_favorite();
            }
            KeyCode::Char('F') => {
                app.toggle_favorites_page();
            }
            KeyCode::PageUp => {
                app.page_up();
            }
            KeyCode::PageDown => {
                app.page_down();
            }
            KeyCode::Char('g') => {
                app.scroll_to_top();
            }
            KeyCode::Char('?') => {
                app.toggle_help();
            }
            _ => {}
        },
    }
    Ok(())
}
