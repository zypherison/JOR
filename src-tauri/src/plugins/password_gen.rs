use crate::plugins::Plugin;
use crate::models::{Entry, EntryKind};
use async_trait::async_trait;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

pub struct PasswordGenPlugin;

#[async_trait]
impl Plugin for PasswordGenPlugin {
    fn id(&self) -> &str { "password_gen" }
    fn name(&self) -> &str { "Password Generator" }
    fn description(&self) -> &str { "Generate secure, random passwords instantly." }
    fn trigger_hint(&self) -> &str { "pass" }
    fn is_pro(&self) -> bool { false }

    async fn search(&self, query: &str, _mode: &str) -> Vec<Entry> {
        let q = query.trim().to_lowercase();
        if q.contains("pass") || q.contains("pw") || q.contains("gen") {
            let pass: String = thread_rng()
                .sample_iter(&Alphanumeric)
                .take(16)
                .map(char::from)
                .collect();

            return vec![
                Entry {
                    name: format!("Generate: {}", pass),
                    name_lower: "password".to_string(),
                    path: format!("password_gen:copy:{}", pass),
                    subtitle: "Password • 16 chars (Alphanumeric) • Click to copy".to_string(),
                    kind: EntryKind::Plugin,
                    score: 100,
                    search_score: 1000,
                }
            ];
        }
        vec![]
    }

    async fn execute(&self, action_id: &str) -> Result<(), String> {
        if let Some(pass) = action_id.strip_prefix("copy:") {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                clipboard.set_text(pass.to_string()).map_err(|e| e.to_string())?;
                return Ok(());
            }
        }
        Ok(())
    }
}
