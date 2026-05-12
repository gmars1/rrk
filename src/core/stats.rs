use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::id::CharId;

/// Frequency counts for a single language.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Frequency {
    pub counts: HashMap<CharId, u64>,
}

impl Frequency {
    pub fn record(&mut self, ch: CharId) {
        *self.counts.entry(ch).or_insert(0) += 1;
    }

    pub fn total(&self) -> u64 {
        self.counts.values().sum()
    }

    pub fn heat_map(&self) -> HashMap<CharId, f32> {
        let max = self.counts.values().copied().max().unwrap_or(1).max(1) as f32;
        self.counts
            .iter()
            .map(|(&k, &v)| (k, v as f32 / max))
            .collect()
    }
}

/// Unigram + bigram statistics for a language.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LanguageStats {
    pub unigrams: Frequency,
    pub bigrams: HashMap<(CharId, CharId), u64>,
}

impl LanguageStats {
    pub fn record_unigram(&mut self, ch: CharId) {
        self.unigrams.record(ch);
    }

    pub fn record_bigram(&mut self, a: CharId, b: CharId) {
        *self.bigrams.entry((a, b)).or_insert(0) += 1;
    }

    pub fn total_events(&self) -> u64 {
        self.unigrams.total()
    }
}
