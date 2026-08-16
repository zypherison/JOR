use crate::plugins::Plugin;
use crate::models::{Entry, EntryKind};
use async_trait::async_trait;
use sysinfo::System;

pub struct SystemInfoPlugin {
    sys: std::sync::Arc<std::sync::Mutex<System>>,
}

impl SystemInfoPlugin {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        Self { sys: std::sync::Arc::new(std::sync::Mutex::new(sys)) }
    }
}

#[async_trait]
impl Plugin for SystemInfoPlugin {
    fn id(&self) -> &str { "sys_info" }
    fn name(&self) -> &str { "System Monitor" }
    fn description(&self) -> &str { "Real-time view of your CPU, RAM, and system health." }
    fn trigger_hint(&self) -> &str { "cpu / ram" }
    fn is_pro(&self) -> bool { false }

    async fn search(&self, query: &str, _mode: &str) -> Vec<Entry> {
        let q = query.trim().to_lowercase();
        if q.contains("cpu") || q.contains("ram") || q.contains("sys") || q.contains("mem") {
            let mut sys = self.sys.lock().unwrap();
            sys.refresh_memory();
            sys.refresh_cpu_all();

            let mem_used = sys.used_memory() / 1024 / 1024;
            let mem_total = sys.total_memory() / 1024 / 1024;
            let cpu_usage = sys.global_cpu_usage();

            return vec![
                Entry {
                    name: format!("CPU Usage: {:.1}%", cpu_usage),
                    name_lower: "cpu".to_string(),
                    path: "sys:cpu".to_string(),
                    subtitle: format!("System • {} cores active", sys.cpus().len()),
                    kind: EntryKind::Plugin,
                    score: 100,
                    search_score: 1000,
                },
                Entry {
                    name: format!("RAM: {}MB / {}MB", mem_used, mem_total),
                    name_lower: "ram".to_string(),
                    path: "sys:ram".to_string(),
                    subtitle: format!("Memory • {:.1}% used", (mem_used as f32 / mem_total as f32) * 100.0),
                    kind: EntryKind::Plugin,
                    score: 90,
                    search_score: 950,
                }
            ];
        }
        vec![]
    }

    async fn execute(&self, _action_id: &str) -> Result<(), String> {
        // Open the platform's system monitor for a deep dive.
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("taskmgr")
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        #[cfg(target_os = "linux")]
        {
            // GNOME first, then KDE, then a plain xterm as last resort.
            for monitor in ["gnome-system-monitor", "ksysguard", "xterm"] {
                if std::process::Command::new(monitor)
                    .spawn()
                    .map(|_| ())
                    .is_ok()
                {
                    break;
                }
            }
        }
        // Other platforms: nothing to launch.
        Ok(())
    }
}
