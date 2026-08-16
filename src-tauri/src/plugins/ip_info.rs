use crate::plugins::Plugin;
use crate::models::{Entry, EntryKind};
use async_trait::async_trait;

pub struct IpPlugin;

/// Determine the primary local IPv4 by opening a UDP socket to a public
/// address (no packets are actually sent) and reading the bound address.
fn local_ip_address() -> Option<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    Some(socket.local_addr().ok()?.ip().to_string())
}

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
            // The primary local IP, discovered without any extra dependency.
            let local_ip = local_ip_address().unwrap_or_else(|| "Unavailable".to_string());
            return vec![
                Entry {
                    name: format!("Local IP: {}", local_ip),
                    name_lower: "ip".to_string(),
                    path: format!("ip_checker:copy:{}", local_ip),
                    subtitle: "Network • Click to copy local address".to_string(),
                    kind: EntryKind::Plugin,
                    score: 100,
                    search_score: 1000,
                },
                Entry {
                    name: "Check Public IP...".to_string(),
                    name_lower: "ip".to_string(),
                    path: "ip_checker:action:check_public".to_string(),
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
            match reqwest::get("https://api.ipify.org").await {
                Ok(resp) => {
                    if let Ok(ip) = resp.text().await {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            clipboard.set_text(ip).ok();
                        }
                    }
                }
                Err(e) => return Err(format!("Could not reach IP service: {}", e)),
            }
        }
        Ok(())
    }
}
