use serde::{Deserialize, Serialize};

use crate::types::*;

/// MLP architecture constants
pub const MLP_INPUT_SIZE: usize = 633;
pub const MLP_HIDDEN_SIZE: usize = 256;
pub const MLP_HIDDEN2_SIZE: usize = 64;

// Per-player feature counts
const NUM_SELL_CARD_TYPES: usize = 54;
const PER_PLAYER_FEATURES: usize = 259;

// ── Parameter indices ──

// MLP weights start at index 0 (no module params)
pub(crate) const MLP_W1: usize = 0;                                                        // [MLP_INPUT_SIZE * MLP_HIDDEN_SIZE = 156,928]
pub(crate) const MLP_B1: usize = MLP_W1 + MLP_INPUT_SIZE * MLP_HIDDEN_SIZE;                // 156,928, [MLP_HIDDEN_SIZE = 256]
pub(crate) const MLP_W2: usize = MLP_B1 + MLP_HIDDEN_SIZE;                                 // 157,184, [MLP_HIDDEN_SIZE * MLP_HIDDEN2_SIZE = 16,384]
pub(crate) const MLP_B2: usize = MLP_W2 + MLP_HIDDEN_SIZE * MLP_HIDDEN2_SIZE;              // 173,568, [MLP_HIDDEN2_SIZE = 64]
pub(crate) const MLP_W3: usize = MLP_B2 + MLP_HIDDEN2_SIZE;                                // 173,632, [MLP_HIDDEN2_SIZE = 64]
pub(crate) const MLP_B3: usize = MLP_W3 + MLP_HIDDEN2_SIZE;                                // 173,696, [1]

/// Number of differentiable parameters (indices 0..NUM_DIFF_PARAMS are updated by gradient descent).
pub const NUM_DIFF_PARAMS: usize = MLP_B3 + 1;                                             // 173,697

// Heuristic control parameters (non-differentiable)
pub(crate) const HEURISTIC_ROUND_THRESHOLD: usize = NUM_DIFF_PARAMS;                       // 173,697
pub(crate) const HEURISTIC_LOOKAHEAD: usize = HEURISTIC_ROUND_THRESHOLD + 1;               // 173,698
pub(crate) const PROGRESSIVE_BIAS_WEIGHT: usize = HEURISTIC_LOOKAHEAD + 1;                 // 173,699

/// Number of learnable parameters in the differentiable evaluation model.
pub const NUM_PARAMS: usize = PROGRESSIVE_BIAS_WEIGHT + 1;                                 // 173,700

/// LeakyReLU negative slope
const LEAKY_RELU_ALPHA: f64 = 0.01;
const LEAKY_RELU_ALPHA_F32: f32 = 0.01;

/// Deterministic pseudo-random number in [-1, 1] for reproducible weight initialization.
/// Uses SplitMix64-style hash for good distribution without needing an RNG crate.
fn deterministic_random(index: usize) -> f64 {
    let mut z = (index as u64).wrapping_add(0x9e3779b97f4a7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z = z ^ (z >> 31);
    (z as i64 as f64) / (i64::MAX as f64)
}

/// All learnable weights for the differentiable evaluation model.
/// Weights are heap-allocated (Box) to avoid stack overflows with the large parameter array.
#[derive(Debug, Clone)]
pub struct DiffEvalParams {
    pub weights: Box<[f64; NUM_PARAMS]>,
}

impl Serialize for DiffEvalParams {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.weights.as_slice().serialize(serializer)
    }
}

/// Allocate a zeroed parameter array on the heap.
fn boxed_zeros() -> Box<[f64; NUM_PARAMS]> {
    vec![0.0f64; NUM_PARAMS].into_boxed_slice().try_into()
        .unwrap_or_else(|_| unreachable!())
}

impl<'de> Deserialize<'de> for DiffEvalParams {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let vec = Vec::<f64>::deserialize(deserializer)?;
        if vec.len() > NUM_PARAMS {
            return Err(serde::de::Error::custom(
                format!("expected at most {} weights, got {}", NUM_PARAMS, vec.len())
            ));
        }
        // Clean break: pad with defaults
        let mut params = DiffEvalParams::default();
        params.weights[..vec.len()].copy_from_slice(&vec);
        Ok(params)
    }
}

impl Default for DiffEvalParams {
    fn default() -> Self {
        let mut w = boxed_zeros();

        // MLP: Random Xavier initialization
        // W1: MLP_INPUT_SIZE x MLP_HIDDEN_SIZE
        let w1_scale = (2.0f64 / (MLP_INPUT_SIZE as f64 + MLP_HIDDEN_SIZE as f64)).sqrt();
        for i in 0..(MLP_INPUT_SIZE * MLP_HIDDEN_SIZE) {
            w[MLP_W1 + i] = deterministic_random(MLP_W1 + i) * w1_scale;
        }
        // B1: zeros (already 0.0)

        // W2: MLP_HIDDEN_SIZE x MLP_HIDDEN2_SIZE
        let w2_scale = (2.0f64 / (MLP_HIDDEN_SIZE as f64 + MLP_HIDDEN2_SIZE as f64)).sqrt();
        for i in 0..(MLP_HIDDEN_SIZE * MLP_HIDDEN2_SIZE) {
            w[MLP_W2 + i] = deterministic_random(MLP_W2 + i) * w2_scale;
        }
        // B2: zeros (already 0.0)

        // W3: MLP_HIDDEN2_SIZE x 1
        let w3_scale = (2.0f64 / (MLP_HIDDEN2_SIZE as f64 + 1.0)).sqrt();
        for i in 0..MLP_HIDDEN2_SIZE {
            w[MLP_W3 + i] = deterministic_random(MLP_W3 + i) * w3_scale;
        }
        // B3: zero (already 0.0)

        // Control params
        w[HEURISTIC_ROUND_THRESHOLD] = 3.0;
        w[HEURISTIC_LOOKAHEAD] = 3.0;
        w[PROGRESSIVE_BIAS_WEIGHT] = 0.0;

        DiffEvalParams { weights: w }
    }
}

impl DiffEvalParams {
    /// Create a new DiffEvalParams with all weights set to zero (heap-allocated).
    pub fn zeros() -> Self {
        DiffEvalParams { weights: boxed_zeros() }
    }

    pub fn heuristic_round_threshold(&self) -> u32 {
        self.weights[HEURISTIC_ROUND_THRESHOLD].max(0.0) as u32
    }

    pub fn heuristic_lookahead(&self) -> u32 {
        self.weights[HEURISTIC_LOOKAHEAD].max(1.0) as u32
    }

    pub fn progressive_bias_weight(&self) -> f64 {
        self.weights[PROGRESSIVE_BIAS_WEIGHT]
    }

    pub fn set_progressive_bias_weight(&mut self, value: f64) {
        self.weights[PROGRESSIVE_BIAS_WEIGHT] = value;
    }

    pub fn set_heuristic_round_threshold(&mut self, value: u32) {
        self.weights[HEURISTIC_ROUND_THRESHOLD] = value as f64;
    }

    pub fn set_heuristic_lookahead(&mut self, value: u32) {
        self.weights[HEURISTIC_LOOKAHEAD] = value as f64;
    }
}

// ── Card type table ──

#[allow(dead_code)]
const ALL_CARDS: [Card; 48] = [
    Card::BasicRed, Card::BasicYellow, Card::BasicBlue,
    Card::Kermes, Card::Weld, Card::Woad,
    Card::Lac, Card::Brazilwood, Card::Pomegranate,
    Card::Sumac, Card::Elderberry, Card::Turnsole,
    Card::Madder, Card::Turmeric, Card::DyersGreenweed,
    Card::Verdigris, Card::Orchil, Card::Logwood,
    Card::VermilionDye, Card::Saffron, Card::PersianBerries,
    Card::Azurite, Card::IndigoDye, Card::Cochineal,
    Card::StarterCeramics, Card::StarterPaintings, Card::StarterTextiles,
    Card::TerraCotta, Card::OchreWare, Card::CobaltWare,
    Card::CinnabarCanvas, Card::OrpimentCanvas, Card::UltramarineCanvas,
    Card::AlizarinFabric, Card::FusticFabric, Card::PastelFabric,
    Card::ClayCanvas, Card::ClayFabric, Card::CanvasFabric,
    Card::Alum, Card::CreamOfTartar, Card::GumArabic,
    Card::Potash, Card::Vinegar, Card::Argol, Card::Chalk,
    Card::LinseedOil, Card::Lye,
];

/// Precomputed f32 MLP weights for fast inference.
pub struct DiffEvalTable {
    w1_f32: Box<[f32; MLP_INPUT_SIZE * MLP_HIDDEN_SIZE]>,
    b1_f32: [f32; MLP_HIDDEN_SIZE],
    w2_f32: Box<[f32; MLP_HIDDEN_SIZE * MLP_HIDDEN2_SIZE]>,
    b2_f32: [f32; MLP_HIDDEN2_SIZE],
    w3_f32: [f32; MLP_HIDDEN2_SIZE],
    b3_f32: f32,
}

impl DiffEvalTable {
    pub fn new(params: &DiffEvalParams) -> Self {
        let w = &params.weights;

        // Precompute f32 MLP weights for fast inference
        let mut w1_f32: Box<[f32; MLP_INPUT_SIZE * MLP_HIDDEN_SIZE]> =
            vec![0.0f32; MLP_INPUT_SIZE * MLP_HIDDEN_SIZE].into_boxed_slice().try_into()
                .unwrap_or_else(|_| unreachable!());
        for i in 0..(MLP_INPUT_SIZE * MLP_HIDDEN_SIZE) {
            w1_f32[i] = w[MLP_W1 + i] as f32;
        }
        let mut b1_f32 = [0.0f32; MLP_HIDDEN_SIZE];
        for i in 0..MLP_HIDDEN_SIZE {
            b1_f32[i] = w[MLP_B1 + i] as f32;
        }
        let mut w2_f32: Box<[f32; MLP_HIDDEN_SIZE * MLP_HIDDEN2_SIZE]> =
            vec![0.0f32; MLP_HIDDEN_SIZE * MLP_HIDDEN2_SIZE].into_boxed_slice().try_into()
                .unwrap_or_else(|_| unreachable!());
        for i in 0..(MLP_HIDDEN_SIZE * MLP_HIDDEN2_SIZE) {
            w2_f32[i] = w[MLP_W2 + i] as f32;
        }
        let mut b2_f32 = [0.0f32; MLP_HIDDEN2_SIZE];
        for i in 0..MLP_HIDDEN2_SIZE {
            b2_f32[i] = w[MLP_B2 + i] as f32;
        }
        let mut w3_f32 = [0.0f32; MLP_HIDDEN2_SIZE];
        for i in 0..MLP_HIDDEN2_SIZE {
            w3_f32[i] = w[MLP_W3 + i] as f32;
        }
        let b3_f32 = w[MLP_B3] as f32;

        DiffEvalTable { w1_f32, b1_f32, w2_f32, b2_f32, w3_f32, b3_f32 }
    }
}

// ── Feature computation ──

/// Compute all MLP input features for the full 2-player game state from `perspective_player`'s point of view.
///
/// Input layout (MLP_INPUT_SIZE = 613):
///   Per player (249 features x 2 players = 498):
///     Player 0 = perspective player, Player 1 = opponent (next in turn order).
///     For each player at offset `player_idx * 249`:
///       [0..12]    Color wheel: 12 counts
///       [12..15]   Materials: 3 counts (textiles, ceramics, paintings)
///       [15]       Ducats: 1
///       [16]       Completed sell card count: 1
///       [17]       Completed sell card ducats: 1
///       [18]       Has acted this round: 1 (0 or 1)
///       [19..65]   Cards in deck: 46 counts
///       [65..111]  Cards in discard: 46 counts
///       [111..157] Cards in drafted: 46 counts
///       [157..203] Cards in workshop: 46 counts
///       [203..249] Cards in workshopped: 46 counts
///
///   Shared state (115 features) at offset 498:
///     [498..552]  Sell card display: 54 binary flags
///     [552..606]  Sell card completed (by any player): 54 binary flags
///     [606]       Sell cards remaining in deck: 1 count
///     [607]       Round: 1
///     [608]       Is draft phase: 1 binary
///     [609]       Is action phase: 1 binary
///     [610]       Draft deck size: 1 count
///     [611]       Destroyed pile size: 1 count
///     [612]       First player offset: 1
pub(crate) fn compute_features(
    state: &GameState,
    perspective_player: usize,
) -> [f32; MLP_INPUT_SIZE] {
    let mut features = [0.0f32; MLP_INPUT_SIZE];
    let num_players = state.players.len();

    // Determine player ordering: player 0 = perspective, player 1 = opponent
    let player_indices = [
        perspective_player,
        (perspective_player + 1) % num_players,
    ];

    // Determine who has acted this round
    let first_player = if state.round > 0 {
        ((state.round - 1) as usize) % num_players
    } else {
        0
    };

    let is_action_phase = matches!(state.phase, GamePhase::Action { .. });
    let is_draft_phase = matches!(state.phase, GamePhase::Draft { .. });

    let current_action_player = match &state.phase {
        GamePhase::Action { action_state } => Some(action_state.current_player_index),
        _ => None,
    };

    // Per-player features
    for (slot, &pi) in player_indices.iter().enumerate() {
        let player = &state.players[pi];
        let base = slot * PER_PLAYER_FEATURES;

        // [0..12] Color wheel counts
        for c in 0..NUM_COLORS {
            features[base + c] = player.color_wheel.counts[c] as f32;
        }

        // [12..15] Materials
        for i in 0..3 {
            features[base + 12 + i] = player.materials.counts[i] as f32;
        }

        // [15] Ducats
        features[base + 15] = player.ducats as f32;

        // [16] Completed sell card count
        features[base + 16] = player.completed_sell_cards.len() as f32;

        // [17] Completed sell card ducats
        let completed_ducats: u32 = player.completed_sell_cards.iter()
            .map(|sc| sc.sell_card.ducats())
            .sum();
        features[base + 17] = completed_ducats as f32;

        // [18] Has acted this round
        // During action phase, players between first_player and current_player_index have acted.
        // During draft phase, no one has "acted" in the action sense yet.
        let has_acted = if is_action_phase {
            if let Some(current_idx) = current_action_player {
                // Players from first_player up to (but not including) current_idx have acted
                has_player_acted(pi, first_player, current_idx, num_players)
            } else {
                false
            }
        } else {
            false
        };
        features[base + 18] = if has_acted { 1.0 } else { 0.0 };

        // Card counts per zone, using card_lookup to map instance IDs to card types
        // [19..67] deck
        count_cards_into(&player.deck, &state.card_lookup, &mut features, base + 19);
        // [67..115] discard
        count_cards_into(&player.discard, &state.card_lookup, &mut features, base + 67);
        // [115..163] drafted
        count_cards_into(&player.drafted_cards, &state.card_lookup, &mut features, base + 115);
        // [163..211] workshop
        count_cards_into(&player.workshop_cards, &state.card_lookup, &mut features, base + 163);
        // [211..259] workshopped
        count_cards_into(&player.workshopped_cards, &state.card_lookup, &mut features, base + 211);
    }

    // Shared state at offset 518
    let shared_base = 518;

    // [498..552] Sell card display: 54 binary flags
    for sci in state.sell_card_display.iter() {
        let idx = sci.sell_card as usize;
        features[shared_base + idx] = 1.0;
    }

    // [552..606] Sell card completed (by any player): 54 binary flags
    for &pi in &player_indices {
        for sci in state.players[pi].completed_sell_cards.iter() {
            let idx = sci.sell_card as usize;
            features[shared_base + NUM_SELL_CARD_TYPES + idx] = 1.0;
        }
    }

    // [626] Sell cards remaining in deck
    features[shared_base + 2 * NUM_SELL_CARD_TYPES] = state.sell_card_deck.len() as f32;

    // [627] Round
    features[shared_base + 2 * NUM_SELL_CARD_TYPES + 1] = state.round as f32;

    // [628] Is draft phase
    features[shared_base + 2 * NUM_SELL_CARD_TYPES + 2] = if is_draft_phase { 1.0 } else { 0.0 };

    // [629] Is action phase
    features[shared_base + 2 * NUM_SELL_CARD_TYPES + 3] = if is_action_phase { 1.0 } else { 0.0 };

    // [630] Draft deck size
    features[shared_base + 2 * NUM_SELL_CARD_TYPES + 4] = state.draft_deck.len() as f32;

    // [631] Destroyed pile size
    features[shared_base + 2 * NUM_SELL_CARD_TYPES + 5] = state.destroyed_pile.len() as f32;

    // [632] First player offset: 0 = perspective player goes first, 1 = opponent goes first
    let first_player_offset = if first_player == perspective_player { 0.0 } else { 1.0 };
    features[shared_base + 2 * NUM_SELL_CARD_TYPES + 6] = first_player_offset;

    features
}

/// Helper: determine if player `pi` has acted this round during action phase.
/// Players act in order starting from `first_player`. A player has acted if
/// they come before `current_player` in the turn order starting from `first_player`.
#[inline]
fn has_player_acted(pi: usize, first_player: usize, current_player: usize, num_players: usize) -> bool {
    if pi == current_player {
        return false; // Currently acting, not yet done
    }
    // Position in turn order relative to first_player
    let pi_pos = (pi + num_players - first_player) % num_players;
    let current_pos = (current_player + num_players - first_player) % num_players;
    pi_pos < current_pos
}

/// Count card types in an UnorderedCards set and write into features at the given offset.
#[inline]
fn count_cards_into(
    cards: &crate::unordered_cards::UnorderedCards,
    card_lookup: &[Card; 256],
    features: &mut [f32; MLP_INPUT_SIZE],
    offset: usize,
) {
    for id in cards.iter() {
        let card = card_lookup[id as usize];
        let card_idx = card as usize;
        features[offset + card_idx] += 1.0;
    }
}

// ── Forward pass ──

/// Aggregation MLP: MLP_INPUT_SIZE -> MLP_HIDDEN_SIZE (LeakyReLU) -> MLP_HIDDEN2_SIZE (LeakyReLU) -> 1
/// Used by diff_eval_score for f64 compatibility with gradient tests.
fn aggregation_mlp(inputs: &[f64; MLP_INPUT_SIZE], w: &[f64; NUM_PARAMS]) -> f64 {
    // Hidden layer 1: MLP_INPUT_SIZE -> MLP_HIDDEN_SIZE
    let mut hidden1 = [0.0f64; MLP_HIDDEN_SIZE];
    for row in 0..MLP_HIDDEN_SIZE {
        let mut sum = w[MLP_B1 + row];
        sum += super::simd_ops::dot_f64(&w[MLP_W1 + row * MLP_INPUT_SIZE..MLP_W1 + (row + 1) * MLP_INPUT_SIZE], inputs);
        hidden1[row] = if sum > 0.0 { sum } else { LEAKY_RELU_ALPHA * sum };
    }

    // Hidden layer 2: MLP_HIDDEN_SIZE -> MLP_HIDDEN2_SIZE
    let mut hidden2 = [0.0f64; MLP_HIDDEN2_SIZE];
    for row in 0..MLP_HIDDEN2_SIZE {
        let mut sum = w[MLP_B2 + row];
        sum += super::simd_ops::dot_f64(&w[MLP_W2 + row * MLP_HIDDEN_SIZE..MLP_W2 + (row + 1) * MLP_HIDDEN_SIZE], &hidden1);
        hidden2[row] = if sum > 0.0 { sum } else { LEAKY_RELU_ALPHA * sum };
    }

    // Output layer: MLP_HIDDEN2_SIZE -> 1
    let mut output = w[MLP_B3];
    output += super::simd_ops::dot_f64(&w[MLP_W3..MLP_W3 + MLP_HIDDEN2_SIZE], &hidden2);

    output
}

/// f32 MLP forward pass using precomputed table weights.
fn mlp_f32(
    input: &[f32; MLP_INPUT_SIZE],
    table: &DiffEvalTable,
) -> f32 {
    // Hidden layer 1: MLP_INPUT_SIZE -> MLP_HIDDEN_SIZE
    let mut hidden1 = [0.0f32; MLP_HIDDEN_SIZE];
    for row in 0..MLP_HIDDEN_SIZE {
        let mut sum = table.b1_f32[row];
        let w1_row = &table.w1_f32[row * MLP_INPUT_SIZE..(row + 1) * MLP_INPUT_SIZE];
        sum += super::simd_ops::dot_f32(w1_row, input);
        hidden1[row] = if sum > 0.0 { sum } else { LEAKY_RELU_ALPHA_F32 * sum };
    }

    // Hidden layer 2: MLP_HIDDEN_SIZE -> MLP_HIDDEN2_SIZE
    let mut hidden2 = [0.0f32; MLP_HIDDEN2_SIZE];
    for row in 0..MLP_HIDDEN2_SIZE {
        let mut sum = table.b2_f32[row];
        let w2_row = &table.w2_f32[row * MLP_HIDDEN_SIZE..(row + 1) * MLP_HIDDEN_SIZE];
        sum += super::simd_ops::dot_f32(w2_row, &hidden1);
        hidden2[row] = if sum > 0.0 { sum } else { LEAKY_RELU_ALPHA_F32 * sum };
    }

    // Output layer: MLP_HIDDEN2_SIZE -> 1
    let mut output = table.b3_f32;
    output += super::simd_ops::dot_f32(&table.w3_f32, &hidden2);
    output
}

/// Full forward pass: compute evaluation logit for a single perspective.
/// Uses f64 MLP for compatibility with gradient finite-difference tests.
pub fn diff_eval_score(
    state: &GameState,
    perspective_player: usize,
    params: &DiffEvalParams,
    _table: &DiffEvalTable,
) -> f64 {
    let features_f32 = compute_features(state, perspective_player);
    let mut inputs = [0.0f64; MLP_INPUT_SIZE];
    for i in 0..MLP_INPUT_SIZE {
        inputs[i] = features_f32[i] as f64;
    }
    aggregation_mlp(&inputs, &params.weights)
}

/// Compute per-player rewards using softmax over diff eval logits.
/// Each player gets P(win) = softmax(logit_i).
///
/// Evaluates from each player's perspective to get their logits, then softmaxes.
pub fn compute_diff_eval_rewards(
    state: &GameState,
    _perspective_player: usize,
    _params: &DiffEvalParams,
    table: &DiffEvalTable,
) -> [f64; MAX_PLAYERS] {
    let n = state.players.len();

    // Compute logit from each player's perspective using f32 path
    let mut logits = [0.0f64; MAX_PLAYERS];
    for i in 0..n {
        let features = compute_features(state, i);
        logits[i] = mlp_f32(&features, table) as f64;
    }

    // Softmax in f64 for numerical stability
    let max_logit = logits[..n].iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut exp_sum = 0.0;
    let mut result = [0.0; MAX_PLAYERS];
    for i in 0..n {
        result[i] = (logits[i] - max_logit).exp();
        exp_sum += result[i];
    }
    for i in 0..n {
        result[i] /= exp_sum;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::create_initial_game_state;
    use crate::draw_phase::execute_draw_phase;
    use rand::SeedableRng;
    use wyrand::WyRand;

    fn make_test_state() -> GameState {
        let mut rng = WyRand::seed_from_u64(42);
        let mut state = create_initial_game_state(2, &[true, true], &mut rng);
        execute_draw_phase(&mut state, &mut rng);
        state
    }

    #[test]
    fn test_default_params_forward_pass() {
        let params = DiffEvalParams::default();
        let table = DiffEvalTable::new(&params);
        let state = make_test_state();

        let score = diff_eval_score(&state, 0, &params, &table);

        // Score should be finite
        assert!(score.is_finite(), "Score should be finite, got {}", score);
    }

    #[test]
    fn test_different_states_get_different_scores() {
        let params = DiffEvalParams::default();
        let table = DiffEvalTable::new(&params);

        let state1 = make_test_state();
        let score1 = diff_eval_score(&state1, 0, &params, &table);

        // Create a different state with a different seed
        let mut rng = WyRand::seed_from_u64(123);
        let mut state2 = create_initial_game_state(2, &[true, true], &mut rng);
        execute_draw_phase(&mut state2, &mut rng);
        let score2 = diff_eval_score(&state2, 0, &params, &table);

        assert!(score1.is_finite());
        assert!(score2.is_finite());
        assert!((score1 - score2).abs() > 1e-10,
            "Different game states should produce different scores: {} vs {}", score1, score2);
    }

    #[test]
    fn test_softmax_rewards_sum_to_one() {
        let params = DiffEvalParams::default();
        let table = DiffEvalTable::new(&params);
        let state = make_test_state();

        let rewards = compute_diff_eval_rewards(&state, 0, &params, &table);
        let sum: f64 = rewards[..2].iter().sum();
        assert!((sum - 1.0).abs() < 1e-10, "Rewards should sum to 1.0, got {}", sum);

        // Both rewards should be positive (valid probabilities)
        assert!(rewards[0] > 0.0, "Player 0 reward should be positive");
        assert!(rewards[1] > 0.0, "Player 1 reward should be positive");
    }
}
