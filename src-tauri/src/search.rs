// ─────────────────────────────────────────────────────────────
// JOR — Smart Search Engine
// Fuzzy matching with intelligent ranking:
//   - Exact substring matches are heavily boosted
//   - Path matching provides secondary discovery
//   - Usage frequency tracking for smart ranking
// ─────────────────────────────────────────────────────────────

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use crate::models::Entry;
use std::collections::HashMap;
use std::sync::Mutex;

pub struct SearchEngine {
    matcher: SkimMatcherV2,
    /// Tracks how many times each entry path has been launched.
    /// Used to boost frequently-used items to the top.
    pub usage_counts: Mutex<HashMap<String, u32>>,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default().smart_case(),
            usage_counts: Mutex::new(HashMap::new()),
        }
    }

    /// Record a launch event for smart ranking.
    pub fn record_usage(&self, path: &str) {
        if let Ok(mut counts) = self.usage_counts.lock() {
            *counts.entry(path.to_string()).or_insert(0) += 1;
        }
    }

    pub fn save_usage(&self, path: &std::path::Path) -> std::io::Result<()> {
        let counts = self.usage_counts.lock().unwrap();
        let encoded = serde_json::to_string(&*counts).unwrap();
        std::fs::write(path, encoded)
    }

    pub fn load_usage(&self, path: &std::path::Path) {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(map) = serde_json::from_str::<HashMap<String, u32>>(&content) {
                let mut counts = self.usage_counts.lock().unwrap();
                *counts = map;
            }
        }
    }

    /// Perform a fuzzy search with smart ranking.
    /// Returns up to 20 results, ordered by composite score:
    ///   fuzzy_score + exact_match_bonus + usage_bonus + static_score
    pub fn search(&self, query: &str, entries: &[Entry]) -> Vec<Entry> {
        if query.is_empty() {
            // When empty, show most-used items first, then recent
            let counts = self.usage_counts.lock().unwrap();
            let mut scored: Vec<(u32, &Entry)> = entries.iter()
                .map(|e| {
                    let usage = counts.get(&e.path).copied().unwrap_or(0);
                    (usage + e.score, e)
                })
                .collect();
            scored.sort_by(|a, b| b.0.cmp(&a.0));
            return scored.into_iter().take(20).map(|(_, e)| e.clone()).collect();
        }

        let query_lower = query.to_lowercase();
        let counts = self.usage_counts.lock().unwrap();

        let mut results: Vec<(i64, Entry)> = entries
            .iter()
            .filter_map(|entry| {
                // Match against name
                let name_score = self.matcher.fuzzy_match(&entry.name_lower, &query_lower);

                // Match against keywords (if any)
                let mut kw_score = None;
                if let Some(kws) = &entry.keywords {
                    for kw in kws {
                        if let Some(s) = self.matcher.fuzzy_match(&kw.to_lowercase(), &query_lower) {
                            kw_score = Some(kw_score.unwrap_or(0).max(s));
                        }
                    }
                }

                // Match against full path (half weight)
                let path_lower = entry.path.to_lowercase();
                let path_score = self.matcher.fuzzy_match(&path_lower, &query_lower)
                    .map(|s| s / 2);

                let best = match (name_score, kw_score, path_score) {
                    (Some(a), Some(b), Some(c)) => Some(a.max(b).max(c)),
                    (Some(a), Some(b), None) => Some(a.max(b)),
                    (Some(a), None, Some(c)) => Some(a.max(c)),
                    (None, Some(b), Some(c)) => Some(b.max(c)),
                    (Some(a), None, None) => Some(a),
                    (None, Some(b), None) => Some(b),
                    (None, None, Some(c)) => Some(c),
                    (None, None, None) => None,
                };

                best.map(|score| {
                    // Exact substring bonus (big boost for direct matches)
                    let exact_bonus: i64 = if entry.name_lower.contains(&query_lower) { 80 } else { 0 };

                    // Starts-with bonus (even bigger for prefix matches)
                    let prefix_bonus: i64 = if entry.name_lower.starts_with(&query_lower) { 120 } else { 0 };

                    // Usage frequency bonus
                    let usage = counts.get(&entry.path).copied().unwrap_or(0) as i64;
                    let usage_bonus = usage * 10; // Each launch adds +10 to ranking

                    // Type priority bonus (Favor Apps > System > Workflows > Folders > Files)
                    let kind_bonus: i64 = match entry.kind {
                        crate::models::EntryKind::App => 200,
                        crate::models::EntryKind::System => 180,
                        crate::models::EntryKind::Workflow => 150,
                        crate::models::EntryKind::Folder => 50,
                        crate::models::EntryKind::File => 0,
                        _ => 0,
                    };

                    let total = score + exact_bonus + prefix_bonus + usage_bonus + kind_bonus + (entry.score as i64);
                    (total, entry.clone())
                })
            })
            .collect();

        results.sort_by(|a, b| b.0.cmp(&a.0));
        results.into_iter().take(20).map(|(_, e)| e).collect()
    }
}
