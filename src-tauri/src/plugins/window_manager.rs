use crate::plugins::Plugin;
use crate::models::Entry;
use async_trait::async_trait;

pub struct WindowManagerPlugin;

#[async_trait]
impl Plugin for WindowManagerPlugin {
    fn id(&self) -> &str { "window_manager" }
    fn name(&self) -> &str { "Window Manager" }
    async fn search(&self, _query: &str, _mode: &str) -> Vec<Entry> {
        vec![]
    }
    async fn execute(&self, _action_id: &str) -> Result<(), String> {
        Ok(())
    }
}
