use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, InputMode, PageMode};
use chrono::{DateTime, Local};

/// Renders the user interface widgets.
pub fn render(app: &App, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.area());

    // Title bar
    let title = match app.page_mode {
        PageMode::FeedList => "Reedy",
        PageMode::FeedManager => "Feed Manager",
        PageMode::Favorites => "Favorites",
    };

    let title = Paragraph::new(title)
        .style(Style::default().fg(Color::Green))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // If we're in help mode, render the help menu instead of the regular content
    if app.input_mode == InputMode::Help {
        render_help_menu(app, frame, chunks[1]);
    } else {
        match app.page_mode {
            PageMode::FeedList => render_feed_content(app, frame, chunks[1]),
            PageMode::FeedManager => render_feed_manager(app, frame, chunks[1]),
            PageMode::Favorites => render_feed_content(app, frame, chunks[1]),
        }
    }

    // Status bar
    let status_text = if app.input_mode == InputMode::Help {
        "[q/Esc/?] Exit Help".to_string()
    } else {
        match app.page_mode {
            PageMode::FeedList => {
                if app.current_feed_content.is_empty() {
                    "[m] Manage Feeds  [c] Refresh Cache  [F] Favorites  [?] Help  [q] Quit".to_string()
                } else {
                    "[↑↓] Navigate  [g] Top  [o] Open in Browser  [m] Manage Feeds  [c] Refresh Cache  [r] Mark as Read  [R] Mark All as Read  [f] Toggle Favorite  [F] Favorites  [?] Help  [q] Quit".to_string()
                }
            }
            PageMode::Favorites => {
                if app.current_feed_content.is_empty() {
                    "[F] Back to Feeds  [?] Help  [q] Quit".to_string()
                } else {
                    "[↑↓] Navigate  [g] Top  [o] Open in Browser  [f] Toggle Favorite  [F] Back to Feeds  [?] Help  [q] Quit".to_string()
                }
            }
            PageMode::FeedManager => match app.input_mode {
                InputMode::Normal => {
                    "[↑↓] Navigate  [g] Top  [a] Add Feed  [d] Delete Feed  [m] Back to Feeds  [?] Help  [q] Quit".to_string()
                }
                InputMode::Adding => format!("Enter RSS URL: {}", app.input_buffer),
                InputMode::Deleting => {
                    "Use ↑↓ to select feed, Enter to delete, Esc to cancel".to_string()
                }
                InputMode::FeedManager => "[m] Back to Feeds  [?] Help".to_string(),
                InputMode::Help => unreachable!(), // This case is already handled above
            },
        }
    };

    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(status, chunks[2]);
}

fn render_feed_content(app: &App, frame: &mut Frame, area: Rect) {
    // Calculate how many items can fit per page (each item takes 3 lines plus a separator)
    let items_per_page = (area.height as usize).saturating_sub(2) / 3;
    let total_items = app.current_feed_content.len();
    
    // Calculate the visible range for items
    let start_idx = app.scroll as usize;
    let end_idx = (start_idx + items_per_page).min(total_items);
    
    let items: Vec<ListItem> = app
        .current_feed_content
        .iter()
        .enumerate()
        .skip(start_idx)
        .take(items_per_page)
        .map(|(i, item)| {
            let style = if Some(i) == app.selected_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::REVERSED)
            } else if app.is_item_read(item) {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };

            let favorite_indicator = if app.is_item_favorite(item) {
                "★ "
            } else {
                "  "
            };

            let date_str = item.published.map_or_else(
                || "No date".to_string(),
                |date| {
                    let datetime: DateTime<Local> = date.into();
                    datetime.format("%Y-%m-%d %H:%M").to_string()
                },
            );

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!("{}", favorite_indicator), style),
                    Span::styled(
                        format!("[{}] ", if app.is_item_read(item) { "✓" } else { " " }),
                        style,
                    ),
                    Span::styled(&item.title, style.add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::raw("   "),
                    Span::styled(date_str, Style::default().fg(Color::Yellow)),
                ]),
                Line::from(vec![
                    Span::raw("   "),
                    Span::styled(&item.description, Style::default().fg(Color::Gray)),
                ]),
            ])
        })
        .collect();

    // Create a title that shows both the current page and total pages
    let page_count = if total_items == 0 {
        1
    } else {
        (total_items + items_per_page - 1) / items_per_page
    };
    
    let current_page = if total_items == 0 {
        1
    } else {
        (start_idx / items_per_page) + 1
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(
                    "Feed Content (Page {}/{}, Items {}-{}/{})",
                    current_page,
                    page_count,
                    if total_items == 0 { 0 } else { start_idx + 1 },
                    end_idx,
                    total_items
                ))
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::White));
    frame.render_widget(list, area);
}

fn render_feed_manager(app: &App, frame: &mut Frame, area: Rect) {
    // Create a layout that splits the area vertically for the list and error message
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // Feed list takes most space
            Constraint::Length(1), // Status line
        ])
        .split(area);

    // Render the feed list
    let items: Vec<ListItem> = app
        .rss_feeds
        .iter()
        .enumerate()
        .map(|(i, url)| {
            let style = if Some(i) == app.selected_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::REVERSED)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(Line::from(vec![
                Span::raw(format!("{}. ", i + 1)),
                Span::raw(url),
            ]))
            .style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().title("RSS Feeds").borders(Borders::ALL))
        .style(Style::default().fg(Color::White));
    frame.render_widget(list, chunks[0]);

    // Render error message if present
    if let Some(error) = &app.error_message {
        let error_text = Line::from(vec![
            Span::styled("Error: ", Style::default().fg(Color::Red)),
            Span::styled(error, Style::default().fg(Color::Red)),
        ]);
        let paragraph = Paragraph::new(error_text).style(Style::default().fg(Color::Red));
        frame.render_widget(paragraph, chunks[1]);
    }
}

/// Renders the help menu with all available commands based on the current page mode
fn render_help_menu(app: &App, frame: &mut Frame, area: Rect) {
    let title = "Help - Available Commands";
    
    // Create the help text based on the current page mode
    let help_text = match app.page_mode {
        PageMode::FeedList => vec![
            Line::from(vec![
                Span::styled("Feed List Commands", Style::default().add_modifier(Modifier::BOLD).fg(Color::Green))
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Navigation", Style::default().add_modifier(Modifier::UNDERLINED).fg(Color::Yellow))
            ]),
            Line::from("↑/k, ↓/j      - Navigate between feed items"),
            Line::from("PgUp, PgDown   - Scroll page up/down"),
            Line::from("g              - Scroll to top of feed"),
            Line::from("Enter          - Read selected feed"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Actions", Style::default().add_modifier(Modifier::UNDERLINED).fg(Color::Yellow))
            ]),
            Line::from("o              - Open selected item in browser"),
            Line::from("r              - Toggle read status of selected item"),
            Line::from("R              - Mark all items as read"),
            Line::from("f              - Toggle favorite status of selected item"),
            Line::from("F              - Toggle favorites view"),
            Line::from("m              - Open feed manager"),
            Line::from("c              - Refresh feed cache"),
            Line::from("?              - Toggle this help menu"),
            Line::from("q/Esc          - Quit application"),
        ],
        PageMode::FeedManager => vec![
            Line::from(vec![
                Span::styled("Feed Manager Commands", Style::default().add_modifier(Modifier::BOLD).fg(Color::Green))
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Navigation", Style::default().add_modifier(Modifier::UNDERLINED).fg(Color::Yellow))
            ]),
            Line::from("↑/k, ↓/j      - Navigate between feeds"),
            Line::from("g              - Scroll to top of feed list"),
            Line::from("Enter          - Select feed and return to feed list"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Actions", Style::default().add_modifier(Modifier::UNDERLINED).fg(Color::Yellow))
            ]),
            Line::from("a              - Add new feed"),
            Line::from("d              - Delete selected feed"),
            Line::from("c              - Refresh feed cache"),
            Line::from("m              - Return to feed list"),
            Line::from("?              - Toggle this help menu"),
            Line::from("q/Esc          - Quit application"),
        ],
        PageMode::Favorites => vec![
            Line::from(vec![
                Span::styled("Favorites View Commands", Style::default().add_modifier(Modifier::BOLD).fg(Color::Green))
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Navigation", Style::default().add_modifier(Modifier::UNDERLINED).fg(Color::Yellow))
            ]),
            Line::from("↑/k, ↓/j      - Navigate between favorite items"),
            Line::from("PgUp, PgDown   - Scroll page up/down"),
            Line::from("g              - Scroll to top of feed"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Actions", Style::default().add_modifier(Modifier::UNDERLINED).fg(Color::Yellow))
            ]),
            Line::from("o              - Open selected item in browser"),
            Line::from("f              - Remove item from favorites"),
            Line::from("F              - Return to all feeds view"),
            Line::from("?              - Toggle this help menu"),
            Line::from("q/Esc          - Quit application"),
        ],
    };
    
    let help_paragraph = Paragraph::new(help_text)
        .block(Block::default().title(title).borders(Borders::ALL))
        .style(Style::default().fg(Color::White));
    
    frame.render_widget(help_paragraph, area);
}
