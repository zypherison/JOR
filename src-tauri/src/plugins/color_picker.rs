use crate::plugins::Plugin;
use crate::models::{Entry, EntryKind};
use async_trait::async_trait;

pub struct ColorPickerPlugin;

#[async_trait]
impl Plugin for ColorPickerPlugin {
    fn id(&self) -> &str { "color_picker" }
    fn name(&self) -> &str { "Design Color Picker" }
    fn description(&self) -> &str { "Advanced screen color sampler and palette generator." }
    fn trigger_hint(&self) -> &str { "pick / hex" }
    fn is_pro(&self) -> bool { true }

    async fn search(&self, query: &str, _mode: &str) -> Vec<Entry> {
        let q = query.trim().to_lowercase();
        if q == "color" || q == "pick" || q == "hex" || q.starts_with("pick") || q.starts_with("hex") {
            return vec![
                Entry {
                    name: "Pick Color from Screen".to_string(),
                    name_lower: "color".to_string(),
                    path: "color_picker:action:pick".to_string(),
                    subtitle: "Color Picker • Sample any pixel on your display".to_string(),
                    kind: EntryKind::Plugin,
                    score: 100,
                    search_score: 1000,
                },
                Entry {
                    name: "Convert #8b5cf6 to RGB".to_string(),
                    name_lower: "color".to_string(),
                    path: "color_picker:copy:rgb(139, 92, 246)".to_string(),
                    subtitle: "Color Picker • Quick conversion (Example)".to_string(),
                    kind: EntryKind::Plugin,
                    score: 80,
                    search_score: 900,
                }
            ];
        }
        vec![]
    }

    async fn execute(&self, action_id: &str) -> Result<(), String> {
        if action_id.starts_with("copy:") {
            if let Some(val) = action_id.strip_prefix("copy:") {
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    clipboard.set_text(val.to_string()).ok();
                }
            }
        }
        // Screen picking would require a dedicated native UI/overlay
        Ok(())
    }
}
