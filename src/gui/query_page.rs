use crate::utils::{connection::Connection, query_executor::QueryExecutor};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Row, Table, TableState, Wrap},
};

pub enum QueryPageAction {
    Back,
    OpenHistory,
}

#[derive(PartialEq)]
pub enum Focus {
    Query,
    Results,
    Explorer,
}

#[derive(Clone)]
pub struct TableInfo {
    pub name: String,
    pub fields: Option<Vec<String>>,
    pub expanded: bool,
}

pub struct QueryPage {
    pub query: String,
    pub cursor_position: usize,
    pub results: Vec<Vec<String>>,
    pub headers: Vec<String>,
    pub error: Option<String>,
    pub connection: Option<Connection>,
    pub executor: Option<QueryExecutor>,
    pub focus: Focus,
    pub query_scroll: u16,
    pub table_state: TableState,
    pub horizontal_scroll: usize,
    pub max_results: u32,
    pub input_buffer: String,
    pub show_input_overlay: bool,
    pub tables: Vec<TableInfo>,
    pub explorer_state: ListState,
}

impl QueryPage {
    pub fn new() -> Self {
        let mut explorer_state = ListState::default();
        explorer_state.select(Some(0));
        
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
            max_results: 0,
            input_buffer: String::new(),
            show_input_overlay: false,
            tables: Vec::new(),
            explorer_state,
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let use_explorer = self.focus == Focus::Explorer || !self.tables.is_empty();
        
        let main_area = if use_explorer {
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(30),
                    Constraint::Min(0),
                ])
                .split(area);
            
            self.render_explorer(f, main_chunks[0]);
            
            // Return the right panel for main content
            main_chunks[1]
        } else {
            area
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(10),
                Constraint::Min(0),
                Constraint::Length(4),
            ])
            .split(main_area);

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

        let help_text = if matches!(self.focus, Focus::Results) && !self.results.is_empty() {
            "Up/Down: Scroll | Left/Right: Columns | PgUp/PgDn: Page | T/B: Top/Bottom | Ctrl+L: Limit | Tab: Focus | 1: Query | 2: Explorer | Esc: Back"
        } else if matches!(self.focus, Focus::Explorer) {
            "Up/Down: Navigate | Enter: Expand/Collapse | 1: Query Focus | 2: Explorer | Esc: Back"
        } else {
            "Ctrl+E: Execute | Ctrl+C: Clear | Ctrl+R: History | Ctrl+L: Set Limit | Tab: Focus | 1: Query | 2: Explorer | Esc: Back"
        };

        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: false });
        f.render_widget(help, chunks[3]);

        // Render input overlay if active
        if self.show_input_overlay {
            crate::gui::input_overlay::draw_input_overlay(f, self);
        }
    }

    fn render_explorer(&mut self, f: &mut Frame, area: Rect) {
        let mut items = Vec::new();
        
        for table in &self.tables {
            items.push(ListItem::new(format!("📁 {}", table.name)));
            
            if table.expanded {
                if let Some(fields) = &table.fields {
                    for field in fields {
                        items.push(ListItem::new(format!("  └─ {}", field))
                            .style(Style::default().fg(Color::Gray)));
                    }
                }
            }
        }

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
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Tables")
                .border_style(if self.focus == Focus::Explorer {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                }))
            .highlight_style(highlight)
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, area, &mut self.explorer_state);
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

        let display_text = if is_focused {
            let mut chars: Vec<char> = self.query.chars().collect();
            let cursor_pos = self.cursor_position.min(chars.len());
            chars.insert(cursor_pos, '|');
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

        let visible_headers: Vec<&String> =
            self.headers.iter().skip(self.horizontal_scroll).collect();
        let num_visible = visible_headers.len().min(10);
        let visible_headers: Vec<&String> =
            visible_headers.iter().take(num_visible).copied().collect();

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

        let display_results: Vec<&Vec<String>> = if self.max_results > 0 {
            self.results.iter().take(self.max_results as usize).collect()
        } else {
            self.results.iter().collect()
        };

        let rows = display_results.iter().enumerate().map(|(row_idx, row)| {
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

        let widths = if num_visible > 0 {
            vec![Constraint::Percentage(100 / num_visible as u16); num_visible]
        } else {
            vec![Constraint::Percentage(100)]
        };

        let total_rows = if self.max_results > 0 {
            self.max_results.min(self.results.len() as u32)
        } else {
            self.results.len() as u32
        };

        let scroll_info = if self.headers.len() > num_visible {
            format!(
                " [Row {}/{}, Col {}/{}] ",
                selected_row + 1,
                total_rows,
                self.horizontal_scroll + 1,
                self.headers.len()
            )
        } else {
            format!(" [Row {}/{}] ", selected_row + 1, total_rows)
        };

        let row_highlight = {
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

        let title = if self.max_results > 0 {
            format!(
                "Results ({} of {} rows, limit: {}){}",
                total_rows,
                self.results.len(),
                self.max_results,
                scroll_info
            )
        } else {
            format!("Results ({} rows){}", self.results.len(), scroll_info)
        };

        let table = Table::new(rows, widths)
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(match self.focus {
                        Focus::Results => Style::default().fg(Color::Yellow),
                        _ => Style::default(),
                    }),
            )
            .row_highlight_style(row_highlight)
            .highlight_symbol(">> ");

        f.render_stateful_widget(table, area, &mut self.table_state);
    }
}