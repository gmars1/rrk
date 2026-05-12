use crate::core::id::KeyId;
use crate::core::keyboard::PhysicalKeyboard;
use crate::core::layout::LayoutState;
use crate::core::stats::LanguageStats;

/// Weights for the different scoring components.
#[derive(Debug, Clone)]
pub struct ScoringWeights {
    pub sfb_weight: f32,
    pub lateral_stretch_weight: f32,
    pub row_jump_weight: f32,
    pub finger_load_deviation_weight: f32,
    pub hand_alternation_bonus: f32,
    pub inward_roll_bonus: f32,
    pub outward_roll_penalty: f32,
    pub layer_activation_cost: f32,
    pub distance_weight: f32,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            sfb_weight: 1.0,
            lateral_stretch_weight: 1.0,
            row_jump_weight: 1.0,
            finger_load_deviation_weight: 1.0,
            hand_alternation_bonus: 0.5,
            inward_roll_bonus: 0.2,
            outward_roll_penalty: 0.3,
            layer_activation_cost: 0.1,
            distance_weight: 0.5,
        }
    }
}

/// Context that does not change during a single optimisation run.
pub struct StaticContext<'a> {
    pub keyboard: &'a PhysicalKeyboard,
    pub stats: &'a LanguageStats,
    pub weights: &'a ScoringWeights,
}

/// Compute the total score for a given layout state.
pub fn compute_score(ctx: &StaticContext, _state: &LayoutState) -> f64 {
    let total_events = ctx.stats.total_events() as f64;
    if total_events < 1.0 {
        return 0.0;
    }
    let stats = &ctx.stats.unigrams;
    let total_for_stats = stats.total() as f64;
    if total_for_stats < 1.0 {
        return 0.0;
    }
    let key_count = ctx.keyboard.keys.len() as f64;
    let mut score = 0.0f64;
    for (char_id, &count) in &stats.counts {
        let freq = count as f64 / total_for_stats;
        let kid = KeyId::new(char_id.as_usize());
        if let Some(key) = ctx.keyboard.find_by_id(kid.0) {
            let pos_x = key.x as f64;
            let pos_y = key.y as f64;
            score += freq * (pos_x + pos_y) as f64;
        }
    }
    score / key_count.max(1.0)
}

/// Simple wrapper that caches the total score.
pub struct ScoreCache<'a> {
    ctx: &'a StaticContext<'a>,
    total: f64,
}

impl<'a> ScoreCache<'a> {
    pub fn new(ctx: &'a StaticContext) -> Self {
        Self { ctx, total: 0.0 }
    }

    pub fn total(&self) -> f64 {
        self.total
    }

    pub fn recompute(&mut self, state: &LayoutState) {
        self.total = compute_score(self.ctx, state);
    }
}
