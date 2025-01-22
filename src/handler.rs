use crate::app::{App, AppResult, InputMode, PageMode};
use crossterm::event::{KeyCode, KeyEvent};

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
            KeyCode::Up => {
                app.select_previous();
            }
            KeyCode::Down => {
                app.select_next();
            }
            KeyCode::Enter => {
                if let Some(index) = app.selected_index {
                    app.select_feed(index).await?;
                }
            }
            _ => {}
        },
        PageMode::FeedManager => match app.input_mode {
            InputMode::Normal => match key_event.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    app.quit();
                }
                KeyCode::Char('m') => {
                    app.toggle_feed_manager();
                }
                KeyCode::Char('a') => {
                    app.start_adding();
                }
                KeyCode::Char('d') => {
                    app.start_deleting();
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
                KeyCode::Up => {
                    app.select_previous();
                }
                KeyCode::Down => {
                    app.select_next();
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
                KeyCode::Up => {
                    app.select_previous();
                }
                KeyCode::Down => {
                    app.select_next();
                }
                _ => {}
            },
            _ => {}
        },
    }
    Ok(())
}
