use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::gui::{QueryPage};

pub fn draw_input_overlay(f: &mut Frame, qpage: &QueryPage) {
    let area = centered_rect(60, 20, f.area());

    f.render_widget(Clear, area);

    let block = Block::default()
        .title("Set Max Rows (0 = unlimited)")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black)
        .fg(Color::Yellow).bold());

    let input = qpage.input_buffer.clone();

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter number: ", Style::default().fg(Color::White).not_bold()),
            Span::styled(input, Style::default().fg(Color::Green).not_bold()),
            Span::styled("█", Style::default().fg(Color::Green).not_bold()),
        ]),
        Line::from(""),
        Line::from(Span::styled("Current: ", Style::default().fg(Color::Gray).not_bold())),
        Line::from(Span::styled(
            if qpage.max_results == 0 { "unlimited".to_string() } else { qpage.max_results.to_string() },
            Style::default().fg(Color::Cyan).not_bold()
        )),
        Line::from(""),
        Line::from(Span::styled("Press Enter to confirm, Esc to cancel", Style::default().fg(Color::White).not_bold())),
    ];

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().bg(Color::Black));

    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}