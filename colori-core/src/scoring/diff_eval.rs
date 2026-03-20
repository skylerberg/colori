use serde::{Deserialize, Serialize};

use crate::colors::{PRIMARIES, SECONDARIES, TERTIARIES, VALID_MIX_PAIRS};
use crate::fixed_vec::FixedVec;
use crate::types::*;

/// MLP architecture constants
pub const MLP_INPUT_SIZE: usize = 117;
pub const MLP_HIDDEN_SIZE: usize = 256;
pub const MLP_HIDDEN2_SIZE: usize = 64;

/// Number of learnable parameters in the differentiable evaluation model.
pub const NUM_PARAMS: usize = 46796;

// ── Parameter indices ──

// Module 1: Color Wheel Value (24 params)
pub(crate) const COLOR_SAT_W: usize = 0;       // [3] primary, secondary, tertiary
pub(crate) const COLOR_SAT_A: usize = 3;       // [3]
pub(crate) const MIX_PAIR_W: usize = 6;        // [9] one per VALID_MIX_PAIRS entry
pub(crate) const COVERAGE_W: usize = 15;       // [3] per tier
pub(crate) const COVERAGE_A: usize = 18;       // [3]
pub(crate) const COVERAGE_B: usize = 21;       // [3]

// Module 2: Sell Card Alignment (20 params)
pub(crate) const SELL_MAT_W: usize = 24;       // [3] per material type
pub(crate) const SELL_DUCAT_W: usize = 27;     // [3] for 2/3/4-ducat tiers
pub(crate) const SELL_COMBINE_W: usize = 30;   // [2] w_combine, w_color
pub(crate) const SELL_AGG_W: usize = 32;       // [3] best, second, sum_rest
pub(crate) const SELL_ROUND_W: usize = 35;     // [2] w_round, b_round
pub(crate) const SELL_SOLD_W: usize = 37;      // [3] w_sold, a_sold, b_sold
// Total module 2: 17 params (I'll adjust)

// Module 3: Deck Color Profile (22 params)
pub(crate) const DECK_COLOR_SAT_W: usize = 40; // [3] per tier
pub(crate) const DECK_COLOR_SAT_A: usize = 43; // [3]
pub(crate) const DECK_PROD_NEED_W: usize = 46; // [3] per ducat tier
pub(crate) const DECK_ACTION_W: usize = 49;    // [5] Alum, CreamOfTartar, GumArabic, Potash, Chalk
pub(crate) const DECK_MAT_CARD_W: usize = 54;  // [3] starter, colored, dual
pub(crate) const DECK_SIZE_W: usize = 57;      // [2] linear, quadratic
pub(crate) const DECK_DIVERSITY_W: usize = 59; // [1]
pub(crate) const DECK_WORKSHOP_W: usize = 60;  // [1]

// Module 4: Material Strategy (11 params)
pub(crate) const MAT_SUFF_W: usize = 61;       // [3] per type
pub(crate) const MAT_SUFF_THRESH: usize = 64;  // [3]
pub(crate) const MAT_DEMAND_W: usize = 67;     // [3]
pub(crate) const MAT_DIVERSITY_W: usize = 70;  // [2]

// Aggregation MLP: MLP_INPUT_SIZE → MLP_HIDDEN_SIZE → MLP_HIDDEN2_SIZE → 1
pub(crate) const MLP_W1: usize = 72;                                                       // [MLP_INPUT_SIZE * MLP_HIDDEN_SIZE = 29952]
pub(crate) const MLP_B1: usize = MLP_W1 + MLP_INPUT_SIZE * MLP_HIDDEN_SIZE;                // 30024, [MLP_HIDDEN_SIZE = 256]
pub(crate) const MLP_W2: usize = MLP_B1 + MLP_HIDDEN_SIZE;                                 // 30280, [MLP_HIDDEN_SIZE * MLP_HIDDEN2_SIZE = 16384]
pub(crate) const MLP_B2: usize = MLP_W2 + MLP_HIDDEN_SIZE * MLP_HIDDEN2_SIZE;              // 46664, [MLP_HIDDEN2_SIZE = 64]
pub(crate) const MLP_W3: usize = MLP_B2 + MLP_HIDDEN2_SIZE;                                // 46728, [MLP_HIDDEN2_SIZE = 64]
pub(crate) const MLP_B3: usize = MLP_W3 + MLP_HIDDEN2_SIZE;                                // 46792, [1]

/// Number of differentiable parameters (indices 0..NUM_DIFF_PARAMS are updated by gradient descent).
pub const NUM_DIFF_PARAMS: usize = MLP_B3 + 1;                                             // 46793

// Heuristic control parameters (non-differentiable)
pub(crate) const HEURISTIC_ROUND_THRESHOLD: usize = NUM_DIFF_PARAMS;                       // 46793
pub(crate) const HEURISTIC_LOOKAHEAD: usize = HEURISTIC_ROUND_THRESHOLD + 1;               // 46794

pub(crate) const PROGRESSIVE_BIAS_WEIGHT: usize = HEURISTIC_LOOKAHEAD + 1;                 // 46795

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

/// Old architecture parameter count (before the deeper MLP change).
/// Used to detect and migrate legacy checkpoints.
const LEGACY_NUM_PARAMS: usize = 30540;
const LEGACY_MLP_B2: usize = 30536;
const LEGACY_HEURISTIC_ROUND_THRESHOLD: usize = 30537;
const LEGACY_HEURISTIC_LOOKAHEAD: usize = 30538;
const LEGACY_PROGRESSIVE_BIAS_WEIGHT: usize = 30539;

impl<'de> Deserialize<'de> for DiffEvalParams {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let vec = Vec::<f64>::deserialize(deserializer)?;
        if vec.len() > NUM_PARAMS {
            return Err(serde::de::Error::custom(
                format!("expected at most {} weights, got {}", NUM_PARAMS, vec.len())
            ));
        }
        // Pad with defaults for backwards compatibility with older checkpoints
        let mut params = DiffEvalParams::default();
        params.weights[..vec.len()].copy_from_slice(&vec);

        // Legacy checkpoint migration: old architecture had 30540 params with a single
        // hidden layer (256->1). The naive copy puts old W2 (256 values at 30280..30536)
        // into new W2 row 0 (same indices), which is correct. But old B2 and control
        // params (30536-30539) land in the middle of new W2 and need to be relocated.
        if vec.len() <= LEGACY_NUM_PARAMS && vec.len() > LEGACY_MLP_B2 {
            // Reset misplaced values in new W2 to default random Xavier init
            let defaults = DiffEvalParams::default();
            for i in LEGACY_MLP_B2..vec.len().min(LEGACY_NUM_PARAMS) {
                params.weights[i] = defaults.weights[i];
            }
            // Place old B2 into new B2[0]
            params.weights[MLP_B2] = vec[LEGACY_MLP_B2];
            // Set W3[0] = 1.0 so first hidden2 neuron passes through old output
            params.weights[MLP_W3] = 1.0;
            // Migrate control params to new positions
            if vec.len() > LEGACY_HEURISTIC_ROUND_THRESHOLD {
                params.weights[HEURISTIC_ROUND_THRESHOLD] = vec[LEGACY_HEURISTIC_ROUND_THRESHOLD];
            }
            if vec.len() > LEGACY_HEURISTIC_LOOKAHEAD {
                params.weights[HEURISTIC_LOOKAHEAD] = vec[LEGACY_HEURISTIC_LOOKAHEAD];
            }
            if vec.len() > LEGACY_PROGRESSIVE_BIAS_WEIGHT {
                params.weights[PROGRESSIVE_BIAS_WEIGHT] = vec[LEGACY_PROGRESSIVE_BIAS_WEIGHT];
            }
        }

        Ok(params)
    }
}

impl Default for DiffEvalParams {
    fn default() -> Self {
        let mut w = boxed_zeros();

        // Module 1: Color wheel - initial values inspired by linear heuristic
        // Color saturation weights (w) and rates (a)
        w[COLOR_SAT_W] = 0.10;     // primary w
        w[COLOR_SAT_W + 1] = 0.20; // secondary w
        w[COLOR_SAT_W + 2] = 0.30; // tertiary w
        w[COLOR_SAT_A] = 1.0;      // primary a
        w[COLOR_SAT_A + 1] = 1.0;  // secondary a
        w[COLOR_SAT_A + 2] = 1.0;  // tertiary a
        // Mix pair weights: small initial values
        for i in 0..9 {
            w[MIX_PAIR_W + i] = 0.05;
        }
        // Coverage: disabled initially
        // w[COVERAGE_W..COVERAGE_B+3] stays 0

        // Module 2: Sell card alignment
        w[SELL_MAT_W] = 1.0;
        w[SELL_MAT_W + 1] = 1.0;
        w[SELL_MAT_W + 2] = 1.0;
        w[SELL_DUCAT_W] = 0.5;     // 2-ducat
        w[SELL_DUCAT_W + 1] = 0.5; // 3-ducat
        w[SELL_DUCAT_W + 2] = 0.5; // 4-ducat
        w[SELL_COMBINE_W] = 0.5;   // w_combine (material)
        w[SELL_COMBINE_W + 1] = 0.5; // w_color
        w[SELL_AGG_W] = 1.0;       // best
        w[SELL_AGG_W + 1] = 0.3;   // second
        w[SELL_AGG_W + 2] = 0.0;   // sum_rest
        w[SELL_ROUND_W] = 0.5;     // w_round
        w[SELL_ROUND_W + 1] = 5.0; // b_round
        w[SELL_SOLD_W] = 0.5;      // w_sold
        w[SELL_SOLD_W + 1] = 0.5;  // a_sold
        w[SELL_SOLD_W + 2] = 3.0;  // b_sold

        // Module 3: Deck color profile
        w[DECK_COLOR_SAT_W] = 0.10;
        w[DECK_COLOR_SAT_W + 1] = 0.15;
        w[DECK_COLOR_SAT_W + 2] = 0.20;
        w[DECK_COLOR_SAT_A] = 1.0;
        w[DECK_COLOR_SAT_A + 1] = 1.0;
        w[DECK_COLOR_SAT_A + 2] = 1.0;
        w[DECK_PROD_NEED_W] = 0.3;
        w[DECK_PROD_NEED_W + 1] = 0.3;
        w[DECK_PROD_NEED_W + 2] = 0.3;
        // Action card values
        w[DECK_ACTION_W] = 1.0;     // Alum
        w[DECK_ACTION_W + 1] = 1.0; // CreamOfTartar
        w[DECK_ACTION_W + 2] = 1.0; // GumArabic
        w[DECK_ACTION_W + 3] = 1.0; // Potash
        w[DECK_ACTION_W + 4] = 0.2; // Chalk
        // Material card subcategory values
        w[DECK_MAT_CARD_W] = 0.2;     // starter
        w[DECK_MAT_CARD_W + 1] = 0.5; // colored material
        w[DECK_MAT_CARD_W + 2] = 0.6; // dual material
        // Deck size
        w[DECK_SIZE_W] = 0.0;
        w[DECK_SIZE_W + 1] = 0.0;
        // Diversity
        w[DECK_DIVERSITY_W] = 0.1;
        // Workshopped
        w[DECK_WORKSHOP_W] = 0.1;

        // Module 4: Material strategy
        w[MAT_SUFF_W] = 2.0;
        w[MAT_SUFF_W + 1] = 2.0;
        w[MAT_SUFF_W + 2] = 2.0;
        w[MAT_SUFF_THRESH] = 1.0;
        w[MAT_SUFF_THRESH + 1] = 1.0;
        w[MAT_SUFF_THRESH + 2] = 1.0;
        w[MAT_DEMAND_W] = 0.1;
        w[MAT_DEMAND_W + 1] = 0.1;
        w[MAT_DEMAND_W + 2] = 0.1;
        w[MAT_DIVERSITY_W] = 0.1;
        w[MAT_DIVERSITY_W + 1] = 0.05;

        // Aggregation MLP: Random Xavier initialization
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

// ── Precomputed per-card tables ──

/// For each card type (indexed by Card enum value), precompute:
/// - color_production: [u8; 12] — how many pips of each color this card produces
/// - color_tier: which tier(s) this card's colors belong to (for grouping)
/// - card_category: enum for action/material subcategory lookup
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckCardCategory {
    Dye,            // Any dye card (has colors)
    ActionAlum,
    ActionCreamOfTartar,
    ActionGumArabic,
    ActionPotash,
    ActionChalk,
    ActionOther,    // Vinegar, Argol (not in draft deck but could appear)
    MaterialStarter,
    MaterialColored,
    MaterialDual,
}

pub struct DiffEvalTable {
    /// For each card type (by Card enum index): per-color production count
    pub color_production: [[u8; NUM_COLORS]; 46],
    /// For each card type: category for deck quality computation
    pub category: [DeckCardCategory; 46],
    /// Precomputed f32 MLP weights for fast batched inference
    w1_f32: Box<[f32; MLP_INPUT_SIZE * MLP_HIDDEN_SIZE]>,
    b1_f32: [f32; MLP_HIDDEN_SIZE],
    w2_f32: Box<[f32; MLP_HIDDEN_SIZE * MLP_HIDDEN2_SIZE]>,
    b2_f32: [f32; MLP_HIDDEN2_SIZE],
    w3_f32: [f32; MLP_HIDDEN2_SIZE],
    b3_f32: f32,
}

const ALL_CARDS: [Card; 46] = [
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
];

impl DiffEvalTable {
    pub fn new(params: &DiffEvalParams) -> Self {
        let mut color_production = [[0u8; NUM_COLORS]; 46];
        let mut category = [DeckCardCategory::Dye; 46];

        for &card in &ALL_CARDS {
            let idx = card as usize;

            // Color production
            for &color in card.colors() {
                color_production[idx][color.index()] += 1;
            }

            // Category
            category[idx] = match card.kind() {
                CardKind::BasicDye | CardKind::Dye => DeckCardCategory::Dye,
                CardKind::Action => match card {
                    Card::Alum => DeckCardCategory::ActionAlum,
                    Card::CreamOfTartar => DeckCardCategory::ActionCreamOfTartar,
                    Card::GumArabic => DeckCardCategory::ActionGumArabic,
                    Card::Potash => DeckCardCategory::ActionPotash,
                    Card::Chalk => DeckCardCategory::ActionChalk,
                    _ => DeckCardCategory::ActionOther,
                },
                CardKind::Material => {
                    let colors = card.colors();
                    let mat_types = card.material_types();
                    if mat_types.len() >= 2 {
                        DeckCardCategory::MaterialDual
                    } else if !colors.is_empty() {
                        DeckCardCategory::MaterialColored
                    } else {
                        DeckCardCategory::MaterialStarter
                    }
                },
            };
        }

        // Precompute f32 MLP weights for fast inference
        let w = &params.weights;
        let mut w1_f32 = Box::new([0.0f32; MLP_INPUT_SIZE * MLP_HIDDEN_SIZE]);
        for i in 0..(MLP_INPUT_SIZE * MLP_HIDDEN_SIZE) {
            w1_f32[i] = w[MLP_W1 + i] as f32;
        }
        let mut b1_f32 = [0.0f32; MLP_HIDDEN_SIZE];
        for i in 0..MLP_HIDDEN_SIZE {
            b1_f32[i] = w[MLP_B1 + i] as f32;
        }
        let mut w2_f32 = Box::new([0.0f32; MLP_HIDDEN_SIZE * MLP_HIDDEN2_SIZE]);
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

        DiffEvalTable { color_production, category, w1_f32, b1_f32, w2_f32, b2_f32, w3_f32, b3_f32 }
    }
}

// ── Forward pass ──

#[inline]
fn sigmoid_f32(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

#[inline]
fn color_tier(color: Color) -> usize {
    let idx = color.index();
    if idx % 4 == 0 { 0 }       // primary: 0, 4, 8
    else if idx % 2 == 0 { 1 }  // secondary: 2, 6, 10
    else { 2 }                   // tertiary: 1, 3, 5, 7, 9, 11
}

/// Aggregation MLP: MLP_INPUT_SIZE → MLP_HIDDEN_SIZE (LeakyReLU) → MLP_HIDDEN2_SIZE (LeakyReLU) → 1
/// Used by diff_eval_score for f64 compatibility with gradient tests.
fn aggregation_mlp(inputs: &[f64; MLP_INPUT_SIZE], w: &[f64; NUM_PARAMS]) -> f64 {
    // Hidden layer 1: MLP_INPUT_SIZE → MLP_HIDDEN_SIZE
    let mut hidden1 = [0.0f64; MLP_HIDDEN_SIZE];
    for row in 0..MLP_HIDDEN_SIZE {
        let mut sum = w[MLP_B1 + row];
        for col in 0..MLP_INPUT_SIZE {
            sum += w[MLP_W1 + row * MLP_INPUT_SIZE + col] * inputs[col];
        }
        hidden1[row] = if sum > 0.0 { sum } else { LEAKY_RELU_ALPHA * sum };
    }

    // Hidden layer 2: MLP_HIDDEN_SIZE → MLP_HIDDEN2_SIZE
    let mut hidden2 = [0.0f64; MLP_HIDDEN2_SIZE];
    for row in 0..MLP_HIDDEN2_SIZE {
        let mut sum = w[MLP_B2 + row];
        for col in 0..MLP_HIDDEN_SIZE {
            sum += w[MLP_W2 + row * MLP_HIDDEN_SIZE + col] * hidden1[col];
        }
        hidden2[row] = if sum > 0.0 { sum } else { LEAKY_RELU_ALPHA * sum };
    }

    // Output layer: MLP_HIDDEN2_SIZE → 1
    let mut output = w[MLP_B3];
    for i in 0..MLP_HIDDEN2_SIZE {
        output += w[MLP_W3 + i] * hidden2[i];
    }

    output
}

/// Precomputed sell card data shared across all player evaluations.
struct SellCardCache {
    sell_demand: [u32; NUM_COLORS],
    mat_demand: [f32; 3],
}

impl SellCardCache {
    fn new(sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>) -> Self {
        let mut sell_demand = [0u32; NUM_COLORS];
        let mut mat_demand = [0.0f32; 3];
        for bi in sell_card_display.iter() {
            let cost = bi.sell_card.color_cost();
            for &c in cost {
                sell_demand[c.index()] += 1;
            }
            let mat_idx = bi.sell_card.required_material() as usize;
            mat_demand[mat_idx] += bi.sell_card.ducats() as f32;
        }
        SellCardCache { sell_demand, mat_demand }
    }
}

/// Compute all MLP input features for a single player in f32.
/// Merges card iteration and uses precomputed sell card cache.
fn compute_features(
    player: &PlayerState,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    card_lookup: &[Card; 256],
    round: u32,
    params: &DiffEvalParams,
    table: &DiffEvalTable,
    cache: &SellCardCache,
) -> [f32; MLP_INPUT_SIZE] {
    let w = &params.weights;
    let mut inputs = [0.0f32; MLP_INPUT_SIZE];

    // ── Single pass over all card sets ──
    let mut production = [0u32; NUM_COLORS];
    let mut card_type_counts = [0u32; 46];
    let mut card_count = 0u32;
    let mut workshopped_count = 0u32;
    let mut action_counts = [0u32; 5];
    let mut mat_card_counts = [0u32; 3];

    let card_sets: [&crate::unordered_cards::UnorderedCards; 5] = [
        &player.deck, &player.discard, &player.workshop_cards, &player.workshopped_cards, &player.drafted_cards,
    ];
    let workshopped_idx = 3;

    for (set_idx, cards) in card_sets.iter().enumerate() {
        for id in cards.iter() {
            let card = card_lookup[id as usize];
            let card_idx = card as usize;
            card_count += 1;
            card_type_counts[card_idx] += 1;

            if set_idx == workshopped_idx {
                workshopped_count += 1;
            }

            for c in 0..NUM_COLORS {
                production[c] += table.color_production[card_idx][c] as u32;
            }

            match table.category[card_idx] {
                DeckCardCategory::ActionAlum => action_counts[0] += 1,
                DeckCardCategory::ActionCreamOfTartar => action_counts[1] += 1,
                DeckCardCategory::ActionGumArabic => action_counts[2] += 1,
                DeckCardCategory::ActionPotash => action_counts[3] += 1,
                DeckCardCategory::ActionChalk => action_counts[4] += 1,
                DeckCardCategory::MaterialStarter => mat_card_counts[0] += 1,
                DeckCardCategory::MaterialColored => mat_card_counts[1] += 1,
                DeckCardCategory::MaterialDual => mat_card_counts[2] += 1,
                DeckCardCategory::Dye | DeckCardCategory::ActionOther => {}
            }
        }
    }

    // ── Module 1: Color wheel value → inputs[0..7] ──
    for (tier, colors) in [&PRIMARIES[..], &SECONDARIES[..], &TERTIARIES[..]].iter().enumerate() {
        let sat_w = w[COLOR_SAT_W + tier] as f32;
        let sat_a = w[COLOR_SAT_A + tier] as f32;
        let mut tier_sum = 0.0f32;
        for &c in *colors {
            let count = player.color_wheel.get(c) as f32;
            tier_sum += sat_w * (1.0f32 + sat_a * count).ln();
        }
        inputs[tier] = tier_sum;
    }

    let mut mix_total = 0.0f32;
    for (i, &(a, b)) in VALID_MIX_PAIRS.iter().enumerate() {
        let count_a = player.color_wheel.get(a) as f32;
        let count_b = player.color_wheel.get(b) as f32;
        mix_total += w[MIX_PAIR_W + i] as f32 * count_a.min(count_b);
    }
    inputs[3] = mix_total;

    for (tier, colors) in [&PRIMARIES[..], &SECONDARIES[..], &TERTIARIES[..]].iter().enumerate() {
        let num_distinct = colors.iter().filter(|&&c| player.color_wheel.get(c) > 0).count() as f32;
        let x = w[COVERAGE_A + tier] as f32 * num_distinct - w[COVERAGE_B + tier] as f32;
        inputs[4 + tier] = w[COVERAGE_W + tier] as f32 * sigmoid_f32(x);
    }

    // ── Module 2: Sell card alignment → inputs[7..12] ──
    let mut alignments = [0.0f32; MAX_SELL_CARD_DISPLAY];
    let n_sell = sell_card_display.len();

    for (i, bi) in sell_card_display.iter().enumerate() {
        let sell_card = bi.sell_card;
        let ducats = sell_card.ducats();
        let mat_type = sell_card.required_material();

        let has_mat = if player.materials.get(mat_type) > 0 { 1.0f32 } else { 0.0f32 };
        let mat_match = sigmoid_f32(w[SELL_MAT_W + mat_type as usize] as f32 * has_mat);

        let cost = sell_card.color_cost();
        let cost_len = cost.len() as f32;
        let color_matches: f32 = cost.iter()
            .map(|&c| (player.color_wheel.get(c) as f32).min(1.0))
            .sum();
        let color_ratio = if cost_len > 0.0 { color_matches / cost_len } else { 0.0 };

        let ducat_tier = match ducats {
            2 => 0,
            3 => 1,
            _ => 2,
        };
        let weighted_color = w[SELL_DUCAT_W + ducat_tier] as f32 * color_ratio;

        alignments[i] = w[SELL_COMBINE_W] as f32 * mat_match + w[SELL_COMBINE_W + 1] as f32 * weighted_color;
    }

    let mut sorted = [0.0f32; MAX_SELL_CARD_DISPLAY];
    sorted[..n_sell].copy_from_slice(&alignments[..n_sell]);
    sorted[..n_sell].sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

    inputs[7] = if n_sell > 0 { sorted[0] } else { 0.0 };
    inputs[8] = if n_sell > 1 { sorted[1] } else { 0.0 };
    inputs[9] = if n_sell > 2 { sorted[2..n_sell].iter().sum() } else { 0.0 };

    let round_f = round as f32;
    inputs[10] = sigmoid_f32(w[SELL_ROUND_W] as f32 * round_f - w[SELL_ROUND_W + 1] as f32);

    let sold_count = player.completed_sell_cards.len() as f32;
    inputs[11] = w[SELL_SOLD_W] as f32 * sold_count * sigmoid_f32(w[SELL_SOLD_W + 1] as f32 * round_f - w[SELL_SOLD_W + 2] as f32);

    // ── Module 3: Deck color profile → inputs[12..21] ──
    let mut distinct_colors = 0u32;
    let mut tier_sums = [0.0f32; 3];
    for c in 0..NUM_COLORS {
        let count = production[c] as f32;
        if count > 0.0 {
            distinct_colors += 1;
        }
        let tier = color_tier(Color::from_index(c));
        let sat_w = w[DECK_COLOR_SAT_W + tier] as f32;
        let sat_a = w[DECK_COLOR_SAT_A + tier] as f32;
        tier_sums[tier] += sat_w * (1.0f32 + sat_a * count).ln();
    }
    inputs[12] = tier_sums[0];
    inputs[13] = tier_sums[1];
    inputs[14] = tier_sums[2];

    let mut prod_need_total = 0.0f32;
    for bi in sell_card_display.iter() {
        let sell_card = bi.sell_card;
        let cost = sell_card.color_cost();
        let producible = cost.iter().filter(|&&c| production[c.index()] > 0).count() as f32;
        let cost_len = cost.len() as f32;
        let fraction = if cost_len > 0.0 { producible / cost_len } else { 0.0 };

        let ducat_tier = match sell_card.ducats() {
            2 => 0,
            3 => 1,
            _ => 2,
        };
        prod_need_total += w[DECK_PROD_NEED_W + ducat_tier] as f32 * fraction;
    }
    inputs[15] = prod_need_total;

    let mut action_total = 0.0f32;
    for i in 0..5 {
        action_total += w[DECK_ACTION_W + i] as f32 * action_counts[i] as f32;
    }
    inputs[16] = action_total;

    let mut mat_card_total = 0.0f32;
    for i in 0..3 {
        mat_card_total += w[DECK_MAT_CARD_W + i] as f32 * mat_card_counts[i] as f32;
    }
    inputs[17] = mat_card_total;

    let size = card_count as f32;
    inputs[18] = w[DECK_SIZE_W] as f32 * size + w[DECK_SIZE_W + 1] as f32 * size * size;
    inputs[19] = w[DECK_DIVERSITY_W] as f32 * distinct_colors as f32 / 12.0;
    inputs[20] = w[DECK_WORKSHOP_W] as f32 * workshopped_count as f32;

    // ── Module 4: Material strategy → inputs[21..28] ──
    let mut types_with_material = 0u32;
    for i in 0..3 {
        let stored = player.materials.counts[i] as f32;
        let x = w[MAT_SUFF_W + i] as f32 * (stored - w[MAT_SUFF_THRESH + i] as f32);
        inputs[21 + i] = sigmoid_f32(x);

        if stored > 0.0 {
            types_with_material += 1;
        }

        let availability = sigmoid_f32(stored - 0.5);
        inputs[24 + i] = w[MAT_DEMAND_W + i] as f32 * cache.mat_demand[i] * availability;
    }

    let mut diversity = 0.0f32;
    if types_with_material >= 2 {
        diversity += w[MAT_DIVERSITY_W] as f32;
    }
    if types_with_material >= 3 {
        diversity += w[MAT_DIVERSITY_W + 1] as f32;
    }
    inputs[27] = diversity;

    // ── Direct inputs → inputs[28..30] ──
    inputs[28] = player.cached_score as f32 / 20.0;
    inputs[29] = round_f / 20.0;

    // ── Raw features → inputs[30..117] ──
    for c in 0..NUM_COLORS {
        inputs[30 + c] = player.color_wheel.counts[c] as f32;
    }

    for c in 0..NUM_COLORS {
        inputs[42 + c] = production[c] as f32;
    }

    // Sell card color demand [54..66] (from precomputed cache)
    for c in 0..NUM_COLORS {
        inputs[54 + c] = cache.sell_demand[c] as f32;
    }

    for i in 0..46 {
        inputs[66 + i] = card_type_counts[i] as f32;
    }

    for i in 0..3 {
        inputs[112 + i] = player.materials.counts[i] as f32;
    }

    inputs[115] = player.completed_sell_cards.len() as f32;
    inputs[116] = player.ducats as f32;

    inputs
}

/// Batched f32 MLP: reads W1 once for all players.
/// Iterates hidden units in the outer loop so each W1 row is loaded once.
fn batched_mlp_f32(
    all_inputs: &[[f32; MLP_INPUT_SIZE]; MAX_PLAYERS],
    n: usize,
    table: &DiffEvalTable,
) -> [f32; MAX_PLAYERS] {
    let mut hidden1 = [[0.0f32; MLP_HIDDEN_SIZE]; MAX_PLAYERS];

    // Hidden layer 1: iterate rows of W1, apply to all players
    for row in 0..MLP_HIDDEN_SIZE {
        let bias = table.b1_f32[row];
        let w1_row = &table.w1_f32[row * MLP_INPUT_SIZE..(row + 1) * MLP_INPUT_SIZE];
        for p in 0..n {
            let mut sum = bias;
            for col in 0..MLP_INPUT_SIZE {
                sum += w1_row[col] * all_inputs[p][col];
            }
            hidden1[p][row] = if sum > 0.0 { sum } else { LEAKY_RELU_ALPHA_F32 * sum };
        }
    }

    // Hidden layer 2: MLP_HIDDEN_SIZE → MLP_HIDDEN2_SIZE
    let mut hidden2 = [[0.0f32; MLP_HIDDEN2_SIZE]; MAX_PLAYERS];
    for row in 0..MLP_HIDDEN2_SIZE {
        let bias = table.b2_f32[row];
        let w2_row = &table.w2_f32[row * MLP_HIDDEN_SIZE..(row + 1) * MLP_HIDDEN_SIZE];
        for p in 0..n {
            let mut sum = bias;
            for col in 0..MLP_HIDDEN_SIZE {
                sum += w2_row[col] * hidden1[p][col];
            }
            hidden2[p][row] = if sum > 0.0 { sum } else { LEAKY_RELU_ALPHA_F32 * sum };
        }
    }

    // Output layer: MLP_HIDDEN2_SIZE → 1
    let mut outputs = [0.0f32; MAX_PLAYERS];
    for p in 0..n {
        let mut out = table.b3_f32;
        for i in 0..MLP_HIDDEN2_SIZE {
            out += table.w3_f32[i] * hidden2[p][i];
        }
        outputs[p] = out;
    }
    outputs
}

/// Full forward pass: compute evaluation logit for a single player.
/// Uses f64 MLP for compatibility with gradient finite-difference tests.
pub fn diff_eval_score(
    player: &PlayerState,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    card_lookup: &[Card; 256],
    round: u32,
    params: &DiffEvalParams,
    table: &DiffEvalTable,
) -> f64 {
    let cache = SellCardCache::new(sell_card_display);
    let features_f32 = compute_features(player, sell_card_display, card_lookup, round, params, table, &cache);
    let mut inputs = [0.0f64; MLP_INPUT_SIZE];
    for i in 0..MLP_INPUT_SIZE {
        inputs[i] = features_f32[i] as f64;
    }
    aggregation_mlp(&inputs, &params.weights)
}

/// Compute per-player rewards using softmax over diff eval logits.
/// Each player gets P(win) = softmax(logit_i).
pub fn compute_diff_eval_rewards(
    players: &FixedVec<PlayerState, MAX_PLAYERS>,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    card_lookup: &[Card; 256],
    round: u32,
    params: &DiffEvalParams,
    table: &DiffEvalTable,
) -> [f64; MAX_PLAYERS] {
    let n = players.len();

    // Precompute sell card data shared across all players
    let cache = SellCardCache::new(sell_card_display);

    // Compute f32 features directly for batched MLP
    let mut all_inputs = [[0.0f32; MLP_INPUT_SIZE]; MAX_PLAYERS];
    for (i, p) in players.iter().enumerate() {
        all_inputs[i] = compute_features(p, sell_card_display, card_lookup, round, params, table, &cache);
    }

    // Batched f32 MLP (reads W1 once for all players)
    let logits_f32 = batched_mlp_f32(&all_inputs, n, table);

    // Softmax in f64 for numerical stability
    let mut logits = [0.0f64; MAX_PLAYERS];
    for i in 0..n {
        logits[i] = logits_f32[i] as f64;
    }
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
    use crate::unordered_cards::UnorderedCards;
    use smallvec::SmallVec;

    fn make_empty_player() -> PlayerState {
        PlayerState {
            deck: UnorderedCards::new(),
            discard: UnorderedCards::new(),
            workshopped_cards: UnorderedCards::new(),
            workshop_cards: UnorderedCards::new(),
            drafted_cards: UnorderedCards::new(),
            color_wheel: ColorWheel::new(),
            materials: Materials::new(),
            completed_sell_cards: SmallVec::new(),
            completed_glass: SmallVec::new(),
            ducats: 0,
            cached_score: 0,
        }
    }

    #[test]
    fn test_default_params_forward_pass() {
        let params = DiffEvalParams::default();
        let table = DiffEvalTable::new(&params);
        let card_lookup = [Card::BasicRed; 256];
        let display: FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY> = FixedVec::new();

        let player = make_empty_player();
        let score = diff_eval_score(&player, &display, &card_lookup, 0, &params, &table);

        // With an empty player and no sell cards, the score should be finite
        assert!(score.is_finite(), "Score should be finite, got {}", score);
    }

    #[test]
    fn test_different_players_get_different_scores() {
        let params = DiffEvalParams::default();
        let table = DiffEvalTable::new(&params);
        let card_lookup = [Card::BasicRed; 256];
        let display: FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY> = FixedVec::new();

        let empty = make_empty_player();
        let empty_score = diff_eval_score(&empty, &display, &card_lookup, 0, &params, &table);

        let mut colored = make_empty_player();
        colored.color_wheel.set(Color::Red, 2);
        colored.color_wheel.set(Color::Yellow, 1);
        colored.color_wheel.set(Color::Vermilion, 1);
        let colored_score = diff_eval_score(&colored, &display, &card_lookup, 0, &params, &table);

        // With untrained weights, we just verify scores are finite and different
        assert!(empty_score.is_finite());
        assert!(colored_score.is_finite());
        assert!((colored_score - empty_score).abs() > 1e-10,
            "Different player states should produce different scores");
    }

    #[test]
    fn test_softmax_rewards_sum_to_one() {
        let params = DiffEvalParams::default();
        let table = DiffEvalTable::new(&params);
        let card_lookup = [Card::BasicRed; 256];
        let display: FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY> = FixedVec::new();

        let mut players: FixedVec<PlayerState, MAX_PLAYERS> = FixedVec::new();
        let mut p1 = make_empty_player();
        p1.ducats = 5;
        p1.cached_score = 5;
        let p2 = make_empty_player();
        players.push(p1);
        players.push(p2);

        let rewards = compute_diff_eval_rewards(&players, &display, &card_lookup, 3, &params, &table);
        let sum: f64 = rewards[..2].iter().sum();
        assert!((sum - 1.0).abs() < 1e-10, "Rewards should sum to 1.0, got {}", sum);
        assert!(rewards[0] > rewards[1], "Player with higher score should have higher reward");
    }

    #[test]
    fn test_diff_eval_table_color_production() {
        let params = DiffEvalParams::default();
        let table = DiffEvalTable::new(&params);

        // Kermes produces 3 red pips
        let kermes_idx = Card::Kermes as usize;
        assert_eq!(table.color_production[kermes_idx][Color::Red.index()], 3);
        assert_eq!(table.color_production[kermes_idx][Color::Yellow.index()], 0);

        // Madder produces 1 orange + 1 red
        let madder_idx = Card::Madder as usize;
        assert_eq!(table.color_production[madder_idx][Color::Orange.index()], 1);
        assert_eq!(table.color_production[madder_idx][Color::Red.index()], 1);

        // Saffron produces 1 amber
        let saffron_idx = Card::Saffron as usize;
        assert_eq!(table.color_production[saffron_idx][Color::Amber.index()], 1);

        // Alum produces no colors
        let alum_idx = Card::Alum as usize;
        assert_eq!(table.color_production[alum_idx].iter().sum::<u8>(), 0);
    }

    #[test]
    fn test_diff_eval_table_categories() {
        let params = DiffEvalParams::default();
        let table = DiffEvalTable::new(&params);

        assert_eq!(table.category[Card::BasicRed as usize], DeckCardCategory::Dye);
        assert_eq!(table.category[Card::Kermes as usize], DeckCardCategory::Dye);
        assert_eq!(table.category[Card::Alum as usize], DeckCardCategory::ActionAlum);
        assert_eq!(table.category[Card::Chalk as usize], DeckCardCategory::ActionChalk);
        assert_eq!(table.category[Card::StarterCeramics as usize], DeckCardCategory::MaterialStarter);
        assert_eq!(table.category[Card::TerraCotta as usize], DeckCardCategory::MaterialColored);
        assert_eq!(table.category[Card::ClayCanvas as usize], DeckCardCategory::MaterialDual);
    }
}
