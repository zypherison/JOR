use crate::plugins::Plugin;
use crate::models::{Entry, EntryKind};
use async_trait::async_trait;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// FX rates are re-fetched at most every 6 hours per currency pair.
const RATE_CACHE_TTL: Duration = Duration::from_secs(6 * 60 * 60);

/// Currencies covered by the Frankfurter API (ECB reference rates).
const CURRENCIES: &[&str] = &[
    "usd", "eur", "gbp", "jpy", "chf", "cad", "aud", "nzd", "sek", "nok", "dkk",
    "hkd", "sgd", "krw", "inr", "cny", "mxn", "zar", "brl", "pln", "try",
];

struct ConversionRule {
    factor: f64,
    category: &'static str,
}

lazy_static::lazy_static! {
    static ref RULES: HashMap<String, ConversionRule> = {
        let mut m = HashMap::new();
        // Length
        m.insert("m".to_string(), ConversionRule { factor: 1.0, category: "length" });
        m.insert("meter".to_string(), ConversionRule { factor: 1.0, category: "length" });
        m.insert("meters".to_string(), ConversionRule { factor: 1.0, category: "length" });
        m.insert("km".to_string(), ConversionRule { factor: 1000.0, category: "length" });
        m.insert("kilometer".to_string(), ConversionRule { factor: 1000.0, category: "length" });
        m.insert("kilometers".to_string(), ConversionRule { factor: 1000.0, category: "length" });
        m.insert("cm".to_string(), ConversionRule { factor: 0.01, category: "length" });
        m.insert("mm".to_string(), ConversionRule { factor: 0.001, category: "length" });
        m.insert("inch".to_string(), ConversionRule { factor: 0.0254, category: "length" });
        m.insert("in".to_string(), ConversionRule { factor: 0.0254, category: "length" });
        m.insert("ft".to_string(), ConversionRule { factor: 0.3048, category: "length" });
        m.insert("feet".to_string(), ConversionRule { factor: 0.3048, category: "length" });
        m.insert("mi".to_string(), ConversionRule { factor: 1609.34, category: "length" });
        m.insert("mile".to_string(), ConversionRule { factor: 1609.34, category: "length" });
        m.insert("miles".to_string(), ConversionRule { factor: 1609.34, category: "length" });

        // Weight
        m.insert("kg".to_string(), ConversionRule { factor: 1.0, category: "weight" });
        m.insert("kilogram".to_string(), ConversionRule { factor: 1.0, category: "weight" });
        m.insert("g".to_string(), ConversionRule { factor: 0.001, category: "weight" });
        m.insert("gram".to_string(), ConversionRule { factor: 0.001, category: "weight" });
        m.insert("lb".to_string(), ConversionRule { factor: 0.453592, category: "weight" });
        m.insert("lbs".to_string(), ConversionRule { factor: 0.453592, category: "weight" });
        m.insert("pound".to_string(), ConversionRule { factor: 0.453592, category: "weight" });
        m.insert("pounds".to_string(), ConversionRule { factor: 0.453592, category: "weight" });
        m.insert("oz".to_string(), ConversionRule { factor: 0.0283495, category: "weight" });
        m.insert("ounce".to_string(), ConversionRule { factor: 0.0283495, category: "weight" });

        m
    };

    // Very permissive regexes
    static ref RE_NATURAL: Regex = Regex::new(r"(?i)(?:convert\s+)?([\d\.]+)\s*([a-z]+)\s+(?:to|in|into|as)\s+([a-z]+)").unwrap();
    static ref RE_QUERY: Regex = Regex::new(r"(?i)(?:what\s+is\s+)?([\d\.]+)\s*([a-z]+)\s+(?:in|as)\s+([a-z]+)").unwrap();
    static ref RE_HOW_MANY: Regex = Regex::new(r"(?i)how\s+many\s+([a-z]+)\s+(?:are\s+in|in|is)\s+([\d\.]+)\s*([a-z]+)").unwrap();
}

pub struct ConverterPlugin {
    /// (from, to) -> (fetched_at, rate). Rates expire after RATE_CACHE_TTL.
    rate_cache: Mutex<HashMap<(String, String), (Instant, f64)>>,
}

impl ConverterPlugin {
    pub fn new() -> Self {
        Self {
            rate_cache: Mutex::new(HashMap::new()),
        }
    }

    /// Fetch a live FX rate from the Frankfurter API, caching it briefly.
    /// The API is case-insensitive but returns uppercase currency codes.
    async fn get_rate(&self, from: &str, to: &str) -> Option<f64> {
        if let Ok(cache) = self.rate_cache.lock() {
            if let Some((fetched_at, rate)) = cache.get(&(from.to_string(), to.to_string())) {
                if fetched_at.elapsed() < RATE_CACHE_TTL {
                    return Some(*rate);
                }
            }
        }

        let from_up = from.to_uppercase();
        let to_up = to.to_uppercase();
        let url = format!("https://api.frankfurter.app/latest?from={}&to={}", from_up, to_up);
        let resp = reqwest::get(&url).await.ok()?;
        let json: serde_json::Value = resp.json().await.ok()?;
        let rate = json["rates"][&to_up].as_f64()?;

        if let Ok(mut cache) = self.rate_cache.lock() {
            cache.insert((from.to_string(), to.to_string()), (Instant::now(), rate));
        }
        Some(rate)
    }
}

#[async_trait]
impl Plugin for ConverterPlugin {
    fn id(&self) -> &str { "converter" }
    fn name(&self) -> &str { "Natural Converter" }
    fn description(&self) -> &str { "Convert units, temperatures, and currencies naturally." }
    fn trigger_hint(&self) -> &str { "10kg to lbs · 10 usd to eur" }
    fn is_pro(&self) -> bool { false }

    async fn search(&self, query: &str, _mode: &str) -> Vec<Entry> {
        let q = query.trim();
        if q.is_empty() { return vec![]; }

        // Try different matchers
        let match_result = if let Some(caps) = RE_NATURAL.captures(q) {
            Some((caps[1].parse::<f64>().unwrap_or(0.0), caps[2].to_lowercase(), caps[3].to_lowercase()))
        } else if let Some(caps) = RE_QUERY.captures(q) {
            Some((caps[1].parse::<f64>().unwrap_or(0.0), caps[2].to_lowercase(), caps[3].to_lowercase()))
        } else if let Some(caps) = RE_HOW_MANY.captures(q) {
            Some((caps[2].parse::<f64>().unwrap_or(0.0), caps[3].to_lowercase(), caps[1].to_lowercase()))
        } else {
            None
        };

        if let Some((amount, from_unit, to_unit)) = match_result {
            if let Some(entry) = self.try_convert(amount, &from_unit, &to_unit).await {
                return vec![entry];
            }
        }

        vec![]
    }

    async fn execute(&self, action_id: &str) -> Result<(), String> {
        // action_id carries the computed result value; copy it to the clipboard.
        if action_id.is_empty() {
            return Ok(());
        }
        let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        clipboard.set_text(action_id.to_string()).map_err(|e| e.to_string())
    }
}

impl ConverterPlugin {
    async fn try_convert(&self, amount: f64, from_unit: &str, to_unit: &str) -> Option<Entry> {
        // Temperature
        let f_unit = from_unit.trim_end_matches('s'); // rudimentary plural handling
        let t_unit = to_unit.trim_end_matches('s');

        if (f_unit == "c" || f_unit == "celcius") && (t_unit == "f" || t_unit == "fahrenheit") {
            let result = (amount * 9.0 / 5.0) + 32.0;
            return Some(self.create_entry(result, to_unit, amount, from_unit));
        }
        if (f_unit == "f" || f_unit == "fahrenheit") && (t_unit == "c" || t_unit == "celcius") {
            let result = (amount - 32.0) * 5.0 / 9.0;
            return Some(self.create_entry(result, to_unit, amount, from_unit));
        }

        // Units
        if let (Some(from_rule), Some(to_rule)) = (RULES.get(from_unit).or(RULES.get(f_unit)), RULES.get(to_unit).or(RULES.get(t_unit))) {
            if from_rule.category == to_rule.category {
                let result = amount * from_rule.factor / to_rule.factor;
                return Some(self.create_entry(result, to_unit, amount, from_unit));
            }
        }

        // Currencies — live rates via Frankfurter (ECB reference rates).
        if CURRENCIES.contains(&f_unit) && CURRENCIES.contains(&t_unit) {
            if let Some(rate) = self.get_rate(f_unit, t_unit).await {
                let result = amount * rate;
                return Some(Entry {
                    name: format!("{:.2} {} = {:.2} {}", amount, f_unit.to_uppercase(), result, t_unit.to_uppercase()),
                    name_lower: "".to_string(),
                    path: format!("converter:{:.4}", result),
                    subtitle: format!("Converter • Live FX · {} {} → {}", amount, f_unit.to_uppercase(), t_unit.to_uppercase()),
                    kind: EntryKind::Plugin,
                    score: 100,
                    search_score: 1000,
                });
            }
            // Offline / API failure — fall through to no result rather than a dead entry.
        }

        None
    }

    fn create_entry(&self, result: f64, to_unit: &str, amount: f64, from_unit: &str) -> Entry {
        Entry {
            name: format!("{:.4} {}", result, to_unit),
            name_lower: "".to_string(),
            path: format!("converter:{:.4}", result),
            subtitle: format!("Converter • {} {} to {}", amount, from_unit, to_unit),
            kind: EntryKind::Plugin,
            score: 100,
            search_score: 1000, // Absolute top
        }
    }
}
