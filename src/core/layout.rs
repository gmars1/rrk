use crate::core::id::KeyId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Mapping from a `KeyId` to the character it currently produces.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutState {
    pub mapping: HashMap<KeyId, char>,
}

impl LayoutState {
    pub fn new() -> Self {
        Self {
            mapping: HashMap::new(),
        }
    }

    /// Swap the characters on two keys.
    pub fn swap(&mut self, a: KeyId, b: KeyId) {
        let ca = self.mapping.get(&a).cloned();
        let cb = self.mapping.get(&b).cloned();
        match (ca, cb) {
            (Some(ca), Some(cb)) => {
                self.mapping.insert(a, cb);
                self.mapping.insert(b, ca);
            }
            _ => {}
        }
    }
}
