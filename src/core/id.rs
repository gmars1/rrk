use serde::{Deserialize, Serialize};

/// Compact identifier for a physical key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyId(pub usize);

impl KeyId {
    pub fn new(v: usize) -> Self {
        Self(v)
    }
    pub fn as_usize(self) -> usize {
        self.0
    }
}

/// Compact identifier for a character (used in bigrams).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CharId(pub usize);

impl CharId {
    pub fn new(v: usize) -> Self {
        Self(v)
    }
    pub fn as_usize(self) -> usize {
        self.0
    }
}
