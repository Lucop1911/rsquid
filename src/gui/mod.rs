mod connection_list;
mod new_connection;
mod query_page;
mod components;

pub use connection_list::*;
pub use new_connection::*;
pub use query_page::*;

use crate::helpers::connection::ConnectionManager;
use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::Frame;

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    ConnectionList,
    NewConnection,
    QueryPage,
}

pub struct App {
    pub state: AppState,
    pub connection_list: ConnectionListPage,
    pub new_connection: NewConnectionPage,
    pub query_page: QueryPage,
    pub connection_manager: ConnectionManager,
}

impl App {
    pub fn new() -> Result<Self> {
        let connection_manager = ConnectionManager::new()?;
        Ok(Self {
            state: AppState::ConnectionList,
            connection_list: ConnectionListPage::new(),
            new_connection: NewConnectionPage::new(),
            query_page: QueryPage::new(),
            connection_manager,
        })
    }

    pub fn render(&mut self, f: &mut Frame) {
        let area = f.area();
        match self.state {
            AppState::ConnectionList => {
                self.connection_list.render(f, area, &self.connection_manager);
            }
            AppState::NewConnection => {
                self.new_connection.render(f, area);
            }
            AppState::QueryPage => {
                self.query_page.render(f, area);
            }
        }
    }

    pub async fn handle_input(&mut self, key: KeyEvent) -> Result<()> {
        match self.state {
            AppState::ConnectionList => {
                if let Some(action) = self.connection_list.handle_input(key) {
                    match action {
                        ConnectionListAction::NewConnection => {
                            self.state = AppState::NewConnection;
                            self.new_connection.reset();
                        }
                        ConnectionListAction::SelectConnection(idx) => {
                            let connections = self.connection_manager.load_connections()?;
                            if idx < connections.len() {
                                let conn = connections[idx].clone();
                                self.query_page.connect(conn).await?;
                                self.state = AppState::QueryPage;
                            }
                        }
                        ConnectionListAction::DeleteConnection(idx) => {
                            self.connection_manager.delete_connection(idx)?;
                        }
                    }
                }
            }
            AppState::NewConnection => {
                if let Some(action) = self.new_connection.handle_input(key) {
                    match action {
                        NewConnectionAction::Cancel => {
                            self.state = AppState::ConnectionList;
                        }
                        NewConnectionAction::Save(conn) => {
                            self.connection_manager.save_connection(conn.clone())?;
                            self.query_page.connect(conn).await?;
                            self.state = AppState::QueryPage;
                        }
                    }
                }
            }
            AppState::QueryPage => {
                if let Some(action) = self.query_page.handle_input(key).await? {
                    match action {
                        QueryPageAction::Back => {
                            self.query_page.disconnect().await;
                            self.state = AppState::ConnectionList;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}