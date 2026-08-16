use crate::plugins::Plugin;
use crate::models::{Entry, EntryKind};
use async_trait::async_trait;

pub struct TimerPlugin;

#[async_trait]
impl Plugin for TimerPlugin {
    fn id(&self) -> &str { "timer" }
    fn name(&self) -> &str { "Focus Timer" }
    fn description(&self) -> &str { "Set quick countdown timers for focus and productivity." }
    fn trigger_hint(&self) -> &str { "timer 5" }
    fn is_pro(&self) -> bool { false }

    async fn search(&self, query: &str, _mode: &str) -> Vec<Entry> {
        let q = query.trim().to_lowercase();
        if q.starts_with("timer") || q.starts_with("remind") || q.starts_with("tm") {
            let parts: Vec<&str> = q.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(mins) = parts[1].parse::<u32>() {
                    return vec![
                        Entry {
                            name: format!("Start {} minute timer", mins),
                            name_lower: "timer".to_string(),
                            path: format!("timer:start:{}", mins),
                            subtitle: "Timer • Will notify you when done".to_string(),
                            kind: EntryKind::Plugin,
                            score: 100,
                            search_score: 1000,
                        }
                    ];
                }
            }
            
            return vec![
                Entry {
                    name: "Start 25m Focus Timer (Pomodoro)".to_string(),
                    name_lower: "timer".to_string(),
                    path: "timer:start:25".to_string(),
                    subtitle: "Timer • Standard focus block".to_string(),
                    kind: EntryKind::Plugin,
                    score: 90,
                    search_score: 950,
                }
            ];
        }
        vec![]
    }

    async fn execute(&self, action_id: &str) -> Result<(), String> {
        if let Some(mins_str) = action_id.strip_prefix("start:") {
            if let Ok(mins) = mins_str.parse::<u64>() {
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(mins * 60));
                    // Alert the user via PowerShell msgbox for a real functionality
                    let msg = format!("Focus block of {} minutes is complete!", mins);
                    let _ = std::process::Command::new("powershell")
                        .args(["-Command", &format!("Add-Type -AssemblyName PresentationFramework; [System.Windows.MessageBox]::Show('{}', 'JOR Systems — Focus Timer')", msg)])
                        .spawn();
                });
            }
        }
        Ok(())
    }
}
