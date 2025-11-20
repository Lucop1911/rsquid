use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders},
};

#[allow(dead_code)]
pub fn styled_block(title: &str, focused: bool) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(if focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        })
}

#[allow(dead_code)]
pub fn help_line<'a>(items: Vec<(&'a str, &'a str)>) -> Line<'a> {
    let mut spans = Vec::new();
    for (i, (key, desc)) in items.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" | "));
        }
        spans.push(Span::styled(
            *key,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(format!(": {}", desc)));
    }
    Line::from(spans)
}

#[allow(dead_code)]
pub fn error_line(message: &str) -> Line<'_> {
    Line::from(vec![
        Span::styled(
            "Error: ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::styled(message, Style::default().fg(Color::Red)),
    ])
}
