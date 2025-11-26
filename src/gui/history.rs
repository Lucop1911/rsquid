use anyhow::{Context, Ok, Result};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::fs;
use std::path::PathBuf;

pub enum HistoryPageAction {
    Back,
    SelectQuery(String),
    DeleteQuery(String),
}

pub struct HistoryManager {
    config_path: PathBuf,
}

impl HistoryManager {
    pub fn new() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .context("Could not find config directory")?
            .join("rsquid");
        
        fs::create_dir_all(&config_dir)?;
        
        let config_path = config_dir.join("history.json");
        
        Ok(Self { config_path })
    }

    pub fn load_history(&self) -> Result<Vec<String>> {
        if !self.config_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.config_path)?;
        let queries: Vec<String> = serde_json::from_str(&content)?;
        Ok(queries)
    }

    pub fn save_query(&self, query_string: String) -> Result<()> {
        let mut queries = self.load_history().unwrap_or_default();
        
        // Wont save consecutive identical queries
        if let Some(last) = queries.last() {
            if last == &query_string {
                return Ok(());
            }
        }
        
        queries.push(query_string);
        
        let content = serde_json::to_string_pretty(&queries)?;
        fs::write(&self.config_path, content)?;
        
        Ok(())
    }

    pub fn clear_history(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&Vec::<String>::new())?;
        fs::write(&self.config_path, content)?;
        Ok(())
    }
}

pub struct HistoryPage {
    pub(crate) list_state: ListState,
    history_manager: HistoryManager,
}

impl HistoryPage {
    pub fn new() -> Result<Self> {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        let history_manager = HistoryManager::new()?;
        
        Ok(Self {
            list_state,
            history_manager,
        })
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(area);

        let title = Paragraph::new("Query History")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        let history = self.history_manager.load_history().unwrap_or_default();

        let items: Vec<ListItem> = if history.is_empty() {
            vec![ListItem::new("No query history yet").style(
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )]
        } else {
            history
                .iter()
                .rev()
                .enumerate()
                .map(|(i, query)| {
                    // Truncate long queries for display
                    let display = if query.len() > 100 {
                        format!("{}. {}...", history.len() - i, &query[..97])
                    } else {
                        format!("{}. {}", history.len() - i, query.replace('\n', " "))
                    };
                    ListItem::new(display)
                })
                .collect()
        };

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
            .block(Block::default().borders(Borders::ALL).title("Queries"))
            .highlight_style(highlight)
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, chunks[1], &mut self.list_state);

        let help_text = if history.is_empty() {
            "Esc: Back"
        } else {
            "↑↓: Navigate | Enter: Use Query | d: Delete Selection | c: Clear History | Esc: Back"
        };

        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help, chunks[2]);

        let total_items = if history.is_empty() { 1 } else { history.len() };
        if let Some(selected) = self.list_state.selected() {
            if selected >= total_items {
                self.list_state.select(Some(total_items.saturating_sub(1)));
            }
        }
    }

    pub fn scroll_up(&mut self) {
        let i = self.list_state.selected().unwrap_or(0);
        if i > 0 {
            self.list_state.select(Some(i - 1));
        }
    }

    pub fn scroll_down(&mut self, max: usize) {
        let i = self.list_state.selected().unwrap_or(0);
        if i < max.saturating_sub(1) {
            self.list_state.select(Some(i + 1));
        }
    }

    pub fn get_selected_query(&self) -> Option<String> {
        let history = self.history_manager.load_history().ok()?;
        if history.is_empty() {
            return None;
        }
        
        let selected = self.list_state.selected()?;
        let actual_index = history.len().saturating_sub(1).saturating_sub(selected);
        history.get(actual_index).cloned()
    }

    pub fn clear_history(&mut self) -> Result<()> {
        self.history_manager.clear_history()?;
        self.list_state.select(Some(0));
        Ok(())
    }

    pub fn delete_query(&self, query_string: String) -> Result<()> {
        let mut history = self.history_manager.load_history().unwrap_or_default();

        if let Some(index) = history.iter().position(|s| s == &query_string) {
            history.remove(index);
        }

        let content = serde_json::to_string_pretty(&history)?;
        fs::write(&self.history_manager.config_path, content)?;

        Ok(())
    }
}