use crate::plugins::Plugin;
use crate::models::{Entry, EntryKind};
use async_trait::async_trait;
use std::collections::HashMap;

pub struct SnippetsPlugin {
    snippets: HashMap<String, String>,
}

impl SnippetsPlugin {
    pub fn new() -> Self {
        let mut snippets = HashMap::new();
        snippets.insert("!sig".to_string(), "Best regards,\nJOR Team".to_string());
        snippets.insert("!sh".to_string(), "I'm working on it right now!".to_string());
        snippets.insert("!meet".to_string(), "Let's hop on a quick call to discuss this.".to_string());
        
        Self { snippets }
    }
}

#[async_trait]
impl Plugin for SnippetsPlugin {
    fn id(&self) -> &str { "snippets" }
    fn name(&self) -> &str { "Text Snippets" }
    fn description(&self) -> &str { "Create and expand text shortcuts into full sentences instantly." }
    fn trigger_hint(&self) -> &str { "!keyword" }
    fn is_pro(&self) -> bool { true }

    async fn search(&self, query: &str, _mode: &str) -> Vec<Entry> {
        let mut results = Vec::new();
        if query.starts_with('!') {
            for (keyword, content) in &self.snippets {
                if keyword.starts_with(query) {
                    results.push(Entry {
                        name: format!("Expand: {}", keyword),
                        name_lower: keyword.to_lowercase(),
                        path: format!("snippets:{}", keyword),
                        subtitle: format!("Snippet • {}", content.chars().take(40).collect::<String>()),
                        kind: EntryKind::Plugin,
                        score: 100,
                        search_score: 1000,
                    });
                }
            }
        }
        results
    }

    async fn execute(&self, action_id: &str) -> Result<(), String> {
        if let Some(content) = self.snippets.get(action_id) {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                clipboard.set_text(content.to_string()).map_err(|e| e.to_string())?;
                // The main.rs loop will handle the auto-paste if it's a plugin action
                return Ok(());
            }
        }
        Err("Snippet not found".into())
    }
}
