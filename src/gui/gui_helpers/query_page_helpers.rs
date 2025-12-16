use ratatui::widgets::TableState;

use crate::{gui::{Focus, QueryPage, TableInfo}, utils::{connection::Connection, query_executor::QueryExecutor}};
use anyhow::Result;

impl QueryPage {
    pub async fn connect(&mut self, connection: Connection) -> Result<()> {
        let executor = QueryExecutor::new(&connection).await?;
        self.connection = Some(connection.clone());
        self.executor = Some(executor);
        self.query.clear();
        self.cursor_position = 0;
        self.results.clear();
        self.headers.clear();
        self.error = None;
        self.focus = Focus::Query;
        self.table_state = TableState::default();
        self.horizontal_scroll = 0;
        
        // Load tables
        self.load_tables().await?;
        
        Ok(())
    }

    pub async fn disconnect(&mut self) {
        if let Some(executor) = self.executor.take() {
            let _ = executor.close().await;
        }
        self.connection = None;
        self.tables.clear();
    }

    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.cursor_position = self.query.chars().count();
        self.focus = Focus::Query;
    }

    async fn load_tables(&mut self) -> Result<()> {
        if let Some(executor) = &self.executor {
            if let Some(conn) = &self.connection {
                let query = match conn.db_type.as_str() {
                    "postgres" => "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'",
                    "mysql" | "mariadb" => "SHOW TABLES",
                    "sqlite" => "SELECT name FROM sqlite_master WHERE type='table'",
                    _ => return Ok(()),
                };
                
                match executor.execute(query).await {
                    Ok((_, rows)) => {
                        self.tables = rows.iter()
                            .map(|row| TableInfo {
                                name: row[0].clone(),
                                fields: None,
                                expanded: false,
                            })
                            .collect();
                    }
                    Err(_) => {
                        self.tables.clear();
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn toggle_table_expansion(&mut self) -> Result<()> {
        if let Some(selected) = self.explorer_state.selected() {
            let mut actual_index = 0;
            let mut found_index = None;
            
            for (i, table) in self.tables.iter().enumerate() {
                if actual_index == selected {
                    found_index = Some(i);
                    break;
                }
                actual_index += 1;
                if table.expanded {
                    actual_index += table.fields.as_ref().map(|f| f.len()).unwrap_or(0);
                }
            }
            
            if let Some(idx) = found_index {
                if self.tables[idx].expanded {
                    self.tables[idx].expanded = false;
                } else {
                    if self.tables[idx].fields.is_none() {
                        if let Some(executor) = &self.executor {
                            if let Some(conn) = &self.connection {
                                let table_name = &self.tables[idx].name;
                                let query = match conn.db_type.as_str() {
                                    "postgres" => format!("SELECT column_name FROM information_schema.columns WHERE table_name = '{}'", table_name),
                                    "mysql" | "mariadb" => format!("DESCRIBE {}", table_name),
                                    "sqlite" => format!("PRAGMA table_info({})", table_name),
                                    _ => String::new(),
                                };
                                
                                match executor.execute(&query).await {
                                    Ok((_, rows)) => {
                                        let field_index = match conn.db_type.as_str() {
                                            "postgres" => 0,
                                            "mysql" | "mariadb" => 0,
                                            "sqlite" => 1,
                                            _ => 0,
                                        };
                                        
                                        self.tables[idx].fields = Some(
                                            rows.iter()
                                                .map(|row| row.get(field_index).cloned().unwrap_or_default())
                                                .collect()
                                        );
                                    }
                                    Err(_) => {}
                                }
                            }
                        }
                    }
                    self.tables[idx].expanded = true;
                }
            }
        }
        Ok(())
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
        let max_len = if self.max_results > 0 {
            self.max_results.min(self.results.len() as u32) as usize
        } else {
            self.results.len()
        };

        let i = match self.table_state.selected() {
            Some(i) => {
                if i < max_len.saturating_sub(1) {
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
        let max_len = if self.max_results > 0 {
            self.max_results.min(self.results.len() as u32) as usize
        } else {
            self.results.len()
        };

        let i = match self.table_state.selected() {
            Some(i) => (i + 10).min(max_len.saturating_sub(1)),
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn explorer_scroll_up(&mut self) {
        if let Some(selected) = self.explorer_state.selected() {
            if selected > 0 {
                self.explorer_state.select(Some(selected - 1));
            }
        }
    }

    pub fn explorer_scroll_down(&mut self) {
        let mut total_items = self.tables.len();
        for table in &self.tables {
            if table.expanded {
                total_items += table.fields.as_ref().map(|f| f.len()).unwrap_or(0);
            }
        }

        if let Some(selected) = self.explorer_state.selected() {
            if selected < total_items.saturating_sub(1) {
                self.explorer_state.select(Some(selected + 1));
            }
        }
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
                    
                    if let Ok(history_manager) = crate::gui::history::HistoryManager::new() {
                        let _ = history_manager.save_query(self.query.clone());
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