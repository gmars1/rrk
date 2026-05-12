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
            let ch_lower = ch.to_ascii_lowercase();
            let cid = if ch_lower == ' ' {
                CharId::new(26)
            } else if ch_lower.is_ascii_alphabetic() {
                CharId::new((ch_lower as u8 - b'a') as usize)
            } else {
                continue;
            };
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
        }
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
                let ch_lower = ch.to_ascii_lowercase();
                let expected = if ch_lower == ' ' {
                    26
                } else {
                    (ch_lower as u8).wrapping_sub(b'a') as usize
                };
                if char_id.as_usize() == expected {
                    map.insert(kid.0, *weight);
                }
            }
        }
        map
    }

    fn record_char(&mut self, ch: char) {
        let ch_lower = ch.to_ascii_lowercase();
        let cid = if ch_lower == ' ' {
            CharId::new(26)
        } else if ch_lower.is_ascii_alphabetic() {
            CharId::new((ch_lower as u8 - b'a') as usize)
        } else {
            return;
        };
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
                self.record_char(match cid.as_usize() {
                    26 => ' ',
                    n => (n as u8 + b'a') as char,
                });
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
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw_heatmap(ui);
        });
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
        assert_eq!(scan_to_char.get(&30), Some(&CharId::new(0))); // KEY_A -> 'a'
        assert_eq!(scan_to_char.get(&48), Some(&CharId::new(1))); // KEY_B -> 'b'
        assert_eq!(scan_to_char.get(&46), Some(&CharId::new(2))); // KEY_C -> 'c'
        assert_eq!(scan_to_char.get(&32), Some(&CharId::new(3))); // KEY_D -> 'd'
        assert_eq!(scan_to_char.get(&18), Some(&CharId::new(4))); // KEY_E -> 'e'
        assert_eq!(scan_to_char.get(&33), Some(&CharId::new(5))); // KEY_F -> 'f'
        assert_eq!(scan_to_char.get(&34), Some(&CharId::new(6))); // KEY_G -> 'g'
        assert_eq!(scan_to_char.get(&35), Some(&CharId::new(7))); // KEY_H -> 'h'
        assert_eq!(scan_to_char.get(&23), Some(&CharId::new(8))); // KEY_I -> 'i'
        assert_eq!(scan_to_char.get(&36), Some(&CharId::new(9))); // KEY_J -> 'j'
        assert_eq!(scan_to_char.get(&37), Some(&CharId::new(10))); // KEY_K -> 'k'
        assert_eq!(scan_to_char.get(&38), Some(&CharId::new(11))); // KEY_L -> 'l'
        assert_eq!(scan_to_char.get(&50), Some(&CharId::new(12))); // KEY_M -> 'm'
        assert_eq!(scan_to_char.get(&49), Some(&CharId::new(13))); // KEY_N -> 'n'
        assert_eq!(scan_to_char.get(&24), Some(&CharId::new(14))); // KEY_O -> 'o'
        assert_eq!(scan_to_char.get(&25), Some(&CharId::new(15))); // KEY_P -> 'p'
        assert_eq!(scan_to_char.get(&16), Some(&CharId::new(16))); // KEY_Q -> 'q'
        assert_eq!(scan_to_char.get(&19), Some(&CharId::new(17))); // KEY_R -> 'r'
        assert_eq!(scan_to_char.get(&31), Some(&CharId::new(18))); // KEY_S -> 's'
        assert_eq!(scan_to_char.get(&20), Some(&CharId::new(19))); // KEY_T -> 't'
        assert_eq!(scan_to_char.get(&22), Some(&CharId::new(20))); // KEY_U -> 'u'
        assert_eq!(scan_to_char.get(&47), Some(&CharId::new(21))); // KEY_V -> 'v'
        assert_eq!(scan_to_char.get(&17), Some(&CharId::new(22))); // KEY_W -> 'w'
        assert_eq!(scan_to_char.get(&45), Some(&CharId::new(23))); // KEY_X -> 'x'
        assert_eq!(scan_to_char.get(&21), Some(&CharId::new(24))); // KEY_Y -> 'y'
        assert_eq!(scan_to_char.get(&44), Some(&CharId::new(25))); // KEY_Z -> 'z'

        // Space
        assert_eq!(scan_to_char.get(&57), Some(&CharId::new(26))); // KEY_SPACE -> ' '

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
