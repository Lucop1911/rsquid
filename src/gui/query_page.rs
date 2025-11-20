use crate::helpers::{connection::Connection, query_executor::QueryExecutor};
use anyhow::Result;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Row, Table, TableState, Wrap},
};

pub enum QueryPageAction {
    Back,
}

pub enum Focus {
    Query,
    Results,
}

pub struct QueryPage {
    pub(crate) query: String,
    pub(crate) cursor_position: usize,
    pub(crate) results: Vec<Vec<String>>,
    pub(crate) headers: Vec<String>,
    error: Option<String>,
    connection: Option<Connection>,
    executor: Option<QueryExecutor>,
    pub(crate) focus: Focus,
    pub(crate) query_scroll: u16,
    pub(crate) table_state: TableState,
    pub(crate) horizontal_scroll: usize,
}

impl QueryPage {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            cursor_position: 0,
            results: Vec::new(),
            headers: Vec::new(),
            error: None,
            connection: None,
            executor: None,
            focus: Focus::Query,
            query_scroll: 0,
            table_state: TableState::default(),
            horizontal_scroll: 0,
        }
    }

    pub async fn connect(&mut self, connection: Connection) -> Result<()> {
        let executor = QueryExecutor::new(&connection).await?;
        self.connection = Some(connection);
        self.executor = Some(executor);
        self.query.clear();
        self.cursor_position = 0;
        self.results.clear();
        self.headers.clear();
        self.error = None;
        self.focus = Focus::Query;
        self.table_state = TableState::default();
        self.horizontal_scroll = 0;
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
                Constraint::Length(4),
            ])
            .split(area);

        let conn_name = self
            .connection
            .as_ref()
            .map(|c| c.name.as_str())
            .unwrap_or("No Connection");
        let title = Paragraph::new(format!("Query Editor - {}", conn_name))
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        self.render_query_input(f, chunks[1]);

        // Error or results
        if let Some(err) = &self.error {
            let error_text = Paragraph::new(err.as_str())
                .style(Style::default().fg(Color::Red))
                .block(Block::default().borders(Borders::ALL).title("Error"))
                .wrap(Wrap { trim: false });
            f.render_widget(error_text, chunks[2]);
        } else if !self.results.is_empty() {
            self.render_table(f, chunks[2]);
        } else {
            let placeholder =
                Paragraph::new("No results yet. Execute a query to see results here.")
                    .style(Style::default().fg(Color::DarkGray))
                    .block(Block::default().borders(Borders::ALL).title("Results"))
                    .alignment(Alignment::Center);
            f.render_widget(placeholder, chunks[2]);
        }

        // Help footer
        let help_text = if matches!(self.focus, Focus::Results) && !self.results.is_empty() {
            "Up/Down: Scroll Rows | Left/Right: Scroll Columns | PgUp/PgDn: Page | T/B: Top/Bottom | Ctrl+E: Execute | Ctrl+C: Clear | Tab: Switch Focus | Esc: Back"
        } else {
            "Ctrl+E: Execute | Ctrl+C: Clear | Tab: Switch Focus | Esc: Back"
        };

        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: false });
        f.render_widget(help, chunks[3]);
    }

    fn render_query_input(&mut self, f: &mut Frame, area: Rect) {
        let is_focused = matches!(self.focus, Focus::Query);

        let query_block = Block::default()
            .borders(Borders::ALL)
            .title(if is_focused {
                "SQL Query (Ctrl+E to Execute) [EDITING]"
            } else {
                "SQL Query (Ctrl+E to Execute)"
            })
            .border_style(if is_focused {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            });

        // Show cursor only when focused
        let display_text = if is_focused {
            let mut chars: Vec<char> = self.query.chars().collect();
            let cursor_pos = self.cursor_position.min(chars.len());
            chars.insert(cursor_pos, '|'); // Use pipe as cursor
            chars.into_iter().collect()
        } else {
            self.query.clone()
        };

        let query_text = Paragraph::new(display_text)
            .block(query_block)
            .wrap(Wrap { trim: false })
            .scroll((self.query_scroll, 0));
        f.render_widget(query_text, area);
    }

    fn render_table(&mut self, f: &mut Frame, area: Rect) {
        let selected_row = self.table_state.selected().unwrap_or(0);

        // Visible columns based on horizontal scroll
        let visible_headers: Vec<&String> =
            self.headers.iter().skip(self.horizontal_scroll).collect();
        let num_visible = visible_headers.len().min(10); // Show max 10 columns at once
        let visible_headers: Vec<&String> =
            visible_headers.iter().take(num_visible).copied().collect();

        // Header
        let header_cells = visible_headers.iter().enumerate().map(|(idx, h)| {
            let actual_col_idx = idx + self.horizontal_scroll;
            let style = if actual_col_idx == self.horizontal_scroll {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                Style::default().fg(Color::Yellow)
            };
            ratatui::widgets::Cell::from(h.as_str()).style(style)
        });
        let header = Row::new(header_cells).height(1).bottom_margin(1);

        // Rows
        let rows = self.results.iter().enumerate().map(|(row_idx, row)| {
            let visible_cells: Vec<String> = row
                .iter()
                .skip(self.horizontal_scroll)
                .take(num_visible)
                .cloned()
                .collect();

            let cells = visible_cells.into_iter().enumerate().map(|(col_idx, c)| {
                let actual_col_idx = col_idx + self.horizontal_scroll;

                let style = if row_idx == selected_row && actual_col_idx == self.horizontal_scroll {
                    Style::default()
                        .fg(Color::Green)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else if row_idx == selected_row {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else if actual_col_idx == self.horizontal_scroll {
                    Style::default().fg(Color::LightBlue)
                } else {
                    Style::default()
                };

                ratatui::widgets::Cell::from(c).style(style)
            });

            Row::new(cells).height(1)
        });

        // Dynamic widths
        let widths = if num_visible > 0 {
            vec![Constraint::Percentage(100 / num_visible as u16); num_visible]
        } else {
            vec![Constraint::Percentage(100)]
        };

        let scroll_info = if self.headers.len() > num_visible {
            format!(
                " [Row {}/{}, Col {}/{}] ",
                selected_row + 1,
                self.results.len(),
                self.horizontal_scroll + 1,
                self.headers.len()
            )
        } else {
            format!(" [Row {}/{}] ", selected_row + 1, self.results.len())
        };

        let table = Table::new(rows, widths)
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(
                        "Results ({} rows){}",
                        self.results.len(),
                        scroll_info
                    ))
                    .border_style(match self.focus {
                        Focus::Results => Style::default().fg(Color::Yellow),
                        Focus::Query => Style::default(),
                    }),
            )
            .row_highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        f.render_stateful_widget(table, area, &mut self.table_state);
    }

    pub fn scroll_up(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i > 0 {
                    i - 1
                } else {
                    0
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn scroll_down(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i < self.results.len().saturating_sub(1) {
                    i + 1
                } else {
                    i
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn scroll_page_up(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => i.saturating_sub(10),
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn scroll_page_down(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => (i + 10).min(self.results.len().saturating_sub(1)),
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub async fn execute_query(&mut self) -> Result<()> {
        self.error = None;
        self.results.clear();
        self.headers.clear();
        self.table_state = TableState::default();
        self.horizontal_scroll = 0;

        if self.query.trim().is_empty() {
            self.error = Some("Query is empty".to_string());
            return Ok(());
        }

        if let Some(executor) = &self.executor {
            match executor.execute(&self.query).await {
                Ok((headers, rows)) => {
                    self.headers = headers;
                    self.results = rows;
                    if !self.results.is_empty() {
                        self.table_state.select(Some(0));
                    }
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
