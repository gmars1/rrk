use eframe::egui;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

use crate::core::id::{CharId, KeyId};
use crate::core::keyboard::PhysicalKeyboard;
use crate::core::layout::LayoutState;
use crate::core::stats::LanguageStats;
use crate::platform::{CoreEvent, Listener};

pub mod editor;
use editor::{EditorAction, KeyboardEditor};

pub struct App {
    keyboards: Vec<PhysicalKeyboard>,
    current_index: usize,
    layout_state: LayoutState,
    scan_to_char: HashMap<u16, CharId>,
    stats: Arc<Mutex<LanguageStats>>,
    _listener: Box<dyn Listener>,
    rx: broadcast::Receiver<CoreEvent>,
    last_save: Instant,
    previous_char: Option<CharId>,
    last_event_time: Instant,
    evdev_alive: bool,
    show_freq: bool,
    show_editor: bool,
    editor: Option<KeyboardEditor>,
}

impl App {
    fn build_mappings(kb: &PhysicalKeyboard) -> (HashMap<KeyId, char>, HashMap<u16, CharId>) {
        let mut mapping = HashMap::new();
        let mut scan_to_char = HashMap::new();
        for k in &kb.keys {
            if k.label.chars().count() != 1 {
                continue;
            }
            let ch = k.label.chars().next().unwrap();
            if ch != ' ' && !ch.is_alphabetic() {
                continue;
            }
            let cid = CharId::new(ch.to_lowercase().next().unwrap_or(ch));
            mapping.insert(KeyId::new(k.id), ch);
            let sc = k.scan_codes.linux_evdev;
            if sc != 0 {
                scan_to_char.insert(sc, cid);
            }
        }
        (mapping, scan_to_char)
    }

    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        stats: LanguageStats,
        keyboards: Vec<PhysicalKeyboard>,
        rx: broadcast::Receiver<CoreEvent>,
        listener: Box<dyn Listener>,
    ) -> Self {
        let mut keyboard = keyboards
            .first()
            .cloned()
            .expect("No keyboard layouts found");
        keyboard.build_index();
        let (mapping, scan_to_char) = Self::build_mappings(&keyboard);

        Self {
            keyboards,
            current_index: 0,
            layout_state: LayoutState { mapping },
            scan_to_char,
            stats: Arc::new(Mutex::new(stats)),
            _listener: listener,
            rx,
            last_save: Instant::now(),
            previous_char: None,
            last_event_time: Instant::now(),
            evdev_alive: false,
            show_freq: false,
            show_editor: false,
            editor: None,
        }
        .auto_detect_layout()
    }

    fn auto_detect_layout(mut self) -> Self {
        let detected = detect_system_layout();
        if let Some(ref id) = detected {
            for (i, kb) in self.keyboards.iter().enumerate() {
                if &kb.id == id {
                    self.switch_layout(i);
                    break;
                }
            }
        }
        self
    }

    fn current_keyboard(&self) -> &PhysicalKeyboard {
        &self.keyboards[self.current_index]
    }

    fn heat_map(&self) -> HashMap<usize, f32> {
        let stats = self.stats.lock().unwrap();
        let heat = stats.unigrams.heat_map();
        let mut map = HashMap::new();
        for (char_id, weight) in &heat {
            for (kid, ch) in &self.layout_state.mapping {
                if &char_id.as_char() == &ch.to_lowercase().next().unwrap_or(*ch) {
                    let entry = map.entry(kid.0).or_insert(0.0f32);
                    *entry = entry.max(*weight);
                }
            }
        }
        // Re-normalize by max of only mapped keys
        let local_max = map
            .values()
            .copied()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(1.0)
            .max(0.001);
        for v in map.values_mut() {
            *v /= local_max;
        }
        map
    }

    fn record_char(&mut self, ch: char) {
        let ch_lower = ch.to_lowercase().next().unwrap_or(ch);
        if ch_lower != ' ' && !ch_lower.is_alphabetic() {
            return;
        }
        let cid = CharId::new(ch_lower);
        let mut stats = self.stats.lock().unwrap();
        stats.record_unigram(cid);
        if let Some(prev) = self.previous_char {
            stats.record_bigram(prev, cid);
        }
        self.previous_char = Some(cid);
    }

    fn process_events(&mut self) {
        let mut had_real = false;
        while let Ok(event) = self.rx.try_recv() {
            had_real = true;
            let CoreEvent::KeyPress(sc) = event;
            if let Some(&cid) = self.scan_to_char.get(&sc) {
                self.record_char(cid.as_char());
            }
        }

        if had_real {
            self.evdev_alive = true;
            self.last_event_time = Instant::now();
        }
    }

    fn process_egui_input(&mut self, ctx: &egui::Context) {
        if self.evdev_alive {
            return;
        }
        use egui::Event;
        let had = ctx.input(|i| {
            let mut any = false;
            for ev in &i.events {
                if let Event::Text(txt) = ev {
                    for ch in txt.chars() {
                        any = true;
                        self.record_char(ch);
                    }
                }
            }
            any
        });
        if had {
            self.last_event_time = Instant::now();
        }
    }

    fn maintain(&mut self) {
        if self.last_save.elapsed() > Duration::from_secs(30) {
            crate::storage::save_stats(&self.stats.lock().unwrap());
            self.last_save = Instant::now();
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_events();
        self.process_egui_input(ctx);
        self.maintain();

        if self.show_editor {
            let action = self
                .editor
                .as_mut()
                .map(|e| e.show(ctx))
                .unwrap_or(EditorAction::None);
            match action {
                EditorAction::Back => {
                    self.show_editor = false;
                    self.editor = None;
                }
                EditorAction::Save => {
                    let saved = self.editor.as_ref().map(|e| {
                        let mut kb = e.keyboard.clone();
                        kb.build_index();
                        kb
                    });
                    if let Some(kb) = saved {
                        crate::storage::save_layout(&kb);
                        if let Some(pos) = self.keyboards.iter().position(|k| k.id == kb.id) {
                            self.keyboards[pos] = kb.clone();
                        } else {
                            self.keyboards.push(kb.clone());
                        }
                        if let Some(new_idx) = self.keyboards.iter().position(|k| k.id == kb.id) {
                            self.switch_layout(new_idx);
                        }
                    }
                    self.show_editor = false;
                    self.editor = None;
                }
                _ => {}
            }
            return;
        }

        ctx.request_repaint_after(Duration::from_millis(100));

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Layout:");
                let names: Vec<String> = self.keyboards.iter().map(|k| k.name.clone()).collect();
                egui::ComboBox::from_id_source("layout_picker")
                    .selected_text(&names[self.current_index])
                    .show_ui(ui, |ui| {
                        for (i, name) in names.iter().enumerate() {
                            if ui.selectable_label(i == self.current_index, name).clicked() {
                                self.switch_layout(i);
                            }
                        }
                    });
                ui.separator();
                let total = self.stats.lock().unwrap().total_events();
                ui.label(format!("Events: {}", total));
                ui.separator();
                if ui.button("📊").clicked() {
                    self.show_freq = !self.show_freq;
                }
                ui.separator();
                if ui.button("+ Create").clicked() {
                    let template = self.current_keyboard().clone();
                    self.show_editor = true;
                    self.editor = Some(KeyboardEditor::new(&template));
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw_heatmap(ui);
        });

        if self.show_freq {
            let layout_chars: std::collections::HashSet<char> = self
                .layout_state
                .mapping
                .values()
                .map(|ch| ch.to_lowercase().next().unwrap_or(*ch))
                .collect();

            egui::Window::new("Letter frequencies")
                .id("freq_window".into())
                .default_size([400.0, 300.0])
                .show(ctx, |ui| {
                    let stats = self.stats.lock().unwrap();
                    let total = stats.unigrams.total();
                    if total == 0 {
                        ui.label("(no data yet)");
                        return;
                    }
                    let mut entries: Vec<(char, u64)> = stats
                        .unigrams
                        .counts
                        .iter()
                        .filter(|(cid, _)| layout_chars.contains(&cid.as_char()))
                        .map(|(cid, &c)| (cid.as_char(), c))
                        .collect();
                    entries.sort_by(|a, b| b.1.cmp(&a.1));
                    let max_w = ui.available_width() - 140.0;
                    egui::Grid::new("freq_grid")
                        .striped(true)
                        .min_col_width(30.0)
                        .show(ui, |ui| {
                            ui.strong("Char");
                            ui.strong("Count");
                            ui.strong("%");
                            ui.strong("");
                            ui.end_row();
                            for (ch, count) in &entries {
                                let pct = *count as f64 / total as f64 * 100.0;
                                let label = if *ch == ' ' {
                                    "⎵".to_string()
                                } else {
                                    ch.to_string()
                                };
                                ui.label(&label);
                                ui.label(format!("{}", count));
                                ui.label(format!("{:.2}%", pct));
                                let bar_w = max_w * (pct as f32 / 100.0);
                                ui.colored_label(
                                    egui::Color32::from_rgb(
                                        (255.0 * pct as f32 / 15.0).min(255.0) as u8,
                                        100,
                                        50,
                                    ),
                                    "█".repeat((bar_w / 8.0) as usize),
                                );
                                ui.end_row();
                            }
                        });
                });
        }
    }
}

impl App {
    fn switch_layout(&mut self, index: usize) {
        if index >= self.keyboards.len() {
            return;
        }
        self.current_index = index;
        let kb = &self.keyboards[index];
        let (mapping, scan_to_char) = Self::build_mappings(kb);
        self.layout_state = LayoutState { mapping };
        self.scan_to_char = scan_to_char;
    }

    fn draw_heatmap(&self, ui: &mut egui::Ui) {
        let heat = self.heat_map();
        let cell = egui::vec2(40.0, 40.0);
        let offset_x = 20.0;
        let offset_y = 20.0;

        for key in &self.current_keyboard().keys {
            let pos = egui::pos2(offset_x + key.x * 45.0, offset_y + key.y * 45.0);
            let rect = egui::Rect::from_min_size(pos, cell);
            let weight = heat.get(&key.id).copied().unwrap_or(0.0);
            let col =
                egui::Color32::from_rgb((255.0 * weight) as u8, (255.0 * (1.0 - weight)) as u8, 0);
            ui.painter().rect_filled(rect, 4.0, col);
            let label = if key.label == " " { "⎵" } else { &key.label };
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(14.0),
                egui::Color32::WHITE,
            );
        }
    }
}

fn detect_system_layout() -> Option<String> {
    // Try setxkbmap -query (works on X11 and some Wayland compositors)
    if let Ok(out) = std::process::Command::new("setxkbmap")
        .arg("-query")
        .output()
    {
        if let Ok(text) = String::from_utf8(out.stdout) {
            for line in text.lines() {
                let line = line.trim();
                if let Some(val) = line.strip_prefix("layout:") {
                    let id = val.trim().split(',').next().unwrap_or("").trim();
                    let mapped = match id {
                        "us" | "us(intl)" => "qwerty",
                        "ru" | "ru( phonetic)" => "ru",
                        "gb" | "uk" => "qwerty",
                        "de" => "qwerty",
                        "fr" => "qwerty",
                        other => other,
                    };
                    return Some(mapped.to_string());
                }
            }
        }
    }

    // Fallback: XKB_DEFAULT_LAYOUT env var
    if let Ok(var) = std::env::var("XKB_DEFAULT_LAYOUT") {
        let id = var.split(',').next().unwrap_or("").trim();
        let mapped = match id {
            "us" => "qwerty",
            "ru" => "ru",
            other => other,
        };
        return Some(mapped.to_string());
    }

    None
}

impl Drop for App {
    fn drop(&mut self) {
        if let Ok(stats) = self.stats.lock() {
            crate::storage::save_stats(&stats);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::keyboard::PhysicalKeyboard;

    #[test]
    fn test_scan_code_mapping() {
        let json = include_str!("../assets/layouts/qwerty.json");
        let mut kb: PhysicalKeyboard = serde_json::from_str(json).expect("valid layout");
        kb.build_index();
        let (mapping, scan_to_char) = App::build_mappings(&kb);

        // Letter keys should map correctly
        assert_eq!(scan_to_char.get(&30), Some(&CharId::new('a'))); // KEY_A -> 'a'
        assert_eq!(scan_to_char.get(&48), Some(&CharId::new('b'))); // KEY_B -> 'b'
        assert_eq!(scan_to_char.get(&46), Some(&CharId::new('c'))); // KEY_C -> 'c'
        assert_eq!(scan_to_char.get(&32), Some(&CharId::new('d'))); // KEY_D -> 'd'
        assert_eq!(scan_to_char.get(&18), Some(&CharId::new('e'))); // KEY_E -> 'e'
        assert_eq!(scan_to_char.get(&33), Some(&CharId::new('f'))); // KEY_F -> 'f'
        assert_eq!(scan_to_char.get(&34), Some(&CharId::new('g'))); // KEY_G -> 'g'
        assert_eq!(scan_to_char.get(&35), Some(&CharId::new('h'))); // KEY_H -> 'h'
        assert_eq!(scan_to_char.get(&23), Some(&CharId::new('i'))); // KEY_I -> 'i'
        assert_eq!(scan_to_char.get(&36), Some(&CharId::new('j'))); // KEY_J -> 'j'
        assert_eq!(scan_to_char.get(&37), Some(&CharId::new('k'))); // KEY_K -> 'k'
        assert_eq!(scan_to_char.get(&38), Some(&CharId::new('l'))); // KEY_L -> 'l'
        assert_eq!(scan_to_char.get(&50), Some(&CharId::new('m'))); // KEY_M -> 'm'
        assert_eq!(scan_to_char.get(&49), Some(&CharId::new('n'))); // KEY_N -> 'n'
        assert_eq!(scan_to_char.get(&24), Some(&CharId::new('o'))); // KEY_O -> 'o'
        assert_eq!(scan_to_char.get(&25), Some(&CharId::new('p'))); // KEY_P -> 'p'
        assert_eq!(scan_to_char.get(&16), Some(&CharId::new('q'))); // KEY_Q -> 'q'
        assert_eq!(scan_to_char.get(&19), Some(&CharId::new('r'))); // KEY_R -> 'r'
        assert_eq!(scan_to_char.get(&31), Some(&CharId::new('s'))); // KEY_S -> 's'
        assert_eq!(scan_to_char.get(&20), Some(&CharId::new('t'))); // KEY_T -> 't'
        assert_eq!(scan_to_char.get(&22), Some(&CharId::new('u'))); // KEY_U -> 'u'
        assert_eq!(scan_to_char.get(&47), Some(&CharId::new('v'))); // KEY_V -> 'v'
        assert_eq!(scan_to_char.get(&17), Some(&CharId::new('w'))); // KEY_W -> 'w'
        assert_eq!(scan_to_char.get(&45), Some(&CharId::new('x'))); // KEY_X -> 'x'
        assert_eq!(scan_to_char.get(&21), Some(&CharId::new('y'))); // KEY_Y -> 'y'
        assert_eq!(scan_to_char.get(&44), Some(&CharId::new('z'))); // KEY_Z -> 'z'

        // Space
        assert_eq!(scan_to_char.get(&57), Some(&CharId::new(' '))); // KEY_SPACE -> ' '

        // Modifier keys should NOT be in the mapping
        assert!(scan_to_char.get(&42).is_none()); // KEY_LEFTSHIFT
        assert!(scan_to_char.get(&58).is_none()); // KEY_CAPS
        assert!(scan_to_char.get(&15).is_none()); // KEY_TAB
        assert!(scan_to_char.get(&29).is_none()); // KEY_LEFTCTRL
        assert!(scan_to_char.get(&56).is_none()); // KEY_LEFTALT

        // Number keys should NOT be in the mapping
        assert!(scan_to_char.get(&2).is_none()); // KEY_1
        assert!(scan_to_char.get(&3).is_none()); // KEY_2
        assert!(scan_to_char.get(&11).is_none()); // KEY_0

        // Heat map mapping should have the same letters
        assert_eq!(mapping.get(&KeyId::new(29)), Some(&'A')); // key 29 = A
        assert_eq!(mapping.get(&KeyId::new(56)), Some(&' ')); // key 56 = space

        // Modifier keys should NOT be in heat map mapping
        assert!(mapping.get(&KeyId::new(41)).is_none()); // Shift
        assert!(mapping.get(&KeyId::new(28)).is_none()); // Caps
        assert!(mapping.get(&KeyId::new(14)).is_none()); // Tab
    }
}
