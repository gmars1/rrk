use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Finger used to press a key.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Finger {
    LeftPinky,
    LeftRing,
    LeftMiddle,
    LeftIndex,
    LeftThumb,
    RightThumb,
    RightIndex,
    RightMiddle,
    RightRing,
    RightPinky,
}

/// Row of the keyboard.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RowType {
    Number,
    Top,
    Home,
    Bottom,
    Thumb,
}

/// Kind of key.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum KeyType {
    Normal,
    Modifier,
    Space,
    LayerToggle,
}

/// Mapping from platform‑specific scan codes to a unified code.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanCodeMap {
    #[serde(default)]
    pub linux_evdev: u16,
    #[serde(default)]
    pub windows_raw: u16,
    #[serde(default)]
    pub macos_hid: u16,
}

/// Definition of a single physical key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyDef {
    pub id: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub finger: Finger,
    pub row_type: RowType,
    pub key_type: KeyType,
    pub scan_codes: ScanCodeMap,
    pub label: String,
}

/// Physical keyboard model (collection of keys + geometry).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalKeyboard {
    pub id: String,
    pub name: String,
    pub keys: Vec<KeyDef>,
    pub bounds: (f32, f32),
    #[serde(skip)]
    pub key_index: HashMap<usize, usize>, // id → position in `keys`
    #[serde(skip)]
    pub render_order: Vec<usize>, // ids in the order we draw them
}

impl PhysicalKeyboard {
    /// Build fast lookup tables after deserialization.
    pub fn build_index(&mut self) {
        self.key_index = HashMap::new();
        for (i, k) in self.keys.iter().enumerate() {
            self.key_index.insert(k.id, i);
        }
        // Simple left‑to‑right, top‑to‑bottom order.
        let mut order: Vec<_> = self.keys.iter().map(|k| k.id).collect();
        order.sort_by(|a, b| {
            let ka = &self.keys[self.key_index[a]];
            let kb = &self.keys[self.key_index[b]];
            ka.y.partial_cmp(&kb.y)
                .unwrap()
                .then(ka.x.partial_cmp(&kb.x).unwrap())
        });
        self.render_order = order;
    }

    pub fn find_by_id(&self, id: usize) -> Option<&KeyDef> {
        self.key_index.get(&id).map(|&i| &self.keys[i])
    }

    pub fn render_order(&self) -> &[usize] {
        &self.render_order
    }
}
