use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, InputMode, PageMode};

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
        PageMode::FeedList => "RSS Reader",
        PageMode::FeedManager => "Feed Manager",
    };

    let title = Paragraph::new(title)
        .style(Style::default().fg(Color::Green))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    match app.page_mode {
        PageMode::FeedList => render_feed_content(app, frame, chunks[1]),
        PageMode::FeedManager => render_feed_manager(app, frame, chunks[1]),
    }

    // Status bar
    let status_text = match app.page_mode {
        PageMode::FeedList => {
            if app.current_feed_content.is_empty() {
                "[m] Manage Feeds  [q] Quit".to_string()
            } else {
                "[↑↓] Navigate  [o] Open in Browser  [m] Manage Feeds  [q] Quit".to_string()
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
    let items: Vec<ListItem> = app
        .current_feed_content
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if Some(i) == app.selected_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::REVERSED)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(format!("{}. ", i + 1), style),
                    Span::styled(&item.title, style.add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::raw("   "),
                    Span::styled(&item.description, Style::default().fg(Color::Gray)),
                ]),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().title("Feed Content").borders(Borders::ALL))
        .style(Style::default().fg(Color::White));
    frame.render_widget(list, area);
}

fn render_feed_manager(app: &App, frame: &mut Frame, area: Rect) {
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
    frame.render_widget(list, area);
}
