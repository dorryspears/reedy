use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    prelude::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, InputMode, PageMode, Theme};
use chrono::{DateTime, Local};

/// Converts a color name string to a ratatui Color
pub fn parse_color(color_name: &str) -> Color {
    match color_name.to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "dark_gray" | "dark_grey" | "darkgray" | "darkgrey" => Color::DarkGray,
        "light_red" | "lightred" => Color::LightRed,
        "light_green" | "lightgreen" => Color::LightGreen,
        "light_yellow" | "lightyellow" => Color::LightYellow,
        "light_blue" | "lightblue" => Color::LightBlue,
        "light_magenta" | "lightmagenta" => Color::LightMagenta,
        "light_cyan" | "lightcyan" => Color::LightCyan,
        "white" => Color::White,
        // Support hex colors like "#ff5500" or "ff5500"
        s if s.starts_with('#') && s.len() == 7 => {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&s[1..3], 16),
                u8::from_str_radix(&s[3..5], 16),
                u8::from_str_radix(&s[5..7], 16),
            ) {
                Color::Rgb(r, g, b)
            } else {
                Color::White // fallback
            }
        }
        s if s.len() == 6 => {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&s[0..2], 16),
                u8::from_str_radix(&s[2..4], 16),
                u8::from_str_radix(&s[4..6], 16),
            ) {
                Color::Rgb(r, g, b)
            } else {
                Color::White // fallback
            }
        }
        _ => Color::White, // default fallback
    }
}

/// Helper struct to hold resolved theme colors for rendering
struct ThemeColors {
    primary: Color,
    secondary: Color,
    text: Color,
    muted: Color,
    error: Color,
    highlight: Color,
    description: Color,
    category: Color,
}

impl ThemeColors {
    fn from_theme(theme: &Theme) -> Self {
        Self {
            primary: parse_color(&theme.primary),
            secondary: parse_color(&theme.secondary),
            text: parse_color(&theme.text),
            muted: parse_color(&theme.muted),
            error: parse_color(&theme.error),
            highlight: parse_color(&theme.highlight),
            description: parse_color(&theme.description),
            category: parse_color(&theme.category),
        }
    }
}

/// Renders the user interface widgets.
pub fn render(app: &App, frame: &mut Frame) {
    let colors = ThemeColors::from_theme(&app.config.theme);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.area());

    // Title bar with pixelated book
    let base_title = match app.page_mode {
        PageMode::FeedList => "Reedy",
        PageMode::FeedManager => "Feed Manager",
        PageMode::Favorites => "Favorites",
    };

    // Add auto-refresh indicator if enabled
    let title_text = if let Some(remaining) = app.time_until_next_refresh() {
        let mins = remaining.as_secs() / 60;
        let secs = remaining.as_secs() % 60;
        format!("{} [Auto: {}:{:02}]", base_title, mins, secs)
    } else {
        base_title.to_string()
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
        .style(Style::default().fg(colors.primary))
        .alignment(Alignment::Center);

    let title_para = Paragraph::new(title_text.clone())
        .style(Style::default().fg(colors.primary))
        .alignment(Alignment::Center);

    // Render the title block with border
    let title_block = Block::default().borders(Borders::ALL);
    frame.render_widget(title_block, chunks[0]);

    // Render book and title text inside the block
    frame.render_widget(book_para, title_layout[1]);
    frame.render_widget(
        Paragraph::new(title_text.clone())
            .style(Style::default().fg(colors.primary))
            .alignment(Alignment::Center),
        title_layout[0],
    );
    frame.render_widget(title_para, title_layout[2]);

    // If we're in help mode, render the help menu instead of the regular content
    if app.input_mode == InputMode::Help {
        render_help_menu(app, frame, chunks[1], &colors);
    } else if app.input_mode == InputMode::Preview {
        render_article_preview(app, frame, chunks[1], &colors);
    } else {
        match app.page_mode {
            PageMode::FeedList => render_feed_content(app, frame, chunks[1], &colors),
            PageMode::FeedManager => render_feed_manager(app, frame, chunks[1], &colors),
            PageMode::Favorites => render_feed_content(app, frame, chunks[1], &colors),
        }
    }

    // Render the command bar with our new function
    render_command_bar(app, frame, chunks[2], &colors);
}

fn render_feed_content(app: &App, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
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
                    .fg(colors.secondary)
                    .add_modifier(Modifier::REVERSED)
            } else if app.is_item_read(item) {
                Style::default().fg(colors.muted)
            } else {
                Style::default().fg(colors.text)
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
                    Span::styled(date_str, Style::default().fg(colors.secondary)),
                ]),
                Line::from(vec![
                    Span::raw("   "),
                    Span::styled(truncated_desc, Style::default().fg(colors.description)),
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
        .style(Style::default().fg(colors.text));
    frame.render_widget(list, area);
}

fn render_feed_manager(app: &App, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    // Create a layout that splits the area vertically for the list and error message
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // Feed list takes most space
            Constraint::Length(1), // Status line
        ])
        .split(area);

    // Get feeds grouped by category
    let feeds_by_category = app.get_feeds_by_category();

    // Build list items with category headers
    let mut items: Vec<ListItem> = Vec::new();
    let mut feed_index = 0;

    for (category, feeds) in &feeds_by_category {
        // Add category header
        let header_text = match category {
            Some(cat) => format!("── {} ──", cat),
            None => "── Uncategorized ──".to_string(),
        };
        items.push(
            ListItem::new(Line::from(Span::styled(
                header_text,
                Style::default()
                    .fg(colors.category)
                    .add_modifier(Modifier::BOLD),
            )))
        );

        // Add feeds in this category
        for feed_info in feeds {
            let style = if Some(feed_index) == app.selected_index {
                Style::default()
                    .fg(colors.secondary)
                    .add_modifier(Modifier::REVERSED)
            } else {
                Style::default().fg(colors.text)
            };

            // Get unread count for this feed
            let unread_count = app.count_unread_for_feed(&feed_info.url);
            let total_count = app.count_total_for_feed(&feed_info.url);

            // Format the count display
            let count_display = if total_count > 0 {
                format!(" ({}/{})", unread_count, total_count)
            } else {
                String::new()
            };

            // Calculate max width for title, accounting for count display and indent
            let count_len = count_display.len();
            let title_max_width = chunks[0].width.saturating_sub(12 + count_len as u16) as usize; // Account for index, spacing, indent, and count
            let truncated_title = truncate_text(&feed_info.title, title_max_width as u16);

            // Style for unread count - highlight if there are unread items
            let count_style = if unread_count > 0 {
                Style::default().fg(colors.highlight).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.muted)
            };

            items.push(
                ListItem::new(Line::from(vec![
                    Span::raw(format!("  {}. ", feed_index + 1)),
                    Span::raw(truncated_title),
                    Span::styled(count_display, count_style),
                ]))
                .style(style)
            );
            feed_index += 1;
        }
    }

    let list = List::new(items)
        .block(Block::default().title("RSS Feeds").borders(Borders::ALL))
        .style(Style::default().fg(colors.text));
    frame.render_widget(list, chunks[0]);

    // Render error message if present
    if let Some(error) = &app.error_message {
        let error_text = Line::from(vec![
            Span::styled("Error: ", Style::default().fg(colors.error)),
            Span::styled(error, Style::default().fg(colors.error)),
        ]);
        let paragraph = Paragraph::new(error_text).style(Style::default().fg(colors.error));
        frame.render_widget(paragraph, chunks[1]);
    }
}

/// Renders the help menu with all available commands based on the current page mode
fn render_help_menu(app: &App, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let title = "Help - Available Commands";

    // Create the help text based on the current page mode
    let help_text = match app.page_mode {
        PageMode::FeedList => vec![
            Line::from(vec![Span::styled(
                "Feed List Commands",
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(colors.primary),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Navigation",
                Style::default()
                    .add_modifier(Modifier::UNDERLINED)
                    .fg(colors.secondary),
            )]),
            Line::from("↑/k, ↓/j      - Navigate between feed items"),
            Line::from("PgUp, PgDown   - Scroll page up/down"),
            Line::from("g              - Scroll to top of feed"),
            Line::from("G              - Scroll to bottom of feed"),
            Line::from("Enter          - Read selected feed"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Search",
                Style::default()
                    .add_modifier(Modifier::UNDERLINED)
                    .fg(colors.secondary),
            )]),
            Line::from("/              - Start search/filter"),
            Line::from("Esc            - Clear search filter (when active)"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Actions",
                Style::default()
                    .add_modifier(Modifier::UNDERLINED)
                    .fg(colors.secondary),
            )]),
            Line::from("p              - Open article preview pane"),
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
                    .fg(colors.primary),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Navigation",
                Style::default()
                    .add_modifier(Modifier::UNDERLINED)
                    .fg(colors.secondary),
            )]),
            Line::from("↑/k, ↓/j      - Navigate between feeds"),
            Line::from("g              - Scroll to top of feed list"),
            Line::from("G              - Scroll to bottom of feed list"),
            Line::from("Enter          - Select feed and return to feed list"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Actions",
                Style::default()
                    .add_modifier(Modifier::UNDERLINED)
                    .fg(colors.secondary),
            )]),
            Line::from("a              - Add new feed"),
            Line::from("d              - Delete selected feed"),
            Line::from("t              - Set category/tag for selected feed"),
            Line::from("e              - Export feeds to clipboard"),
            Line::from("E              - Export feeds to OPML file"),
            Line::from("i              - Import feeds from clipboard"),
            Line::from("I              - Import feeds from OPML file"),
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
                    .fg(colors.primary),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Navigation",
                Style::default()
                    .add_modifier(Modifier::UNDERLINED)
                    .fg(colors.secondary),
            )]),
            Line::from("↑/k, ↓/j      - Navigate between favorite items"),
            Line::from("PgUp, PgDown   - Scroll page up/down"),
            Line::from("g              - Scroll to top of feed"),
            Line::from("G              - Scroll to bottom of feed"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Search",
                Style::default()
                    .add_modifier(Modifier::UNDERLINED)
                    .fg(colors.secondary),
            )]),
            Line::from("/              - Start search/filter"),
            Line::from("Esc            - Clear search filter (when active)"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Actions",
                Style::default()
                    .add_modifier(Modifier::UNDERLINED)
                    .fg(colors.secondary),
            )]),
            Line::from("p              - Open article preview pane"),
            Line::from("o              - Open selected item in browser"),
            Line::from("f              - Remove item from favorites"),
            Line::from("F              - Return to all feeds view"),
            Line::from("?              - Toggle this help menu"),
            Line::from("q              - Quit application"),
        ],
    };

    let help_paragraph = Paragraph::new(help_text)
        .block(Block::default().title(title).borders(Borders::ALL))
        .style(Style::default().fg(colors.text));

    frame.render_widget(help_paragraph, area);
}

/// Renders the article preview pane showing full article content
fn render_article_preview(app: &App, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let Some(item) = app.get_preview_item() else {
        // Should not happen, but render empty if no item
        let paragraph = Paragraph::new("No article selected")
            .block(Block::default().title("Article Preview").borders(Borders::ALL))
            .style(Style::default().fg(colors.description));
        frame.render_widget(paragraph, area);
        return;
    };

    // Parse the title to extract article title and feed name
    let (article_title, feed_name) = if let Some(pos) = item.title.rfind(" | ") {
        (&item.title[..pos], &item.title[pos + 3..])
    } else {
        (item.title.as_str(), "")
    };

    // Format the date
    let date_str = item.published.map_or_else(
        || "No date".to_string(),
        |date| {
            let datetime: DateTime<Local> = date.into();
            datetime.format("%Y-%m-%d %H:%M").to_string()
        },
    );

    // Build the content lines
    let mut lines: Vec<Line> = Vec::new();

    // Title section
    lines.push(Line::from(vec![Span::styled(
        article_title,
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(colors.primary),
    )]));
    lines.push(Line::from(""));

    // Metadata section
    if !feed_name.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Feed: ", Style::default().fg(colors.secondary)),
            Span::styled(feed_name, Style::default().fg(colors.text)),
        ]));
    }

    lines.push(Line::from(vec![
        Span::styled("Date: ", Style::default().fg(colors.secondary)),
        Span::styled(date_str, Style::default().fg(colors.text)),
    ]));

    if !item.link.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Link: ", Style::default().fg(colors.secondary)),
            Span::styled(&item.link, Style::default().fg(colors.highlight)),
        ]));
    }

    // Read/Favorite status
    let status = format!(
        "Status: {}{}",
        if app.is_item_read(item) { "[Read] " } else { "[Unread] " },
        if app.is_item_favorite(item) { "★ Favorite" } else { "" }
    );
    lines.push(Line::from(vec![Span::styled(
        status,
        Style::default().fg(colors.description),
    )]));

    // Separator
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "─".repeat(area.width.saturating_sub(4) as usize),
        Style::default().fg(colors.muted),
    )]));
    lines.push(Line::from(""));

    // Content section - wrap the description text
    let content_width = area.width.saturating_sub(4) as usize;
    for line in item.description.lines() {
        // Word wrap each line
        let wrapped = wrap_text(line, content_width);
        for wrapped_line in wrapped {
            lines.push(Line::from(wrapped_line));
        }
    }

    // Calculate total lines and visible area height
    let total_lines = lines.len();
    let visible_height = area.height.saturating_sub(2) as usize; // Account for borders

    // Clamp scroll to prevent scrolling past content
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = (app.preview_scroll as usize).min(max_scroll);

    // Build the title with scroll indicator
    let title = if total_lines > visible_height {
        format!(
            "Article Preview (Line {}/{})",
            scroll + 1,
            total_lines
        )
    } else {
        "Article Preview".to_string()
    };

    let paragraph = Paragraph::new(lines)
        .block(Block::default().title(title).borders(Borders::ALL))
        .style(Style::default().fg(colors.text))
        .scroll((scroll as u16, 0));

    frame.render_widget(paragraph, area);
}

/// Wraps text to fit within a specified width
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![];
    }
    if text.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            // First word on line
            if word.len() > max_width {
                // Word is longer than max width, force break it
                let mut remaining = word;
                while remaining.len() > max_width {
                    lines.push(remaining[..max_width].to_string());
                    remaining = &remaining[max_width..];
                }
                current_line = remaining.to_string();
            } else {
                current_line = word.to_string();
            }
        } else if current_line.len() + 1 + word.len() <= max_width {
            // Word fits on current line
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            // Word doesn't fit, start new line
            lines.push(current_line);
            if word.len() > max_width {
                // Word is longer than max width, force break it
                let mut remaining = word;
                while remaining.len() > max_width {
                    lines.push(remaining[..max_width].to_string());
                    remaining = &remaining[max_width..];
                }
                current_line = remaining.to_string();
            } else {
                current_line = word.to_string();
            }
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
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

fn render_command_bar(app: &App, frame: &mut Frame, area: Rect, colors: &ThemeColors) {
    let commands = if app.input_mode == InputMode::Help {
        "[q/Esc/?] Exit Help".to_string()
    } else if app.input_mode == InputMode::Preview {
        "[↑↓/jk] Scroll  [PgUp/PgDn] Page  [o] Open in Browser  [r] Toggle Read  [f] Toggle Favorite  [Esc/q/p] Close".to_string()
    } else if app.input_mode == InputMode::Searching {
        format!("Search: {}█  [Enter] Confirm  [Esc] Cancel", app.search_query)
    } else {
        match app.page_mode {
            PageMode::FeedList => {
                if app.current_feed_content.is_empty() {
                    "[m] Manage Feeds  [c] Refresh Cache  [F] Favorites  [?] Help  [q] Quit".to_string()
                } else if app.filtered_indices.is_some() {
                    "[↑↓] Navigate  [/] Search  [Esc] Clear Filter  [p] Preview  [o] Open  [f] Favorite  [?] Help  [q] Quit".to_string()
                } else {
                    "[↑↓] Navigate  [/] Search  [p] Preview  [o] Open  [m] Manage  [c] Refresh  [r] Read  [f] Favorite  [F] Favorites  [?] Help  [q] Quit".to_string()
                }
            }
            PageMode::Favorites => {
                if app.current_feed_content.is_empty() {
                    "[F] Back to Feeds  [?] Help  [q] Quit".to_string()
                } else if app.filtered_indices.is_some() {
                    "[↑↓] Navigate  [/] Search  [Esc] Clear Filter  [p] Preview  [o] Open  [f] Favorite  [F] Back  [?] Help  [q] Quit".to_string()
                } else {
                    "[↑↓] Navigate  [/] Search  [p] Preview  [o] Open  [f] Favorite  [F] Back to Feeds  [?] Help  [q] Quit".to_string()
                }
            }
            PageMode::FeedManager => match app.input_mode {
                InputMode::Normal => {
                    "[↑↓] Navigate  [a] Add  [d] Delete  [t] Tag  [e/E] Export  [i/I] Import  [m] Back  [?] Help  [q] Quit".to_string()
                }
                InputMode::Adding => format!("Enter RSS URL: {}", app.input_buffer),
                InputMode::Deleting => {
                    "Use ↑↓ to select feed, Enter to delete, Esc to cancel".to_string()
                }
                InputMode::Importing => {
                    let line_count = app.input_buffer.lines().count();
                    format!("Import feeds ({} URLs pasted) - [Enter] Import  [Esc] Cancel", line_count)
                }
                InputMode::SettingCategory => {
                    format!("Set category: {}█  [Enter] Save  [Esc] Cancel  (empty to remove)", app.input_buffer)
                }
                InputMode::FeedManager => "[m] Back to Feeds  [?] Help".to_string(),
                InputMode::Help | InputMode::Searching | InputMode::Preview => unreachable!(), // These cases are already handled above
            },
        }
    };

    // Truncate the commands to fit in the available width
    let truncated_commands = truncate_text(&commands, area.width.saturating_sub(2));

    let command_bar = Paragraph::new(truncated_commands)
        .style(Style::default().fg(colors.secondary))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Left);

    frame.render_widget(command_bar, area);
}
