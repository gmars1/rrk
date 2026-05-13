use eframe::egui;

use crate::core::keyboard::*;

#[derive(Debug, Clone, PartialEq)]
pub enum EditorAction {
    None,
    Back,
    Save,
}

pub struct KeyboardEditor {
    pub keyboard: PhysicalKeyboard,
    pub next_id: usize,
    pub selected_id: Option<usize>,
}

impl KeyboardEditor {
    pub fn new(template: &PhysicalKeyboard) -> Self {
        let mut keyboard = template.clone();
        keyboard.name = format!("{} (custom)", keyboard.name);
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        keyboard.id = format!("{}_{}_custom", keyboard.id, ts);
        let max_id = keyboard.keys.iter().map(|k| k.id).max().unwrap_or(0);
        Self {
            keyboard,
            next_id: max_id + 1,
            selected_id: None,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> EditorAction {
        let mut action = EditorAction::None;

        egui::TopBottomPanel::top("editor_top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("\u{2190} Back").clicked() {
                    action = EditorAction::Back;
                }
                ui.separator();
                ui.label("Keyboard:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.keyboard.name)
                        .desired_width(150.0)
                        .hint_text("Keyboard Name"),
                );
                ui.label("ID:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.keyboard.id)
                        .desired_width(120.0)
                        .hint_text("unique_id"),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("\u{1F4BE} Save").clicked() {
                        if !self.keyboard.id.is_empty() && !self.keyboard.name.is_empty() {
                            action = EditorAction::Save;
                        }
                    }
                });
            });
        });

        egui::SidePanel::right("editor_side_panel")
            .resizable(false)
            .default_width(320.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.side_panel(ui);
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.keyboard.keys.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(80.0);
                    ui.label("No keys yet. Click \"Add\" in the right panel.");
                });
            } else {
                egui::ScrollArea::both().show(ui, |ui| {
                    self.preview(ui);
                });
            }
        });

        action
    }

    fn side_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Keys");
        ui.horizontal(|ui| {
            if ui.button("+ Add").clicked() {
                self.add_key();
            }
            if ui.button("- Remove").clicked() {
                if let Some(id) = self.selected_id {
                    self.keyboard.keys.retain(|k| k.id != id);
                    self.selected_id = None;
                }
            }
        });
        ui.separator();

        let mut to_select = None;
        let mut sorted: Vec<_> = self.keyboard.keys.iter().collect();
        sorted.sort_by(|a, b| a.id.cmp(&b.id));

        egui::ScrollArea::vertical()
            .id_source("key_list_scroll")
            .max_height(180.0)
            .show(ui, |ui: &mut egui::Ui| {
                for key in &sorted {
                    let selected = self.selected_id == Some(key.id);
                    let label = format!("#{}  {}  ({:.0},{:.0})", key.id, key.label, key.x, key.y);
                    if ui.selectable_label(selected, label).clicked() {
                        to_select = Some(key.id);
                    }
                }
            });
        if let Some(id) = to_select {
            self.selected_id = Some(id);
        }

        if let Some(sel_id) = self.selected_id {
            if let Some(idx) = self.keyboard.keys.iter().position(|k| k.id == sel_id) {
                ui.separator();
                ui.heading("Properties");
                let key = &mut self.keyboard.keys[idx];
                Self::edit_key_properties(ui, key);
            } else {
                self.selected_id = None;
            }
        }

        ui.separator();
        ui.heading("Board");
        ui.horizontal(|ui| {
            ui.label("Bounds W:");
            ui.add(egui::DragValue::new(&mut self.keyboard.bounds.0).speed(0.5));
            ui.label("H:");
            ui.add(egui::DragValue::new(&mut self.keyboard.bounds.1).speed(0.5));
        });
        let total_keys = self.keyboard.keys.len();
        ui.label(format!("Total keys: {}", total_keys));
    }

    fn edit_key_properties(ui: &mut egui::Ui, key: &mut KeyDef) {
        ui.horizontal(|ui| {
            ui.label("Label:");
            ui.add(
                egui::TextEdit::singleline(&mut key.label)
                    .desired_width(80.0)
                    .hint_text("key label"),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Pos:");
            ui.add(egui::DragValue::new(&mut key.x).speed(0.1).prefix("X:"));
            ui.add(egui::DragValue::new(&mut key.y).speed(0.1).prefix("Y:"));
        });
        ui.horizontal(|ui| {
            ui.label("Size:");
            ui.add(
                egui::DragValue::new(&mut key.width)
                    .speed(0.05)
                    .prefix("W:"),
            );
            ui.add(
                egui::DragValue::new(&mut key.height)
                    .speed(0.05)
                    .prefix("H:"),
            );
        });

        let all_fingers = [
            Finger::LeftPinky,
            Finger::LeftRing,
            Finger::LeftMiddle,
            Finger::LeftIndex,
            Finger::LeftThumb,
            Finger::RightThumb,
            Finger::RightIndex,
            Finger::RightMiddle,
            Finger::RightRing,
            Finger::RightPinky,
        ];
        let mut new_finger = key.finger;
        ui.horizontal(|ui| {
            ui.label("Finger:");
            egui::ComboBox::from_id_source("finger_combo")
                .selected_text(format!("{:?}", new_finger))
                .show_ui(ui, |ui| {
                    for f in &all_fingers {
                        ui.selectable_value(&mut new_finger, *f, format!("{:?}", f));
                    }
                });
        });
        if new_finger != key.finger {
            key.finger = new_finger;
        }

        let all_rows = [
            RowType::Number,
            RowType::Top,
            RowType::Home,
            RowType::Bottom,
            RowType::Thumb,
        ];
        let mut new_row = key.row_type;
        ui.horizontal(|ui| {
            ui.label("Row:");
            egui::ComboBox::from_id_source("row_combo")
                .selected_text(format!("{:?}", new_row))
                .show_ui(ui, |ui| {
                    for r in &all_rows {
                        ui.selectable_value(&mut new_row, *r, format!("{:?}", r));
                    }
                });
        });
        if new_row != key.row_type {
            key.row_type = new_row;
        }

        let all_types = [
            KeyType::Normal,
            KeyType::Modifier,
            KeyType::Space,
            KeyType::LayerToggle,
        ];
        let mut new_type = key.key_type;
        ui.horizontal(|ui| {
            ui.label("Type:");
            egui::ComboBox::from_id_source("type_combo")
                .selected_text(format!("{:?}", new_type))
                .show_ui(ui, |ui| {
                    for t in &all_types {
                        ui.selectable_value(&mut new_type, *t, format!("{:?}", t));
                    }
                });
        });
        if new_type != key.key_type {
            key.key_type = new_type;
        }

        ui.label("Scan codes:");
        let sc = &mut key.scan_codes;
        ui.horizontal(|ui| {
            ui.label("Linux:");
            ui.add(
                egui::DragValue::new(&mut sc.linux_evdev)
                    .range(0..=65535)
                    .speed(1.0),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Win:");
            ui.add(
                egui::DragValue::new(&mut sc.windows_raw)
                    .range(0..=65535)
                    .speed(1.0),
            );
        });
        ui.horizontal(|ui| {
            ui.label("macOS:");
            ui.add(
                egui::DragValue::new(&mut sc.macos_hid)
                    .range(0..=65535)
                    .speed(1.0),
            );
        });
    }

    fn add_key(&mut self) {
        let id = self.next_id;
        self.next_id += 1;
        let key = KeyDef {
            id,
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            finger: Finger::LeftPinky,
            row_type: RowType::Home,
            key_type: KeyType::Normal,
            scan_codes: ScanCodeMap::default(),
            label: "?".to_string(),
        };
        self.keyboard.keys.push(key);
        self.selected_id = Some(id);
    }

    fn preview(&mut self, ui: &mut egui::Ui) {
        let cell_w = 40.0;
        let spacing = 5.0;
        let step = cell_w + spacing;

        let max_x = self
            .keyboard
            .keys
            .iter()
            .map(|k| (k.x + k.width) * step)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(800.0);
        let max_y = self
            .keyboard
            .keys
            .iter()
            .map(|k| (k.y + k.height) * step)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(400.0);

        let total_size = egui::vec2(max_x + 40.0, max_y + 40.0);
        let (response, painter) = ui.allocate_painter(total_size, egui::Sense::click_and_drag());
        let origin = response.rect.min + egui::vec2(20.0, 20.0);

        let mut key_rects: Vec<(usize, egui::Rect)> = Vec::new();

        for key in &self.keyboard.keys {
            let x = origin.x + key.x * step;
            let y = origin.y + key.y * step;
            let w = key.width * step - spacing;
            let h = key.height * step - spacing;
            let rect =
                egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w.max(4.0), h.max(4.0)));
            key_rects.push((key.id, rect));

            let is_selected = self.selected_id == Some(key.id);
            let color = if is_selected {
                egui::Color32::from_rgb(70, 130, 240)
            } else {
                egui::Color32::from_gray(80)
            };

            painter.rect_filled(rect, 4.0, color);
            painter.rect_stroke(
                rect,
                4.0,
                egui::Stroke::new(1.0, egui::Color32::from_gray(140)),
            );

            let label = if key.label == " " {
                "\u{23B5}"
            } else {
                &key.label
            };
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(14.0),
                egui::Color32::WHITE,
            );
        }

        if response.dragged() {
            if self.selected_id.is_none() {
                let pos = ui.ctx().input(|i| i.pointer.interact_pos());
                if let Some(pos) = pos {
                    for (id, rect) in &key_rects {
                        if rect.contains(pos) {
                            self.selected_id = Some(*id);
                            break;
                        }
                    }
                }
            }
            if let Some(sel_id) = self.selected_id {
                if let Some(key) = self.keyboard.keys.iter_mut().find(|k| k.id == sel_id) {
                    let delta = response.drag_delta();
                    key.x += delta.x / step;
                    key.y += delta.y / step;
                }
            }
        }

        if response.clicked() {
            let pos = ui.ctx().input(|i| i.pointer.interact_pos());
            if let Some(pos) = pos {
                for (id, rect) in &key_rects {
                    if rect.contains(pos) {
                        self.selected_id = Some(*id);
                        break;
                    }
                }
            }
        }
    }
}
