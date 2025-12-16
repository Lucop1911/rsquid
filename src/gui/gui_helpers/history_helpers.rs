use std::fs;

use crate::gui::HistoryPage;
use anyhow::Result;

impl HistoryPage {
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