use crate::app::{App, AppResult, InputMode, PageMode};
use crossterm::event::{KeyCode, KeyEvent};
use log::{debug, error};

/// Handles the key events and updates the state of [`App`].
pub async fn handle_key_events(key_event: KeyEvent, app: &mut App) -> AppResult<()> {
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
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.select_next();
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
                app.scroll_up();
            }
            KeyCode::PageDown => {
                app.scroll_down();
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
                        }
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    app.select_previous();
                }
                KeyCode::Down | KeyCode::Char('j') => app.select_next(),
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
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.select_next();
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
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.select_next();
            }
            KeyCode::Char('f') => {
                app.toggle_favorite();
            }
            KeyCode::Char('F') => {
                app.toggle_favorites_page();
            }
            KeyCode::PageUp => {
                app.scroll_up();
            }
            KeyCode::PageDown => {
                app.scroll_down();
            }
            _ => {}
        },
    }
    Ok(())
}
