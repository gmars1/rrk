use crate::core::keyboard::PhysicalKeyboard;
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

fn layouts_dir() -> PathBuf {
    let mut dir = config_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.push("key-optimizer");
    dir.push("layouts");
    dir
}

/// Save a custom keyboard layout to the config directory.
pub fn save_layout(kb: &PhysicalKeyboard) {
    let dir = layouts_dir();
    let _ = fs::create_dir_all(&dir);
    let mut file = dir;
    file.push(format!("{}.json", kb.id));
    if let Ok(json) = serde_json::to_string_pretty(kb) {
        let _ = fs::write(file, json);
    }
}

/// Load custom keyboard layouts from the config directory.
pub fn load_custom_layouts() -> Vec<PhysicalKeyboard> {
    let dir = layouts_dir();
    let mut layouts = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                if let Ok(data) = fs::read_to_string(&path) {
                    if let Ok(mut kb) = serde_json::from_str::<PhysicalKeyboard>(&data) {
                        kb.build_index();
                        if !layouts.iter().any(|k: &PhysicalKeyboard| k.id == kb.id) {
                            layouts.push(kb);
                        }
                    }
                }
            }
        }
    }
    layouts
}
