use crate::core::stats::LanguageStats;
use dirs::config_dir;
use std::fs;
use std::path::PathBuf;

/// Load persisted stats from `~/.config/key-optimizer/stats.json`.
pub fn load_stats() -> LanguageStats {
    let mut path = config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("key-optimizer");
    path.push("stats.json");
    if let Ok(data) = fs::read_to_string(&path) {
        if let Ok(stats) = serde_json::from_str(&data) {
            return stats;
        }
    }
    LanguageStats::default()
}

/// Save stats to the same location.
pub fn save_stats(stats: &LanguageStats) {
    let mut dir = config_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.push("key-optimizer");
    let _ = fs::create_dir_all(&dir);
    let mut file = dir;
    file.push("stats.json");
    if let Ok(txt) = serde_json::to_string_pretty(stats) {
        let _ = fs::write(file, txt);
    }
}
