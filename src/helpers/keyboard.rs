use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, KeyEventKind};
use anyhow::Result;
use crate::{gui::{ConnectionListAction, ConnectionListPage, Field, Focus, NewConnectionAction, NewConnectionPage, QueryPage, QueryPageAction}, helpers::connection::ConnectionManager};

impl QueryPage {
    pub async fn handle_input(&mut self, key: KeyEvent, kind: KeyEventKind) -> Result<Option<QueryPageAction>> {
        if kind != KeyEventKind::Press {
            return Ok(None);
        }
        
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
                    self.cursor_position = 0;
                    self.query_scroll = 0;
                }
                Ok(None)
            }
            KeyCode::Up if matches!(self.focus, Focus::Results) => {
                self.scroll_up();
                Ok(None)
            }
            KeyCode::Down if matches!(self.focus, Focus::Results) => {
                self.scroll_down();
                Ok(None)
            }
            KeyCode::Left if matches!(self.focus, Focus::Results) => {
                if self.horizontal_scroll > 0 {
                    self.horizontal_scroll -= 1;
                }
                Ok(None)
            }
            KeyCode::Right if matches!(self.focus, Focus::Results) => {
                if self.horizontal_scroll + 1 < self.headers.len() {
                    self.horizontal_scroll += 1;
                }
                Ok(None)
            }
            KeyCode::PageUp if matches!(self.focus, Focus::Results) => {
                self.scroll_page_up();
                Ok(None)
            }
            KeyCode::PageDown if matches!(self.focus, Focus::Results) => {
                self.scroll_page_down();
                Ok(None)
            }
            KeyCode::Char('t') | KeyCode::Char('T') if matches!(self.focus, Focus::Results) => {
                self.table_state.select(Some(0));
                Ok(None)
            }
            KeyCode::Char('b') | KeyCode::Char('B') if matches!(self.focus, Focus::Results) => {
                if !self.results.is_empty() {
                    self.table_state.select(Some(self.results.len() - 1));
                }
                Ok(None)
            }
            KeyCode::Char(c) if matches!(self.focus, Focus::Query) && !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let mut chars: Vec<char> = self.query.chars().collect();
                let cursor_pos = self.cursor_position.min(chars.len());
                chars.insert(cursor_pos, c);
                self.query = chars.into_iter().collect();
                self.cursor_position += 1;
                Ok(None)
            }
            KeyCode::Backspace if matches!(self.focus, Focus::Query) => {
                if self.cursor_position > 0 {
                    let mut chars: Vec<char> = self.query.chars().collect();
                    let cursor_pos = self.cursor_position.min(chars.len());
                    if cursor_pos > 0 {
                        chars.remove(cursor_pos - 1);
                        self.query = chars.into_iter().collect();
                        self.cursor_position -= 1;
                    }
                }
                Ok(None)
            }
            KeyCode::Delete if matches!(self.focus, Focus::Query) => {
                let mut chars: Vec<char> = self.query.chars().collect();
                let cursor_pos = self.cursor_position.min(chars.len());
                if cursor_pos < chars.len() {
                    chars.remove(cursor_pos);
                    self.query = chars.into_iter().collect();
                }
                Ok(None)
            }
            KeyCode::Enter if matches!(self.focus, Focus::Query) => {
                let mut chars: Vec<char> = self.query.chars().collect();
                let cursor_pos = self.cursor_position.min(chars.len());
                chars.insert(cursor_pos, '\n');
                self.query = chars.into_iter().collect();
                self.cursor_position += 1;
                Ok(None)
            }
            KeyCode::Left if matches!(self.focus, Focus::Query) => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
                Ok(None)
            }
            KeyCode::Right if matches!(self.focus, Focus::Query) => {
                if self.cursor_position < self.query.chars().count() {
                    self.cursor_position += 1;
                }
                Ok(None)
            }
            KeyCode::PageUp if matches!(self.focus, Focus::Query) => {
                self.cursor_position = 0;
                Ok(None)
            }
            KeyCode::PageDown if matches!(self.focus, Focus::Query) => {
                self.cursor_position = self.query.chars().count();
                Ok(None)
            }
            _ => Ok(None),
        }
    }
}

impl ConnectionListPage {
    pub fn handle_input(&mut self, key: KeyEvent, kind: KeyEventKind) -> Option<ConnectionListAction> {
        if kind != KeyEventKind::Press {
            return None;
        }

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

impl NewConnectionPage {
    pub fn handle_input(&mut self, key: KeyEvent, kind: KeyEventKind) -> Option<NewConnectionAction> {

        if kind != KeyEventKind::Press {
            return None;
        }

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
}