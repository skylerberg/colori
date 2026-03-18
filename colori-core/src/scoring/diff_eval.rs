use serde::{Deserialize, Serialize};

use crate::colors::{PRIMARIES, SECONDARIES, TERTIARIES, VALID_MIX_PAIRS};
use crate::fixed_vec::FixedVec;
use crate::types::*;

/// MLP architecture constants
pub const MLP_INPUT_SIZE: usize = 117;
pub const MLP_HIDDEN_SIZE: usize = 256;

/// Number of learnable parameters in the differentiable evaluation model.
pub const NUM_PARAMS: usize = 30540;

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

// Aggregation MLP: MLP_INPUT_SIZE → MLP_HIDDEN_SIZE → 1
pub(crate) const MLP_W1: usize = 72;                                                       // [MLP_INPUT_SIZE * MLP_HIDDEN_SIZE = 29952]
pub(crate) const MLP_B1: usize = MLP_W1 + MLP_INPUT_SIZE * MLP_HIDDEN_SIZE;                // 30024, [MLP_HIDDEN_SIZE = 256]
pub(crate) const MLP_W2: usize = MLP_B1 + MLP_HIDDEN_SIZE;                                 // 30280, [MLP_HIDDEN_SIZE = 256]
pub(crate) const MLP_B2: usize = MLP_W2 + MLP_HIDDEN_SIZE;                                 // 30536, [1]

/// Number of differentiable parameters (indices 0..NUM_DIFF_PARAMS are updated by gradient descent).
pub const NUM_DIFF_PARAMS: usize = MLP_B2 + 1;                                             // 30537

// Heuristic control parameters (non-differentiable)
pub(crate) const HEURISTIC_ROUND_THRESHOLD: usize = NUM_DIFF_PARAMS;                       // 30537
pub(crate) const HEURISTIC_LOOKAHEAD: usize = HEURISTIC_ROUND_THRESHOLD + 1;               // 30538

pub(crate) const PROGRESSIVE_BIAS_WEIGHT: usize = HEURISTIC_LOOKAHEAD + 1;                 // 30539

/// All learnable weights for the differentiable evaluation model.
/// Stored as a Vec for serde compatibility (Rust serde doesn't support large arrays).
/// The Vec must have exactly NUM_PARAMS elements.
#[derive(Debug, Clone)]
pub struct DiffEvalParams {
    pub weights: [f64; NUM_PARAMS],
}

impl Serialize for DiffEvalParams {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.weights.as_slice().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DiffEvalParams {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let vec = Vec::<f64>::deserialize(deserializer)?;
        if vec.len() > NUM_PARAMS {
            return Err(serde::de::Error::custom(
                format!("expected at most {} weights, got {}", NUM_PARAMS, vec.len())
            ));
        }
        // Pad with defaults for backwards compatibility with older checkpoints
        let defaults = DiffEvalParams::default();
        let mut weights = defaults.weights;
        weights[..vec.len()].copy_from_slice(&vec);
        Ok(DiffEvalParams { weights })
    }
}

impl Default for DiffEvalParams {
    fn default() -> Self {
        let mut w = [0.0; NUM_PARAMS];

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

        // Aggregation MLP: Xavier initialization
        // W1: MLP_INPUT_SIZE x MLP_HIDDEN_SIZE, scale = sqrt(2.0 / (input + hidden))
        let w1_scale = (2.0f64 / (MLP_INPUT_SIZE as f64 + MLP_HIDDEN_SIZE as f64)).sqrt();
        for row in 0..MLP_HIDDEN_SIZE {
            for col in 0..MLP_INPUT_SIZE {
                let sign = if (row + col) % 2 == 0 { 1.0 } else { -1.0 };
                w[MLP_W1 + row * MLP_INPUT_SIZE + col] = sign * w1_scale;
            }
        }
        // B1: zeros
        // (already 0.0)

        // W2: MLP_HIDDEN_SIZE x 1, scale = sqrt(2.0 / (hidden + 1))
        let w2_scale = (2.0f64 / (MLP_HIDDEN_SIZE as f64 + 1.0)).sqrt();
        for i in 0..MLP_HIDDEN_SIZE {
            let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
            w[MLP_W2 + i] = sign * w2_scale;
        }
        // B2: zero
        // (already 0.0)

        // Control params
        w[HEURISTIC_ROUND_THRESHOLD] = 3.0;
        w[HEURISTIC_LOOKAHEAD] = 3.0;
        w[PROGRESSIVE_BIAS_WEIGHT] = 0.0;

        DiffEvalParams { weights: w }
    }
}

impl DiffEvalParams {
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
    pub fn new() -> Self {
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

        DiffEvalTable { color_production, category }
    }
}

// ── Forward pass ──

#[inline]
fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

#[inline]
fn color_tier(color: Color) -> usize {
    let idx = color.index();
    if idx % 4 == 0 { 0 }       // primary: 0, 4, 8
    else if idx % 2 == 0 { 1 }  // secondary: 2, 6, 10
    else { 2 }                   // tertiary: 1, 3, 5, 7, 9, 11
}

/// Module 1: Color wheel value (7 outputs)
/// [0-2] Per-tier saturation
/// [3]   Mix-pair total
/// [4-6] Per-tier coverage
fn color_wheel_value(player: &PlayerState, w: &[f64; NUM_PARAMS]) -> [f64; 7] {
    let mut out = [0.0f64; 7];

    // Log-saturation per tier
    for (tier, colors) in [&PRIMARIES[..], &SECONDARIES[..], &TERTIARIES[..]].iter().enumerate() {
        let sat_w = w[COLOR_SAT_W + tier];
        let sat_a = w[COLOR_SAT_A + tier];
        let mut tier_sum = 0.0;
        for &c in *colors {
            let count = player.color_wheel.get(c) as f64;
            tier_sum += sat_w * (1.0 + sat_a * count).ln();
        }
        out[tier] = tier_sum;
    }

    // Mix-pair interaction
    let mut mix_total = 0.0;
    for (i, &(a, b)) in VALID_MIX_PAIRS.iter().enumerate() {
        let count_a = player.color_wheel.get(a) as f64;
        let count_b = player.color_wheel.get(b) as f64;
        mix_total += w[MIX_PAIR_W + i] * count_a.min(count_b);
    }
    out[3] = mix_total;

    // Coverage gating per tier
    for (tier, colors) in [&PRIMARIES[..], &SECONDARIES[..], &TERTIARIES[..]].iter().enumerate() {
        let num_distinct = colors.iter().filter(|&&c| player.color_wheel.get(c) > 0).count() as f64;
        let x = w[COVERAGE_A + tier] * num_distinct - w[COVERAGE_B + tier];
        out[4 + tier] = w[COVERAGE_W + tier] * sigmoid(x);
    }

    out
}

/// Module 2: Sell card alignment (5 outputs)
/// [0] Best alignment score
/// [1] Second best alignment score
/// [2] Sum of remaining alignment scores
/// [3] Urgency
/// [4] Sold value
fn sell_card_alignment(
    player: &PlayerState,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    round: u32,
    w: &[f64; NUM_PARAMS],
) -> [f64; 5] {
    let mut out = [0.0f64; 5];
    let mut alignments = [0.0f64; MAX_SELL_CARD_DISPLAY];
    let n = sell_card_display.len();

    for (i, bi) in sell_card_display.iter().enumerate() {
        let sell_card = bi.sell_card;
        let ducats = sell_card.ducats();
        let mat_type = sell_card.required_material();

        // Material match
        let has_mat = if player.materials.get(mat_type) > 0 { 1.0 } else { 0.0 };
        let mat_match = sigmoid(w[SELL_MAT_W + mat_type as usize] * has_mat);

        // Color match ratio
        let cost = sell_card.color_cost();
        let cost_len = cost.len() as f64;
        let color_matches: f64 = cost.iter()
            .map(|&c| (player.color_wheel.get(c) as f64).min(1.0))
            .sum();
        let color_ratio = if cost_len > 0.0 { color_matches / cost_len } else { 0.0 };

        // Weight by ducat tier
        let ducat_tier = match ducats {
            2 => 0,
            3 => 1,
            _ => 2,
        };
        let weighted_color = w[SELL_DUCAT_W + ducat_tier] * color_ratio;

        // Combine
        alignments[i] = w[SELL_COMBINE_W] * mat_match + w[SELL_COMBINE_W + 1] * weighted_color;
    }

    // Sort descending to get best, second, rest
    let mut sorted = [0.0f64; MAX_SELL_CARD_DISPLAY];
    sorted[..n].copy_from_slice(&alignments[..n]);
    sorted[..n].sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

    out[0] = if n > 0 { sorted[0] } else { 0.0 };
    out[1] = if n > 1 { sorted[1] } else { 0.0 };
    out[2] = if n > 2 { sorted[2..n].iter().sum() } else { 0.0 };

    // Round-dependent urgency
    let round_f = round as f64;
    out[3] = sigmoid(w[SELL_ROUND_W] * round_f - w[SELL_ROUND_W + 1]);

    // Already-sold scaling
    let sold_count = player.completed_sell_cards.len() as f64;
    out[4] = w[SELL_SOLD_W] * sold_count * sigmoid(w[SELL_SOLD_W + 1] * round_f - w[SELL_SOLD_W + 2]);

    out
}

/// Module 3: Deck color profile (9 outputs)
/// [0-2] Per-tier production saturation
/// [3]   Production-need interaction total
/// [4]   Action card value total
/// [5]   Material card value total
/// [6]   Deck size effect
/// [7]   Diversity
/// [8]   Workshopped bonus
fn deck_color_profile(
    player: &PlayerState,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    card_lookup: &[Card; 256],
    w: &[f64; NUM_PARAMS],
    table: &DiffEvalTable,
) -> [f64; 9] {
    let mut out = [0.0f64; 9];

    // Accumulate per-color production counts and card category counts
    let mut production = [0u32; NUM_COLORS];
    let mut card_count = 0u32;
    let mut workshopped_count = 0u32;
    let mut action_counts = [0u32; 5]; // Alum, CreamOfTartar, GumArabic, Potash, Chalk
    let mut mat_card_counts = [0u32; 3]; // starter, colored, dual

    let card_sets: [&crate::unordered_cards::UnorderedCards; 5] = [
        &player.deck, &player.discard, &player.workshop_cards, &player.workshopped_cards, &player.drafted_cards,
    ];
    let workshopped_idx = 3; // index of workshopped_cards in the array above

    for (set_idx, cards) in card_sets.iter().enumerate() {
        for id in cards.iter() {
            let card = card_lookup[id as usize];
            let card_idx = card as usize;
            card_count += 1;

            if set_idx == workshopped_idx {
                workshopped_count += 1;
            }

            // Accumulate color production
            for c in 0..NUM_COLORS {
                production[c] += table.color_production[card_idx][c] as u32;
            }

            // Accumulate category counts
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

    // Per-color production with diminishing returns, grouped by tier
    let mut distinct_colors = 0u32;
    let mut tier_sums = [0.0f64; 3];
    for c in 0..NUM_COLORS {
        let count = production[c] as f64;
        if count > 0.0 {
            distinct_colors += 1;
        }
        let tier = color_tier(Color::from_index(c));
        let sat_w = w[DECK_COLOR_SAT_W + tier];
        let sat_a = w[DECK_COLOR_SAT_A + tier];
        tier_sums[tier] += sat_w * (1.0 + sat_a * count).ln();
    }
    out[0] = tier_sums[0];
    out[1] = tier_sums[1];
    out[2] = tier_sums[2];

    // Production-need interaction with sell cards
    let mut prod_need_total = 0.0;
    for bi in sell_card_display.iter() {
        let sell_card = bi.sell_card;
        let cost = sell_card.color_cost();
        let producible = cost.iter().filter(|&&c| production[c.index()] > 0).count() as f64;
        let cost_len = cost.len() as f64;
        let fraction = if cost_len > 0.0 { producible / cost_len } else { 0.0 };

        let ducat_tier = match sell_card.ducats() {
            2 => 0,
            3 => 1,
            _ => 2,
        };
        prod_need_total += w[DECK_PROD_NEED_W + ducat_tier] * fraction;
    }
    out[3] = prod_need_total;

    // Action card values
    let mut action_total = 0.0;
    for i in 0..5 {
        action_total += w[DECK_ACTION_W + i] * action_counts[i] as f64;
    }
    out[4] = action_total;

    // Material card subcategory values
    let mut mat_card_total = 0.0;
    for i in 0..3 {
        mat_card_total += w[DECK_MAT_CARD_W + i] * mat_card_counts[i] as f64;
    }
    out[5] = mat_card_total;

    // Deck size effect
    let size = card_count as f64;
    out[6] = w[DECK_SIZE_W] * size + w[DECK_SIZE_W + 1] * size * size;

    // Diversity
    out[7] = w[DECK_DIVERSITY_W] * distinct_colors as f64 / 12.0;

    // Workshopped bonus
    out[8] = w[DECK_WORKSHOP_W] * workshopped_count as f64;

    out
}

/// Module 4: Material strategy (7 outputs)
/// [0-2] Per-type sufficiency
/// [3-5] Per-type demand
/// [6]   Diversity value
fn material_strategy(
    player: &PlayerState,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    w: &[f64; NUM_PARAMS],
) -> [f64; 7] {
    let mut out = [0.0f64; 7];

    // Material sufficiency per type
    let mut types_with_material = 0u32;
    for i in 0..3 {
        let stored = player.materials.counts[i] as f64;
        let x = w[MAT_SUFF_W + i] * (stored - w[MAT_SUFF_THRESH + i]);
        out[i] = sigmoid(x);

        if stored > 0.0 {
            types_with_material += 1;
        }

        // Material x sell-card demand
        let mut demand = 0.0;
        for bi in sell_card_display.iter() {
            if bi.sell_card.required_material() as usize == i {
                demand += bi.sell_card.ducats() as f64;
            }
        }
        let availability = sigmoid(stored - 0.5); // ~1 if stored >= 1
        out[3 + i] = w[MAT_DEMAND_W + i] * demand * availability;
    }

    // Diversity bonus
    let mut diversity = 0.0;
    if types_with_material >= 2 {
        diversity += w[MAT_DIVERSITY_W];
    }
    if types_with_material >= 3 {
        diversity += w[MAT_DIVERSITY_W + 1];
    }
    out[6] = diversity;

    out
}

/// Extract raw features from player state for direct MLP input.
/// Returns counts for: color wheel (12), deck color production (12),
/// sell card color demand (12), card type counts (46), material counts (3),
/// completed sell cards (1), ducats (1).
/// Total: 12 + 12 + 12 + 46 + 3 + 1 + 1 = 87
fn extract_raw_features(
    player: &PlayerState,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    card_lookup: &[Card; 256],
    table: &DiffEvalTable,
) -> [f64; 87] {
    let mut out = [0.0f64; 87];

    // Raw color wheel counts [0..12]
    for c in 0..NUM_COLORS {
        out[c] = player.color_wheel.counts[c] as f64;
    }

    // Raw deck color production [12..24]
    // and card type counts [36..82]
    let mut production = [0u32; NUM_COLORS];
    let mut card_type_counts = [0u32; 46];

    let card_sets: [&crate::unordered_cards::UnorderedCards; 5] = [
        &player.deck, &player.discard, &player.workshop_cards, &player.workshopped_cards, &player.drafted_cards,
    ];

    for cards in card_sets.iter() {
        for id in cards.iter() {
            let card = card_lookup[id as usize];
            let card_idx = card as usize;

            // Color production
            for c in 0..NUM_COLORS {
                production[c] += table.color_production[card_idx][c] as u32;
            }

            // Card type count
            card_type_counts[card_idx] += 1;
        }
    }

    for c in 0..NUM_COLORS {
        out[12 + c] = production[c] as f64;
    }

    // Raw sell card color demand [24..36]
    let mut sell_demand = [0u32; NUM_COLORS];
    for bi in sell_card_display.iter() {
        let cost = bi.sell_card.color_cost();
        for &c in cost {
            sell_demand[c.index()] += 1;
        }
    }
    for c in 0..NUM_COLORS {
        out[24 + c] = sell_demand[c] as f64;
    }

    // Raw card type counts [36..82]
    for i in 0..46 {
        out[36 + i] = card_type_counts[i] as f64;
    }

    // Raw material counts [82..85]
    for i in 0..3 {
        out[82 + i] = player.materials.counts[i] as f64;
    }

    // Completed sell cards [85]
    out[85] = player.completed_sell_cards.len() as f64;

    // Ducats [86]
    out[86] = player.ducats as f64;

    out
}

/// Aggregation MLP: MLP_INPUT_SIZE → MLP_HIDDEN_SIZE (ReLU) → 1
fn aggregation_mlp(inputs: &[f64; MLP_INPUT_SIZE], w: &[f64; NUM_PARAMS]) -> f64 {
    // Hidden layer: MLP_INPUT_SIZE → MLP_HIDDEN_SIZE
    let mut hidden = [0.0f64; MLP_HIDDEN_SIZE];
    for row in 0..MLP_HIDDEN_SIZE {
        let mut sum = w[MLP_B1 + row];
        for col in 0..MLP_INPUT_SIZE {
            sum += w[MLP_W1 + row * MLP_INPUT_SIZE + col] * inputs[col];
        }
        hidden[row] = sum.max(0.0); // ReLU
    }

    // Output layer: MLP_HIDDEN_SIZE → 1
    let mut output = w[MLP_B2];
    for i in 0..MLP_HIDDEN_SIZE {
        output += w[MLP_W2 + i] * hidden[i];
    }

    output
}

/// Full forward pass: compute evaluation logit for a single player.
pub fn diff_eval_score(
    player: &PlayerState,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    card_lookup: &[Card; 256],
    round: u32,
    params: &DiffEvalParams,
    table: &DiffEvalTable,
) -> f64 {
    let w = &params.weights;

    let color_out = color_wheel_value(player, w);
    let sell_out = sell_card_alignment(player, sell_card_display, round, w);
    let deck_out = deck_color_profile(player, sell_card_display, card_lookup, w, table);
    let mat_out = material_strategy(player, sell_card_display, w);
    let raw = extract_raw_features(player, sell_card_display, card_lookup, table);

    let mut inputs = [0.0f64; MLP_INPUT_SIZE];

    // Module 1: Color Wheel Value (7 outputs) -> [0..7]
    inputs[0] = color_out[0];
    inputs[1] = color_out[1];
    inputs[2] = color_out[2];
    inputs[3] = color_out[3];
    inputs[4] = color_out[4];
    inputs[5] = color_out[5];
    inputs[6] = color_out[6];

    // Module 2: Sell Card Alignment (5 outputs) -> [7..12]
    inputs[7] = sell_out[0];
    inputs[8] = sell_out[1];
    inputs[9] = sell_out[2];
    inputs[10] = sell_out[3];
    inputs[11] = sell_out[4];

    // Module 3: Deck Color Profile (9 outputs) -> [12..21]
    for i in 0..9 {
        inputs[12 + i] = deck_out[i];
    }

    // Module 4: Material Strategy (7 outputs) -> [21..28]
    for i in 0..7 {
        inputs[21 + i] = mat_out[i];
    }

    // Direct inputs -> [28..30]
    inputs[28] = player.cached_score as f64 / 20.0;
    inputs[29] = round as f64 / 20.0;

    // Raw color wheel counts -> [30..42]
    for c in 0..NUM_COLORS {
        inputs[30 + c] = raw[c];
    }

    // Raw deck color production -> [42..54]
    for c in 0..NUM_COLORS {
        inputs[42 + c] = raw[12 + c];
    }

    // Raw sell card color demand -> [54..66]
    for c in 0..NUM_COLORS {
        inputs[54 + c] = raw[24 + c];
    }

    // Raw card type counts -> [66..112]
    for i in 0..46 {
        inputs[66 + i] = raw[36 + i];
    }

    // Raw material counts -> [112..115]
    for i in 0..3 {
        inputs[112 + i] = raw[82 + i];
    }

    // Completed sell cards and ducats -> [115..117]
    inputs[115] = raw[85];
    inputs[116] = raw[86];

    aggregation_mlp(&inputs, w)
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
    let mut logits = [0.0f64; MAX_PLAYERS];
    for (i, p) in players.iter().enumerate() {
        logits[i] = diff_eval_score(p, sell_card_display, card_lookup, round, params, table);
    }

    // Softmax
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
        let table = DiffEvalTable::new();
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
        let table = DiffEvalTable::new();
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
        let table = DiffEvalTable::new();
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
        let table = DiffEvalTable::new();

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
        let table = DiffEvalTable::new();

        assert_eq!(table.category[Card::BasicRed as usize], DeckCardCategory::Dye);
        assert_eq!(table.category[Card::Kermes as usize], DeckCardCategory::Dye);
        assert_eq!(table.category[Card::Alum as usize], DeckCardCategory::ActionAlum);
        assert_eq!(table.category[Card::Chalk as usize], DeckCardCategory::ActionChalk);
        assert_eq!(table.category[Card::StarterCeramics as usize], DeckCardCategory::MaterialStarter);
        assert_eq!(table.category[Card::TerraCotta as usize], DeckCardCategory::MaterialColored);
        assert_eq!(table.category[Card::ClayCanvas as usize], DeckCardCategory::MaterialDual);
    }
}
