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

    /// Perform a fuzzy search with smart ranking.
    /// Returns up to 20 results, ordered by composite score:
    ///   fuzzy_score + exact_match_bonus + usage_bonus + static_score
    pub fn search(&self, query: &str, entries: &[Entry]) -> Vec<Entry> {
        if query.is_empty() {
            // When empty, show most-used items first, then recent
            let counts = self.usage_counts.lock().unwrap();
            let mut scored: Vec<(u32, Entry)> = entries.iter()
                .map(|e| {
                    let usage = counts.get(&e.path).copied().unwrap_or(0);
                    let score = usage + e.score;
                    let mut entry = e.clone();
                    entry.search_score = score as i64;
                    (score, entry)
                })
                .collect();
            scored.sort_by(|a, b| b.0.cmp(&a.0));
            return scored.into_iter().take(20).map(|(_, e)| e).collect();
        }

        let query_lower = query.to_lowercase();
        let counts = self.usage_counts.lock().unwrap();

        let mut results: Vec<(i64, Entry)> = entries
            .iter()
            .filter_map(|entry| {
                // Primary: match against entry name
                let name_score = self.matcher.fuzzy_match(&entry.name_lower, &query_lower);

                // Secondary: match against full path (half weight)
                let path_lower = entry.path.to_lowercase();
                let path_score = self.matcher.fuzzy_match(&path_lower, &query_lower)
                    .map(|s| s / 2);

                let best = match (name_score, path_score) {
                    (Some(a), Some(b)) => Some(a.max(b)),
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                };

                best.map(|score| {
                    // Exact substring bonus (big boost for direct matches)
                    let exact_bonus: i64 = if entry.name_lower.contains(&query_lower) { 80 } else { 0 };

                    // Starts-with bonus (even bigger for prefix matches)
                    let prefix_bonus: i64 = if entry.name_lower.starts_with(&query_lower) { 120 } else { 0 };

                    // Usage frequency bonus
                    let usage = counts.get(&entry.path).copied().unwrap_or(0) as i64;
                    let usage_bonus = usage * 10; // Each launch adds +10 to ranking

                    let total = score + exact_bonus + prefix_bonus + usage_bonus + (entry.score as i64);
                    let mut e = entry.clone();
                    e.search_score = total;
                    (total, e)
                })
            })
            .collect();

        results.sort_by(|a, b| b.0.cmp(&a.0));
        results.into_iter().take(20).map(|(_, e)| e).collect()
    }
}
