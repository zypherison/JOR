use crate::plugins::Plugin;
use crate::models::{Entry, EntryKind};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Forecasts are cached per city for 10 minutes to stay polite to the API.
const FORECAST_CACHE_TTL: Duration = Duration::from_secs(10 * 60);

pub struct WeatherPlugin {
    /// city (lowercase) -> (fetched_at, result_name, result_subtitle)
    cache: Mutex<HashMap<String, (Instant, String, String)>>,
}

impl WeatherPlugin {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// WMO weather code → human readable description.
    fn code_description(code: i64) -> &'static str {
        match code {
            0 => "Clear sky",
            1 => "Mainly clear",
            2 => "Partly cloudy",
            3 => "Overcast",
            45 | 48 => "Foggy",
            51 | 53 | 55 => "Drizzle",
            56 | 57 => "Freezing drizzle",
            61 | 63 | 65 => "Rain",
            66 | 67 => "Freezing rain",
            71 | 73 | 75 => "Snow",
            77 => "Snow grains",
            80 | 81 | 82 => "Rain showers",
            85 | 86 => "Snow showers",
            95 => "Thunderstorm",
            96 | 99 => "Thunderstorm with hail",
            _ => "Unknown",
        }
    }

    /// Resolve coordinates for a city name via Open-Meteo geocoding.
    async fn geocode(&self, city: &str) -> Option<(f64, f64, String)> {
        let url = format!(
            "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=en&format=json",
            city.trim()
        );
        let resp = reqwest::get(&url).await.ok()?;
        let json: serde_json::Value = resp.json().await.ok()?;
        let first = json["results"].as_array()?.first()?;
        let lat = first["latitude"].as_f64()?;
        let lon = first["longitude"].as_f64()?;
        let name = first["name"].as_str()?.to_string();
        Some((lat, lon, name))
    }

    /// Best-effort location from the public IP (no API key required).
    async fn locate_by_ip(&self) -> Option<(f64, f64, String)> {
        let resp = reqwest::get("https://ipapi.co/json/").await.ok()?;
        let json: serde_json::Value = resp.json().await.ok()?;
        let lat = json["latitude"].as_f64()?;
        let lon = json["longitude"].as_f64()?;
        let city = json["city"].as_str().unwrap_or("your location").to_string();
        Some((lat, lon, city))
    }

    /// Fetch current conditions for a city, producing the display strings.
    async fn fetch_conditions(&self, city: &str) -> Option<(String, String)> {
        let (lat, lon, city_name) = match self.geocode(city).await {
            Some(geo) => geo,
            None => self.locate_by_ip().await?,
        };

        let url = format!(
            "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,apparent_temperature,weather_code,wind_speed_10m",
            lat, lon
        );
        let resp = reqwest::get(&url).await.ok()?;
        let json: serde_json::Value = resp.json().await.ok()?;
        let current = &json["current"];

        let temp = current["temperature_2m"].as_f64()?;
        let feels = current["apparent_temperature"].as_f64()?;
        let wind = current["wind_speed_10m"].as_f64().unwrap_or(0.0);
        let code = current["weather_code"].as_i64().unwrap_or(0);
        let desc = Self::code_description(code);

        let name = format!("{:.0}°C • {} in {}", temp, desc, city_name);
        let subtitle = format!(
            "Weather • Feels like {:.0}°C · Wind {:.0} km/h · tap for full forecast",
            feels, wind
        );
        Some((name, subtitle))
    }
}

#[async_trait]
impl Plugin for WeatherPlugin {
    fn id(&self) -> &str { "weather" }
    fn name(&self) -> &str { "Hyper-Local Weather" }
    fn description(&self) -> &str { "Live weather for any city, powered by Open-Meteo." }
    fn trigger_hint(&self) -> &str { "weather <city>" }
    fn is_pro(&self) -> bool { true }

    async fn search(&self, query: &str, _mode: &str) -> Vec<Entry> {
        let q = query.trim().to_lowercase();
        if q.is_empty() { return vec![]; }

        // Match "weather <city>", "<city> weather", or "wt <city>".
        let city = if let Some(rest) = q.strip_prefix("weather") {
            rest.trim()
        } else if let Some(rest) = q.strip_prefix("wt") {
            rest.trim()
        } else if q.ends_with("weather") {
            q[..q.len() - "weather".len()].trim()
        } else {
            return vec![];
        };

        let cache_key = if city.is_empty() { "@ip".to_string() } else { city.to_string() };

        // Serve from cache when fresh.
        if let Ok(cache) = self.cache.lock() {
            if let Some((fetched_at, name, subtitle)) = cache.get(&cache_key) {
                if fetched_at.elapsed() < FORECAST_CACHE_TTL {
                    return vec![self.entry(name, subtitle)];
                }
            }
        }

        let fetched = self.fetch_conditions(city).await;
        match fetched {
            Some((name, subtitle)) => {
                if let Ok(mut cache) = self.cache.lock() {
                    cache.insert(cache_key, (Instant::now(), name.clone(), subtitle.clone()));
                }
                vec![self.entry(&name, &subtitle)]
            }
            None => {
                // No weather available (offline / unknown city) — show a useful hint.
                vec![Entry {
                    name: "Weather unavailable".to_string(),
                    name_lower: "weather".to_string(),
                    path: "weather:open".to_string(),
                    subtitle: "Weather • Try \"weather london\" — needs an internet connection".to_string(),
                    kind: EntryKind::Plugin,
                    score: 60,
                    search_score: 800,
                }]
            }
        }
    }

    async fn execute(&self, action_id: &str) -> Result<(), String> {
        // Open a forecast search for the requested city (or a generic page).
        let query = action_id.strip_prefix("open:").unwrap_or("");
        let url = if query.is_empty() {
            "https://weather.com".to_string()
        } else {
            format!("https://www.google.com/search?q=weather+in+{}", query)
        };
        opener::open(&url).map_err(|e| e.to_string())
    }
}

impl WeatherPlugin {
    fn entry(&self, name: &str, subtitle: &str) -> Entry {
        Entry {
            name: name.to_string(),
            name_lower: "weather".to_string(),
            // Encode the city so the execute action can open the right forecast.
            path: format!(
                "weather:open:{}",
                name.split(" in ").last().unwrap_or("").trim()
            ),
            subtitle: subtitle.to_string(),
            kind: EntryKind::Plugin,
            score: 100,
            search_score: 1000,
        }
    }
}
