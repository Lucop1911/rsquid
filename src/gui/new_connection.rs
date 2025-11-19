use crate::helpers::connection::Connection;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

pub enum NewConnectionAction {
    Cancel,
    Save(Connection),
}

#[derive(Debug, Clone, PartialEq)]
enum Field {
    Name,
    DbType,
    Host,
    Port,
    Database,
    Username,
    Password,
}

pub struct NewConnectionPage {
    fields: Vec<Field>,
    field_state: ListState,
    name: String,
    db_type: String,
    host: String,
    port: String,
    database: String,
    username: String,
    password: String,
    error: Option<String>,
}

impl NewConnectionPage {
    pub fn new() -> Self {
        let mut field_state = ListState::default();
        field_state.select(Some(0));
        Self {
            fields: vec![
                Field::Name,
                Field::DbType,
                Field::Host,
                Field::Port,
                Field::Database,
                Field::Username,
                Field::Password,
            ],
            field_state,
            name: String::new(),
            db_type: String::from("postgres"),
            host: String::from("localhost"),
            port: String::from("5432"),
            database: String::new(),
            username: String::new(),
            password: String::new(),
            error: None,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(5),
            ])
            .split(area);

        // Title
        let title = Paragraph::new("New Database Connection")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Form fields
        let items: Vec<ListItem> = vec![
            ListItem::new(format!("Name: {}", self.name)),
            ListItem::new(format!("Database Type: {} (postgres/mysql/sqlite)", self.db_type)),
            ListItem::new(format!("Host: {}", self.host)),
            ListItem::new(format!("Port: {}", self.port)),
            ListItem::new(format!("Database: {}", self.database)),
            ListItem::new(format!("Username: {}", self.username)),
            ListItem::new(format!("Password: {}", "*".repeat(self.password.len()))),
        ];

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Connection Details"))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, chunks[1], &mut self.field_state);

        // Help and error
        let mut help_lines = vec![
            Line::from(vec![
                Span::raw("↑↓: Navigate | "),
                Span::raw("Type: Edit | "),
                Span::raw("Ctrl+S: Save | "),
                Span::raw("Esc: Cancel"),
            ]),
        ];

        if let Some(err) = &self.error {
            help_lines.push(Line::from(vec![
                Span::styled("Error: ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::styled(err, Style::default().fg(Color::Red)),
            ]));
        }

        let help = Paragraph::new(help_lines)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help, chunks[2]);
    }

    pub fn handle_input(&mut self, key: KeyEvent) -> Option<NewConnectionAction> {
        self.error = None;

        match key.code {
            KeyCode::Up => {
                let i = self.field_state.selected().unwrap_or(0);
                if i > 0 {
                    self.field_state.select(Some(i - 1));
                }
                None
            }
            KeyCode::Down => {
                let i = self.field_state.selected().unwrap_or(0);
                if i < self.fields.len() - 1 {
                    self.field_state.select(Some(i + 1));
                }
                None
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.validate_and_save()
            }
            KeyCode::Esc => Some(NewConnectionAction::Cancel),
            KeyCode::Char(c) => {
                let selected = self.field_state.selected().unwrap_or(0);
                match self.fields[selected] {
                    Field::Name => self.name.push(c),
                    Field::DbType => self.db_type.push(c),
                    Field::Host => self.host.push(c),
                    Field::Port => self.port.push(c),
                    Field::Database => self.database.push(c),
                    Field::Username => self.username.push(c),
                    Field::Password => self.password.push(c),
                }
                None
            }
            KeyCode::Backspace => {
                let selected = self.field_state.selected().unwrap_or(0);
                match self.fields[selected] {
                    Field::Name => { self.name.pop(); },
                    Field::DbType => { self.db_type.pop(); },
                    Field::Host => { self.host.pop(); },
                    Field::Port => { self.port.pop(); },
                    Field::Database => { self.database.pop(); },
                    Field::Username => { self.username.pop(); },
                    Field::Password => { self.password.pop(); },
                }
                None
            }
            _ => None,
        }
    }

    fn validate_and_save(&mut self) -> Option<NewConnectionAction> {
        if self.name.is_empty() {
            self.error = Some("Name is required".to_string());
            return None;
        }
        if !["postgres", "mysql", "sqlite"].contains(&self.db_type.as_str()) {
            self.error = Some("Invalid database type".to_string());
            return None;
        }
        if self.host.is_empty() {
            self.error = Some("Host is required".to_string());
            return None;
        }

        let conn = Connection {
            name: self.name.clone(),
            db_type: self.db_type.clone(),
            host: self.host.clone(),
            port: self.port.parse().unwrap_or(5432),
            database: self.database.clone(),
            username: self.username.clone(),
            password: self.password.clone(),
        };

        Some(NewConnectionAction::Save(conn))
    }
}