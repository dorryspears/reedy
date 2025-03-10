use reedy::ui::truncate_text;

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
