use crate::core::id::KeyId;
use crate::core::layout::LayoutState;
use crate::core::scoring::{ScoreCache, StaticContext};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

#[derive(Debug, Clone)]
pub struct OptimizerConfig {
    pub initial_temp: f64,
    pub cooling_rate: f64,
    pub max_steps: usize,
    pub seed: u64,
    pub locked_keys: Vec<KeyId>,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            initial_temp: 100.0,
            cooling_rate: 0.995,
            max_steps: 1_000_000,
            seed: 42,
            locked_keys: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct OptimizerProgress {
    pub step: usize,
    pub total_steps: usize,
    pub current_score: f64,
    pub best_score: f64,
}

pub fn optimize(
    ctx: &StaticContext,
    state: &mut LayoutState,
    config: &OptimizerConfig,
    cancel_flag: std::sync::Arc<AtomicBool>,
    progress_tx: mpsc::Sender<OptimizerProgress>,
) -> f64 {
    let mut rng = SmallRng::seed_from_u64(config.seed);
    let mut cache = ScoreCache::new(ctx);
    cache.recompute(state);
    let mut best_mapping = state.mapping.clone();
    let mut best_score = cache.total();
    let mut temperature = config.initial_temp;

    let locked_set: HashSet<KeyId> = config.locked_keys.iter().copied().collect();
    let mut unlocked_ids: Vec<KeyId> = state.mapping.keys().copied().collect();
    unlocked_ids.retain(|id| !locked_set.contains(id));
    unlocked_ids.sort_by_key(|k| k.0);

    for step in 0..config.max_steps {
        if cancel_flag.load(Ordering::Relaxed) {
            break;
        }
        if unlocked_ids.len() < 2 {
            break;
        }

        let idx_a = rng.gen_range(0..unlocked_ids.len());
        let mut idx_b = rng.gen_range(0..unlocked_ids.len());
        while idx_b == idx_a {
            idx_b = rng.gen_range(0..unlocked_ids.len());
        }
        let (id_a, id_b) = (unlocked_ids[idx_a], unlocked_ids[idx_b]);

        let old_total = cache.total();
        state.swap(id_a, id_b);
        cache.recompute(state);
        let new_total = cache.total();
        let delta = new_total - old_total;

        let accept = if delta < 0.0 {
            true
        } else if temperature < 1e-6 {
            false
        } else {
            let safe_delta = delta.abs().min(700.0);
            rng.gen::<f64>() < (-safe_delta / temperature).exp()
        };

        if accept {
            if new_total < best_score {
                best_score = new_total;
                best_mapping = state.mapping.clone();
            }
        } else {
            state.swap(id_a, id_b);
            cache.recompute(state);
        }

        temperature *= config.cooling_rate;

        if step % 5000 == 0 {
            let _ = progress_tx.send(OptimizerProgress {
                step,
                total_steps: config.max_steps,
                current_score: cache.total(),
                best_score,
            });
        }
    }

    state.mapping = best_mapping;
    best_score
}
