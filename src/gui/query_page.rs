use crate::helpers::{connection::Connection, query_executor::QueryExecutor};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, Wrap},
    Frame,
};

pub enum QueryPageAction {
    Back,
}

enum Focus {
    Query,
    Results,
}

pub struct QueryPage {
    query: String,
    results: Vec<Vec<String>>,
    headers: Vec<String>,
    error: Option<String>,
    connection: Option<Connection>,
    executor: Option<QueryExecutor>,
    focus: Focus,
    query_scroll: u16,
}

impl QueryPage {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            headers: Vec::new(),
            error: None,
            connection: None,
            executor: None,
            focus: Focus::Query,
            query_scroll: 0,
        }
    }

    pub async fn connect(&mut self, connection: Connection) -> Result<()> {
        let executor = QueryExecutor::new(&connection).await?;
        self.connection = Some(connection);
        self.executor = Some(executor);
        self.query.clear();
        self.results.clear();
        self.headers.clear();
        self.error = None;
        Ok(())
    }

    pub async fn disconnect(&mut self) {
        if let Some(executor) = self.executor.take() {
            let _ = executor.close().await;
        }
        self.connection = None;
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(10),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(area);

        // Title with connection info
        let conn_name = self
            .connection
            .as_ref()
            .map(|c| c.name.as_str())
            .unwrap_or("No Connection");
        let title = Paragraph::new(format!("Query Editor - {}", conn_name))
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Query input
        let query_block = Block::default()
            .borders(Borders::ALL)
            .title("SQL Query (Ctrl+E to Execute)")
            .border_style(match self.focus {
                Focus::Query => Style::default().fg(Color::Yellow),
                Focus::Results => Style::default(),
            });

        let query_text = Paragraph::new(self.query.as_str())
            .block(query_block)
            .wrap(Wrap { trim: false })
            .scroll((self.query_scroll, 0));
        f.render_widget(query_text, chunks[1]);

        // Results or error
        if let Some(err) = &self.error {
            let error_text = Paragraph::new(err.as_str())
                .style(Style::default().fg(Color::Red))
                .block(Block::default().borders(Borders::ALL).title("Error"))
                .wrap(Wrap { trim: false });
            f.render_widget(error_text, chunks[2]);
        } else if !self.results.is_empty() {
            let header_cells = self
                .headers
                .iter()
                .map(|h| ratatui::widgets::Cell::from(h.as_str()).style(Style::default().fg(Color::Yellow)));
            let header = Row::new(header_cells).height(1).bottom_margin(1);

            let rows = self.results.iter().map(|row| {
                let cells = row.iter().map(|c| ratatui::widgets::Cell::from(c.as_str()));
                Row::new(cells).height(1)
            });

            let widths = vec![Constraint::Percentage(100 / self.headers.len().max(1) as u16); self.headers.len()];

            let table = Table::new(rows, widths)
                .header(header)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!("Results ({} rows)", self.results.len()))
                        .border_style(match self.focus {
                            Focus::Results => Style::default().fg(Color::Yellow),
                            Focus::Query => Style::default(),
                        }),
                )
                .style(Style::default().fg(Color::White));

            f.render_widget(table, chunks[2]);
        } else {
            let placeholder = Paragraph::new("No results yet. Execute a query to see results here.")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL).title("Results"))
                .alignment(Alignment::Center);
            f.render_widget(placeholder, chunks[2]);
        }

        // Help
        let help = Paragraph::new(Line::from(vec![
            Span::raw("Ctrl+E: Execute | "),
            Span::raw("Ctrl+C: Clear | "),
            Span::raw("Tab: Switch Focus | "),
            Span::raw("Esc: Back"),
        ]))
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(help, chunks[3]);
    }

    pub async fn handle_input(&mut self, key: KeyEvent) -> Result<Option<QueryPageAction>> {
        match key.code {
            KeyCode::Esc => Ok(Some(QueryPageAction::Back)),
            KeyCode::Tab => {
                self.focus = match self.focus {
                    Focus::Query => Focus::Results,
                    Focus::Results => Focus::Query,
                };
                Ok(None)
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.execute_query().await?;
                Ok(None)
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if matches!(self.focus, Focus::Query) {
                    self.query.clear();
                    self.query_scroll = 0;
                }
                Ok(None)
            }
            KeyCode::Char(c) => {
                if matches!(self.focus, Focus::Query) {
                    self.query.push(c);
                }
                Ok(None)
            }
            KeyCode::Backspace => {
                if matches!(self.focus, Focus::Query) {
                    self.query.pop();
                }
                Ok(None)
            }
            KeyCode::Enter => {
                if matches!(self.focus, Focus::Query) {
                    self.query.push('\n');
                }
                Ok(None)
            }
            KeyCode::Up => {
                if matches!(self.focus, Focus::Query) && self.query_scroll > 0 {
                    self.query_scroll -= 1;
                }
                Ok(None)
            }
            KeyCode::Down => {
                if matches!(self.focus, Focus::Query) {
                    self.query_scroll += 1;
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    async fn execute_query(&mut self) -> Result<()> {
        self.error = None;
        self.results.clear();
        self.headers.clear();

        if self.query.trim().is_empty() {
            self.error = Some("Query is empty".to_string());
            return Ok(());
        }

        if let Some(executor) = &self.executor {
            match executor.execute(&self.query).await {
                Ok((headers, rows)) => {
                    self.headers = headers;
                    self.results = rows;
                }
                Err(e) => {
                    self.error = Some(format!("Query error: {}", e));
                }
            }
        } else {
            self.error = Some("Not connected to database".to_string());
        }

        Ok(())
    }
}