use crate::models::Entry;
use async_trait::async_trait;

#[async_trait]
pub trait Plugin: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn init(&self, _app: &tauri::AppHandle) {}
    async fn search(&self, query: &str, mode: &str) -> Vec<Entry>;
    async fn execute(&self, action_id: &str) -> Result<(), String>;
}

pub mod clipboard;
mod clipboard_db;
pub mod converter;
pub mod window_manager;
