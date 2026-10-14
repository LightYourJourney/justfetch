use anstyle::{AnsiColor, Color, Style};

/// v0.1 built-in styling, kept independent of data collection.
/// Empty text stays empty: padding rows must not emit an empty styled span.
pub fn accent(text: &str, color: AnsiColor, enabled: bool) -> String {
    if !enabled || text.is_empty() {
        return text.to_owned();
    }
    let style = Style::new().fg_color(Some(Color::Ansi(color))).bold();
    format!("{style}{text}{style:#}")
}
