use crate::plugins::Plugin;
use crate::models::{Entry, EntryKind};
use async_trait::async_trait;

pub struct IpPlugin;

#[async_trait]
impl Plugin for IpPlugin {
    fn id(&self) -> &str { "ip_checker" }
    fn name(&self) -> &str { "Network IP Checker" }
    fn description(&self) -> &str { "Quickly view your local and public IP addresses." }
    fn trigger_hint(&self) -> &str { "ip" }
    fn is_pro(&self) -> bool { false }

    async fn search(&self, query: &str, _mode: &str) -> Vec<Entry> {
        let q = query.trim().to_lowercase();
        if q == "ip" || q == "net" || q == "network" || q.starts_with("ip") {
            // Mock local IP for now
            let local_ip = "192.168.1.42";
            return vec![
                Entry {
                    name: format!("Local IP: {}", local_ip),
                    name_lower: "ip".to_string(),
                    path: format!("copy:{}", local_ip),
                    subtitle: "Network • Click to copy local address".to_string(),
                    kind: EntryKind::Plugin,
                    score: 100,
                    search_score: 1000,
                },
                Entry {
                    name: "Check Public IP...".to_string(),
                    name_lower: "ip".to_string(),
                    path: "action:check_public".to_string(),
                    subtitle: "Network • Fetches your external address".to_string(),
                    kind: EntryKind::Plugin,
                    score: 90,
                    search_score: 950,
                }
            ];
        }
        vec![]
    }

    async fn execute(&self, action_id: &str) -> Result<(), String> {
        if let Some(val) = action_id.strip_prefix("copy:") {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                clipboard.set_text(val.to_string()).ok();
            }
        } else if action_id == "action:check_public" {
            // Actively fetch public IP for a real functionality
            if let Ok(resp) = reqwest::blocking::get("https://api.ipify.org") {
                if let Ok(ip) = resp.text() {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        clipboard.set_text(ip).ok();
                    }
                }
            }
        }
        Ok(())
    }
}
