mod components;
mod connection_list;
mod new_connection;
mod query_page;

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
    pub error_message: Option<String>,
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
            error_message: None,
        })
    }

    pub fn render(&mut self, f: &mut Frame) {
        let area = f.area();
        match self.state {
            AppState::ConnectionList => {
                self.connection_list
                    .render(f, area, &self.connection_manager, &self.error_message);
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
        if self.state == AppState::ConnectionList && self.error_message.is_some() {
            self.error_message = None;
        }

        match self.state {
            AppState::ConnectionList => {
                if let Some(action) = self.connection_list.handle_input(key, key.kind) {
                    match action {
                        ConnectionListAction::NewConnection => {
                            self.state = AppState::NewConnection;
                            self.new_connection.reset();
                        }
                        ConnectionListAction::SelectConnection(idx) => {
                            let connections = self.connection_manager.load_connections()?;
                            if idx < connections.len() {
                                let conn = connections[idx].clone();
                                // Connection attempt
                                match self.query_page.connect(conn).await {
                                    Ok(_) => {
                                        self.state = AppState::QueryPage;
                                        self.error_message = None;
                                    }
                                    Err(e) => {
                                        self.error_message =
                                            Some(format!("Connection failed: {}", e));
                                    }
                                }
                            }
                        }
                        ConnectionListAction::DeleteConnection(idx) => {
                            self.connection_manager.delete_connection(idx)?;
                        }
                        ConnectionListAction::ModifyConnection(idx) => {
                            let connections = self.connection_manager.load_connections()?;
                            if idx < connections.len() {
                                self.new_connection.reset();
                                self.new_connection.load_connection(&connections[idx]);
                                self.new_connection.modifying_index = Some(idx);
                                self.state = AppState::NewConnection;
                            }
                        }
                    }
                }
            }
            AppState::NewConnection => {
                if let Some(action) = self.new_connection.handle_input(key, key.kind) {
                    match action {
                        NewConnectionAction::Cancel => {
                            self.state = AppState::ConnectionList;
                        }
                        NewConnectionAction::Save(conn) => {
                            self.connection_manager.save_connection(conn.clone())?;
                            // Connection attempt
                            match self.query_page.connect(conn).await {
                                Ok(_) => {
                                    self.state = AppState::QueryPage;
                                    self.error_message = None;
                                }
                                Err(e) => {
                                    self.state = AppState::ConnectionList;
                                    self.error_message = Some(format!("Connection failed: {}", e));
                                }
                            }
                        }
                        NewConnectionAction::Update(idx, conn) => {
                            self.connection_manager
                                .update_connection(idx, conn.clone())?;
                            self.state = AppState::ConnectionList;
                        }
                    }
                }
            }
            AppState::QueryPage => {
                if let Some(action) = self.query_page.handle_input(key, key.kind).await? {
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
