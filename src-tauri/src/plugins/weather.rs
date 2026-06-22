use crate::plugins::Plugin;
use crate::models::{Entry, EntryKind};
use async_trait::async_trait;

pub struct WeatherPlugin;

#[async_trait]
impl Plugin for WeatherPlugin {
    fn id(&self) -> &str { "weather" }
    fn name(&self) -> &str { "Hyper-Local Weather" }
    fn description(&self) -> &str { "Precise weather forecasts and severe weather alerts." }
    fn trigger_hint(&self) -> &str { "weather" }
    fn is_pro(&self) -> bool { true }

    async fn search(&self, query: &str, _mode: &str) -> Vec<Entry> {
        let q = query.trim().to_lowercase();
        if q.starts_with("weather") || q.starts_with("temp") || q.starts_with("rain") || q.starts_with("wt") {
            // Mock data for demo
            return vec![
                Entry {
                    name: "22°C • Mostly Sunny".to_string(),
                    name_lower: "weather".to_string(),
                    path: "action:open_forecast".to_string(),
                    subtitle: "Weather • Feels like 24°C • Low chance of rain".to_string(),
                    kind: EntryKind::Plugin,
                    score: 100,
                    search_score: 1000,
                },
                Entry {
                    name: "View 7-Day Forecast".to_string(),
                    name_lower: "weather".to_string(),
                    path: "action:open_forecast".to_string(),
                    subtitle: "Weather • High-resolution predictions".to_string(),
                    kind: EntryKind::Plugin,
                    score: 80,
                    search_score: 900,
                }
            ];
        }
        vec![]
    }

    async fn execute(&self, _action_id: &str) -> Result<(), String> {
        // Open detailed forecast in browser using the standard open function
        opener::open("https://weather.com").ok();
        Ok(())
    }
}
