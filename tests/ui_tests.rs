use reedy::ui::{parse_color, truncate_text};
use ratatui::style::Color;

#[test]
fn test_truncate_text_no_truncation_needed() {
    let text = "Short text";
    let max_width = 20;
    let result = truncate_text(text, max_width);
    assert_eq!(result, text);
}

#[test]
fn test_truncate_text_with_truncation() {
    let text = "This is a very long text that needs truncation";
    let max_width = 15;
    let result = truncate_text(text, max_width);
    assert_eq!(result, "This is a ve...");
    assert_eq!(result.len(), max_width as usize);
}

#[test]
fn test_truncate_text_exact_length() {
    let text = "Exact Length";
    let max_width = 12;
    let result = truncate_text(text, max_width);
    assert_eq!(result, text);
    assert_eq!(result.len(), max_width as usize);
}

#[test]
fn test_truncate_text_unicode() {
    let text = "Unicode: 😀 🌟 🚀";
    let max_width = 10;
    let result = truncate_text(text, max_width);
    // Unicode characters are counted properly
    assert!(result.len() <= (max_width as usize + 3)); // +3 for ellipsis
    assert!(result.ends_with("..."));
}

#[test]
fn test_truncate_text_very_small_width() {
    // Test edge case where max_width < 3 (too small for ellipsis)
    let text = "Hello World";

    // Width of 0 should return empty string
    assert_eq!(truncate_text(text, 0), "");

    // Width of 1 should return first character
    assert_eq!(truncate_text(text, 1), "H");

    // Width of 2 should return first two characters
    assert_eq!(truncate_text(text, 2), "He");

    // Width of 3 should use normal truncation with ellipsis
    assert_eq!(truncate_text(text, 3), "...");
}

#[test]
fn test_parse_color_basic_colors() {
    assert_eq!(parse_color("red"), Color::Red);
    assert_eq!(parse_color("green"), Color::Green);
    assert_eq!(parse_color("blue"), Color::Blue);
    assert_eq!(parse_color("yellow"), Color::Yellow);
    assert_eq!(parse_color("white"), Color::White);
    assert_eq!(parse_color("black"), Color::Black);
    assert_eq!(parse_color("cyan"), Color::Cyan);
    assert_eq!(parse_color("magenta"), Color::Magenta);
    assert_eq!(parse_color("gray"), Color::Gray);
    assert_eq!(parse_color("dark_gray"), Color::DarkGray);
}

#[test]
fn test_parse_color_case_insensitive() {
    assert_eq!(parse_color("RED"), Color::Red);
    assert_eq!(parse_color("Green"), Color::Green);
    assert_eq!(parse_color("BLUE"), Color::Blue);
    assert_eq!(parse_color("DarkGray"), Color::DarkGray);
    assert_eq!(parse_color("DARK_GRAY"), Color::DarkGray);
}

#[test]
fn test_parse_color_light_colors() {
    assert_eq!(parse_color("light_red"), Color::LightRed);
    assert_eq!(parse_color("light_green"), Color::LightGreen);
    assert_eq!(parse_color("light_blue"), Color::LightBlue);
    assert_eq!(parse_color("light_cyan"), Color::LightCyan);
    assert_eq!(parse_color("light_magenta"), Color::LightMagenta);
    assert_eq!(parse_color("light_yellow"), Color::LightYellow);
}

#[test]
fn test_parse_color_hex_with_hash() {
    assert_eq!(parse_color("#ff0000"), Color::Rgb(255, 0, 0));
    assert_eq!(parse_color("#00ff00"), Color::Rgb(0, 255, 0));
    assert_eq!(parse_color("#0000ff"), Color::Rgb(0, 0, 255));
    assert_eq!(parse_color("#ffffff"), Color::Rgb(255, 255, 255));
    assert_eq!(parse_color("#000000"), Color::Rgb(0, 0, 0));
}

#[test]
fn test_parse_color_hex_without_hash() {
    assert_eq!(parse_color("ff0000"), Color::Rgb(255, 0, 0));
    assert_eq!(parse_color("00ff00"), Color::Rgb(0, 255, 0));
    assert_eq!(parse_color("0000ff"), Color::Rgb(0, 0, 255));
}

#[test]
fn test_parse_color_fallback() {
    // Unknown color names should fall back to white
    assert_eq!(parse_color("unknown"), Color::White);
    assert_eq!(parse_color("notacolor"), Color::White);
    // Invalid hex should fall back to white
    assert_eq!(parse_color("#zzzzzz"), Color::White);
}
