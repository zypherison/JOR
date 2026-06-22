use crate::models::Entry;
use async_trait::async_trait;

#[async_trait]
pub trait Plugin: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn trigger_hint(&self) -> &str;
    fn is_pro(&self) -> bool { false }
    fn init(&self, _app: &tauri::AppHandle) {}
    async fn search(&self, query: &str, mode: &str) -> Vec<Entry>;
    async fn execute(&self, action_id: &str) -> Result<(), String>;
}

pub mod clipboard;
mod clipboard_db;
pub mod converter;
pub mod window_manager;
pub mod snippets;
pub mod sys_info;
pub mod password_gen;
pub mod timer;
pub mod color_picker;
pub mod ip_info;
pub mod weather;
