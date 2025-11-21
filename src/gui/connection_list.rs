use crate::helpers::connection::ConnectionManager;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

pub enum ConnectionListAction {
    NewConnection,
    SelectConnection(usize),
    DeleteConnection(usize),
    ModifyConnection(usize)
}

pub struct ConnectionListPage {
    pub(crate) list_state: ListState,
}

impl ConnectionListPage {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self { list_state }
    }

    pub fn render(
        &mut self,
        f: &mut Frame,
        area: Rect,
        conn_manager: &ConnectionManager,
        error: &Option<String>,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(if error.is_some() { 5 } else { 3 }),
            ])
            .split(area);

        let title = Paragraph::new("Database Client - Connection Manager")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Lista connessioni
        let connections = conn_manager.load_connections().unwrap_or_default();

        let mut items: Vec<ListItem> = connections
            .iter()
            .enumerate()
            .map(|(i, conn)| {
                let content = format!(
                    "{}. {} ({}) - {}",
                    i + 1,
                    conn.name,
                    conn.db_type,
                    conn.host
                );
                ListItem::new(content)
            })
            .collect();

        items.push(
            ListItem::new("+ Create New Connection").style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        );

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Connections"))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, chunks[1], &mut self.list_state);

        // Help text or error
        let mut help_lines = vec![Line::from(vec![
            Span::raw("↑↓: Navigate | "),
            Span::raw("Enter: Select | "),
            Span::raw("m: Modify | "),
            Span::raw("d: Delete | "),
            Span::raw("Esc: Quit"),
        ])];

        if let Some(err) = error {
            help_lines.push(Line::from(""));
            help_lines.push(Line::from(vec![
                Span::styled(
                    "Error: ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::styled(err, Style::default().fg(Color::Red)),
            ]));
        }

        let help = Paragraph::new(help_lines)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help, chunks[2]);

        // Forza selezione valida
        let total_items = connections.len() + 1;
        if let Some(selected) = self.list_state.selected() {
            if selected >= total_items {
                self.list_state.select(Some(total_items.saturating_sub(1)));
            }
        }
    }
}
