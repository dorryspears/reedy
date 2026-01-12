use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
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

    // Title bar with pixelated book
    let title_text = match app.page_mode {
        PageMode::FeedList => "Reedy",
        PageMode::FeedManager => "Feed Manager",
        PageMode::Favorites => "Favorites",
    };

    // Create a layout for the title area to position the book icon and title text
    let title_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35), // Left space
            Constraint::Percentage(30), // Center for book
            Constraint::Percentage(35), // Right space
        ])
        .split(chunks[0]);

    // Pixelated book ASCII art
    let book_art = vec![
        Line::from("   ┌─────┐  "),
        Line::from("  ┌│░░░░░│┐ "),
        Line::from(" ┌││░░░░░││┐"),
        Line::from(" ││└─────┘││"),
        Line::from(" └└───────┘┘"),
    ];

    let book_para = Paragraph::new(book_art)
        .style(Style::default().fg(Color::Green))
        .alignment(Alignment::Center);

    let title_para = Paragraph::new(title_text)
        .style(Style::default().fg(Color::Green))
        .alignment(Alignment::Center);

    // Render the title block with border
    let title_block = Block::default().borders(Borders::ALL);
    frame.render_widget(title_block, chunks[0]);

    // Render book and title text inside the block
    frame.render_widget(book_para, title_layout[1]);
    frame.render_widget(title_para.clone(), title_layout[0]);
    frame.render_widget(title_para, title_layout[2]);

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

    // Render the command bar with our new function
    render_command_bar(app, frame, chunks[2]);
}

fn render_feed_content(app: &App, frame: &mut Frame, area: Rect) {
    // Calculate how many items can fit per page (each item takes 3 lines plus a separator)
    let items_per_page = ((area.height as usize).saturating_sub(2) / 3).max(1); // Ensure at least 1 item per page

    // Get visible items (filtered or all)
    let visible_items = app.get_visible_items();
    let total_visible = visible_items.len();
    let total_items = app.current_feed_content.len();

    // Calculate the visible range for items
    let start_idx = app.scroll as usize;
    let end_idx = (start_idx + items_per_page).min(total_visible);

    let items: Vec<ListItem> = visible_items
        .iter()
        .enumerate()
        .skip(start_idx)
        .take(items_per_page)
        .map(|(visible_idx, (_actual_idx, item))| {
            let style = if Some(visible_idx) == app.selected_index {
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

            // Calculate max width for title and description
            let title_max_width = area.width.saturating_sub(10) as usize; // Account for favorite icon, read status and spacing
            let desc_max_width = area.width.saturating_sub(6) as usize; // Account for indentation and borders

            // Truncate title and description
            let truncated_title = truncate_text(&item.title, title_max_width as u16);
            let truncated_desc = truncate_text(&item.description, desc_max_width as u16);

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(favorite_indicator, style),
                    Span::styled(
                        format!("[{}] ", if app.is_item_read(item) { "✓" } else { " " }),
                        style,
                    ),
                    Span::styled(truncated_title, style.add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::raw("   "),
                    Span::styled(date_str, Style::default().fg(Color::Yellow)),
                ]),
                Line::from(vec![
                    Span::raw("   "),
                    Span::styled(truncated_desc, Style::default().fg(Color::Gray)),
                ]),
            ])
        })
        .collect();

    // Create a title that shows both the current page and total pages
    let page_count = if total_visible == 0 {
        1
    } else {
        // Ceiling division
        total_visible.div_ceil(items_per_page)
    };

    let current_page = if total_visible == 0 {
        1
    } else {
        (start_idx / items_per_page) + 1
    };

    // Build title with filter indicator if active
    let title = if app.filtered_indices.is_some() {
        format!(
            "Feed Content [Filter: \"{}\"] (Page {}/{}, Items {}-{}/{} of {})",
            app.search_query,
            current_page,
            page_count,
            if total_visible == 0 { 0 } else { start_idx + 1 },
            end_idx,
            total_visible,
            total_items
        )
    } else {
        format!(
            "Feed Content (Page {}/{}, Items {}-{}/{})",
            current_page,
            page_count,
            if total_visible == 0 { 0 } else { start_idx + 1 },
            end_idx,
            total_visible
        )
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
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

            // Calculate max width for URL
            let url_max_width = chunks[0].width.saturating_sub(8) as usize; // Account for index and spacing
            let truncated_url = truncate_text(url, url_max_width as u16);

            ListItem::new(Line::from(vec![
                Span::raw(format!("{}. ", i + 1)),
                Span::raw(truncated_url),
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
            Line::from(vec![Span::styled(
                "Feed List Commands",
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Green),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Navigation",
                Style::default()
                    .add_modifier(Modifier::UNDERLINED)
                    .fg(Color::Yellow),
            )]),
            Line::from("↑/k, ↓/j      - Navigate between feed items"),
            Line::from("PgUp, PgDown   - Scroll page up/down"),
            Line::from("g              - Scroll to top of feed"),
            Line::from("Enter          - Read selected feed"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Search",
                Style::default()
                    .add_modifier(Modifier::UNDERLINED)
                    .fg(Color::Yellow),
            )]),
            Line::from("/              - Start search/filter"),
            Line::from("Esc            - Clear search filter (when active)"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Actions",
                Style::default()
                    .add_modifier(Modifier::UNDERLINED)
                    .fg(Color::Yellow),
            )]),
            Line::from("o              - Open selected item in browser"),
            Line::from("r              - Toggle read status of selected item"),
            Line::from("R              - Mark all items as read"),
            Line::from("f              - Toggle favorite status of selected item"),
            Line::from("F              - Toggle favorites view"),
            Line::from("m              - Open feed manager"),
            Line::from("c              - Refresh feed cache"),
            Line::from("?              - Toggle this help menu"),
            Line::from("q              - Quit application"),
        ],
        PageMode::FeedManager => vec![
            Line::from(vec![Span::styled(
                "Feed Manager Commands",
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Green),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Navigation",
                Style::default()
                    .add_modifier(Modifier::UNDERLINED)
                    .fg(Color::Yellow),
            )]),
            Line::from("↑/k, ↓/j      - Navigate between feeds"),
            Line::from("g              - Scroll to top of feed list"),
            Line::from("Enter          - Select feed and return to feed list"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Actions",
                Style::default()
                    .add_modifier(Modifier::UNDERLINED)
                    .fg(Color::Yellow),
            )]),
            Line::from("a              - Add new feed"),
            Line::from("d              - Delete selected feed"),
            Line::from("c              - Refresh feed cache"),
            Line::from("m              - Return to feed list"),
            Line::from("?              - Toggle this help menu"),
            Line::from("q/Esc          - Quit application"),
        ],
        PageMode::Favorites => vec![
            Line::from(vec![Span::styled(
                "Favorites View Commands",
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Green),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Navigation",
                Style::default()
                    .add_modifier(Modifier::UNDERLINED)
                    .fg(Color::Yellow),
            )]),
            Line::from("↑/k, ↓/j      - Navigate between favorite items"),
            Line::from("PgUp, PgDown   - Scroll page up/down"),
            Line::from("g              - Scroll to top of feed"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Search",
                Style::default()
                    .add_modifier(Modifier::UNDERLINED)
                    .fg(Color::Yellow),
            )]),
            Line::from("/              - Start search/filter"),
            Line::from("Esc            - Clear search filter (when active)"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Actions",
                Style::default()
                    .add_modifier(Modifier::UNDERLINED)
                    .fg(Color::Yellow),
            )]),
            Line::from("o              - Open selected item in browser"),
            Line::from("f              - Remove item from favorites"),
            Line::from("F              - Return to all feeds view"),
            Line::from("?              - Toggle this help menu"),
            Line::from("q              - Quit application"),
        ],
    };

    let help_paragraph = Paragraph::new(help_text)
        .block(Block::default().title(title).borders(Borders::ALL))
        .style(Style::default().fg(Color::White));

    frame.render_widget(help_paragraph, area);
}

/// Truncates text to fit within a specified width, adding ellipsis if necessary
pub fn truncate_text(text: &str, max_width: u16) -> String {
    // Handle edge case where max_width is too small for ellipsis
    if max_width < 3 {
        return text.chars().take(max_width as usize).collect();
    }
    if text.len() <= max_width as usize {
        text.to_string()
    } else {
        let mut truncated = text
            .chars()
            .take((max_width - 3) as usize)
            .collect::<String>();
        truncated.push_str("...");
        truncated
    }
}

fn render_command_bar(app: &App, frame: &mut Frame, area: Rect) {
    let commands = if app.input_mode == InputMode::Help {
        "[q/Esc/?] Exit Help".to_string()
    } else if app.input_mode == InputMode::Searching {
        format!("Search: {}█  [Enter] Confirm  [Esc] Cancel", app.search_query)
    } else {
        match app.page_mode {
            PageMode::FeedList => {
                if app.current_feed_content.is_empty() {
                    "[m] Manage Feeds  [c] Refresh Cache  [F] Favorites  [?] Help  [q] Quit".to_string()
                } else if app.filtered_indices.is_some() {
                    "[↑↓] Navigate  [/] Search  [Esc] Clear Filter  [o] Open  [f] Favorite  [?] Help  [q] Quit".to_string()
                } else {
                    "[↑↓] Navigate  [/] Search  [o] Open  [m] Manage  [c] Refresh  [r] Read  [f] Favorite  [F] Favorites  [?] Help  [q] Quit".to_string()
                }
            }
            PageMode::Favorites => {
                if app.current_feed_content.is_empty() {
                    "[F] Back to Feeds  [?] Help  [q] Quit".to_string()
                } else if app.filtered_indices.is_some() {
                    "[↑↓] Navigate  [/] Search  [Esc] Clear Filter  [o] Open  [f] Favorite  [F] Back  [?] Help  [q] Quit".to_string()
                } else {
                    "[↑↓] Navigate  [/] Search  [o] Open  [f] Favorite  [F] Back to Feeds  [?] Help  [q] Quit".to_string()
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
                InputMode::Help | InputMode::Searching => unreachable!(), // These cases are already handled above
            },
        }
    };

    // Truncate the commands to fit in the available width
    let truncated_commands = truncate_text(&commands, area.width.saturating_sub(2));

    let command_bar = Paragraph::new(truncated_commands)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Left);

    frame.render_widget(command_bar, area);
}
