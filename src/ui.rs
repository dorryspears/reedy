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

    match app.page_mode {
        PageMode::FeedList => render_feed_content(app, frame, chunks[1]),
        PageMode::FeedManager => render_feed_manager(app, frame, chunks[1]),
        PageMode::Favorites => render_feed_content(app, frame, chunks[1]),
    }

    // Status bar
    // TODO: ? button to show more yes
    let status_text = match app.page_mode {
        PageMode::FeedList => {
            if app.current_feed_content.is_empty() {
                "[m] Manage Feeds  [c] Refresh Cache  [F] Favorites  [q] Quit".to_string()
            } else {
                "[↑↓] Navigate  [o] Open in Browser  [m] Manage Feeds  [c] Refresh Cache  [r] Mark as Read  [R] Mark All as Read  [f] Toggle Favorite  [F] Favorites  [q] Quit".to_string()
            }
        }
        PageMode::Favorites => {
            if app.current_feed_content.is_empty() {
                "[F] Back to Feeds  [q] Quit".to_string()
            } else {
                "[↑↓] Navigate  [o] Open in Browser  [f] Toggle Favorite  [F] Back to Feeds  [q] Quit".to_string()
            }
        }
        PageMode::FeedManager => match app.input_mode {
            InputMode::Normal => {
                "[a] Add Feed  [d] Delete Feed  [m] Back to Feeds  [q] Quit".to_string()
            }
            InputMode::Adding => format!("Enter RSS URL: {}", app.input_buffer),
            InputMode::Deleting => {
                "Use ↑↓ to select feed, Enter to delete, Esc to cancel".to_string()
            }
            InputMode::FeedManager => "[m] Back to Feeds".to_string(),
        },
    };

    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(status, chunks[2]);
}

fn render_feed_content(app: &App, frame: &mut Frame, area: Rect) {
    let height = area.height as usize;
    let items: Vec<ListItem> = app
        .current_feed_content
        .iter()
        .enumerate()
        .skip(app.scroll as usize)
        .take(height)
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

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(
                    "Feed Content ({}/{})",
                    app.scroll + 1,
                    app.current_feed_content.len()
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
