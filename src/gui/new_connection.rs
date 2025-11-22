use crate::helpers::connection::Connection;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

pub enum NewConnectionAction {
    Cancel,
    Save(Connection),
    Update(usize, Connection),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Field {
    Name,
    DbType,
    Host,
    Port,
    Database,
    Username,
    Password,
}

pub struct NewConnectionPage {
    pub(crate) fields: Vec<Field>,
    pub(crate) field_state: ListState,
    pub(crate) name: String,
    pub(crate) db_type: String,
    pub(crate) host: String,
    pub(crate) port: String,
    pub(crate) database: String,
    pub(crate) username: String,
    pub(crate) password: String,
    pub(crate) error: Option<String>,
    pub(crate) modifying_index: Option<usize>,
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
            modifying_index: None,
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
        let title_text = if self.modifying_index.is_some() {
            "Modify Database Connection"
        } else {
            "New Database Connection"
        };

        let title = Paragraph::new(title_text)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(title, chunks[0]);

        // Form fields
        let items: Vec<ListItem> = vec![
            ListItem::new(format!("Name: {}", self.name)),
            ListItem::new(format!(
                "Database Type: {} (postgres/mysql/sqlite)",
                self.db_type
            )),
            ListItem::new(format!("Host: {}", self.host)),
            ListItem::new(format!("Port: {}", self.port)),
            ListItem::new(format!("Database: {}", self.database)),
            ListItem::new(format!("Username: {}", self.username)),
            ListItem::new(format!("Password: {}", "*".repeat(self.password.len()))),
        ];
        
        let highlight = {
            #[cfg(target_os = "windows")]
            {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            }

            #[cfg(not(target_os = "windows"))]
            {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            }
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Connection Details"),
            )
            .highlight_style(highlight)
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, chunks[1], &mut self.field_state);

        // Help and error
        let mut help_lines = vec![Line::from(vec![
            Span::raw("↑↓: Navigate | "),
            Span::raw("Type: Edit | "),
            Span::raw("Ctrl+S: Save | "),
            Span::raw("Esc: Cancel"),
        ])];

        if let Some(err) = &self.error {
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
    }

    pub fn validate_and_save(&mut self) -> Option<NewConnectionAction> {
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

        if self.host == "127.0.0.1" {
            self.host = "localhost".to_string();
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

        if let Some(index) = self.modifying_index {
            Some(NewConnectionAction::Update(index, conn))
        } else {
            Some(NewConnectionAction::Save(conn))
        }
    }

    pub fn load_connection(&mut self, connection: &Connection) {
        self.name = connection.name.clone();
        self.db_type = connection.db_type.clone();
        self.host = connection.host.clone();
        self.port = connection.port.to_string();
        self.database = connection.database.clone();
        self.username = connection.username.clone();
        self.password = connection.password.clone();
        self.error = None;
        self.field_state.select(Some(0));
    }
}
