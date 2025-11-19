use crate::helpers::connection::ConnectionManager;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

pub enum ConnectionListAction {
    NewConnection,
    SelectConnection(usize),
    DeleteConnection(usize),
}

pub struct ConnectionListPage {
    list_state: ListState,
}

impl ConnectionListPage {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self { list_state }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect, conn_manager: &ConnectionManager) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(area);

        // Title
        let title = Paragraph::new("Database Client - Connection Manager")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Connection list
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

        items.push(ListItem::new("+ Create New Connection").style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Connections"))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, chunks[1], &mut self.list_state);

        // Help text
        let help = Paragraph::new(Line::from(vec![
            Span::raw("↑↓: Navigate | "),
            Span::raw("Enter: Select | "),
            Span::raw("d: Delete | "),
            Span::raw("q: Quit"),
        ]))
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(help, chunks[2]);

        // Ensure valid selection
        let total_items = connections.len() + 1;
        if let Some(selected) = self.list_state.selected() {
            if selected >= total_items {
                self.list_state.select(Some(total_items.saturating_sub(1)));
            }
        }
    }

    pub fn handle_input(&mut self, key: KeyEvent) -> Option<ConnectionListAction> {
        match key.code {
            KeyCode::Up => {
                let i = self.list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.list_state.select(Some(i - 1));
                }
                None
            }
            KeyCode::Down => {
                let i = self.list_state.selected().unwrap_or(0);
                self.list_state.select(Some(i + 1));
                None
            }
            KeyCode::Enter => {
                let selected = self.list_state.selected().unwrap_or(0);
                let connections = ConnectionManager::new().ok()?.load_connections().ok()?;
                
                if selected == connections.len() {
                    Some(ConnectionListAction::NewConnection)
                } else {
                    Some(ConnectionListAction::SelectConnection(selected))
                }
            }
            KeyCode::Char('d') => {
                let selected = self.list_state.selected().unwrap_or(0);
                let connections = ConnectionManager::new().ok()?.load_connections().ok()?;
                
                if selected < connections.len() {
                    Some(ConnectionListAction::DeleteConnection(selected))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}