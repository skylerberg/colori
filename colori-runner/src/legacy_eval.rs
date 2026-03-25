//! Legacy forward pass for the old 117-input diff_eval model.
//!
//! This is a self-contained, read-only reimplementation of the OLD diff_eval
//! architecture (72 hand-crafted module params + MLP on 117 inputs). It is used
//! solely for generating distillation targets when bootstrapping the new
//! 613-input neural net from a trained old model.
//!
//! Old model architecture:
//!   - 72 module params + MLP: 117 → 256 (LeakyReLU) → 64 (LeakyReLU) → 1
//!   - Total: 46,796 params

use colori_core::colors::VALID_MIX_PAIRS;
use colori_core::types::*;

use serde::Deserialize;

// ── Parameter count ──

const LEGACY_MLP_INPUT_SIZE: usize = 117;
const LEGACY_MLP_HIDDEN_SIZE: usize = 256;
const LEGACY_MLP_HIDDEN2_SIZE: usize = 64;

const LEGACY_NUM_PARAMS: usize = 46_796;

// ── Parameter index constants ──

const COLOR_SAT_W: usize = 0; // [3]
const COLOR_SAT_A: usize = 3; // [3]
const MIX_PAIR_W: usize = 6; // [9]
const COVERAGE_W: usize = 15; // [3]
const COVERAGE_A: usize = 18; // [3]
const COVERAGE_B: usize = 21; // [3]
const SELL_MAT_W: usize = 24; // [3]
const SELL_DUCAT_W: usize = 27; // [3]
const SELL_COMBINE_W: usize = 30; // [2]
#[allow(dead_code)]
const SELL_AGG_W: usize = 32; // [3]
const SELL_ROUND_W: usize = 35; // [2]
const SELL_SOLD_W: usize = 37; // [3]
const DECK_COLOR_SAT_W: usize = 40; // [3]
const DECK_COLOR_SAT_A: usize = 43; // [3]
const DECK_PROD_NEED_W: usize = 46; // [3]
const DECK_ACTION_W: usize = 49; // [5]
const DECK_MAT_CARD_W: usize = 54; // [3]
const DECK_SIZE_W: usize = 57; // [2]
const DECK_DIVERSITY_W: usize = 59; // [1]
const DECK_WORKSHOP_W: usize = 60; // [1]
const MAT_SUFF_W: usize = 61; // [3]
const MAT_SUFF_THRESH: usize = 64; // [3]
const MAT_DEMAND_W: usize = 67; // [3]
const MAT_DIVERSITY_W: usize = 70; // [2]
const MLP_W1: usize = 72;

// Derived MLP layout (starts at MLP_W1 = 72)
const MLP_B1: usize = MLP_W1 + LEGACY_MLP_INPUT_SIZE * LEGACY_MLP_HIDDEN_SIZE; // 72 + 29,952 = 30,024
const MLP_W2: usize = MLP_B1 + LEGACY_MLP_HIDDEN_SIZE; // 30,024 + 256 = 30,280
const MLP_B2: usize = MLP_W2 + LEGACY_MLP_HIDDEN_SIZE * LEGACY_MLP_HIDDEN2_SIZE; // 30,280 + 16,384 = 46,664
const MLP_W3: usize = MLP_B2 + LEGACY_MLP_HIDDEN2_SIZE; // 46,664 + 64 = 46,728
const MLP_B3: usize = MLP_W3 + LEGACY_MLP_HIDDEN2_SIZE; // 46,728 + 64 = 46,792

const LEAKY_RELU_ALPHA: f64 = 0.01;

// ── Legacy params struct ──

pub struct LegacyDiffEvalParams {
    pub weights: Vec<f64>,
}

impl LegacyDiffEvalParams {
    pub fn load(path: &str) -> Self {
        let contents = std::fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Failed to read legacy params file: {}", path));
        let weights: Vec<f64> = serde_json::from_str(&contents)
            .unwrap_or_else(|_| panic!("Failed to parse legacy params file: {}", path));
        assert!(
            weights.len() >= LEGACY_NUM_PARAMS,
            "Legacy params file has {} weights, expected at least {}",
            weights.len(),
            LEGACY_NUM_PARAMS
        );
        LegacyDiffEvalParams { weights }
    }
}

impl<'de> Deserialize<'de> for LegacyDiffEvalParams {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let weights = Vec::<f64>::deserialize(deserializer)?;
        Ok(LegacyDiffEvalParams { weights })
    }
}

// ── Helper functions ──

#[inline]
fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Determine color tier: 0 = primary, 1 = secondary, 2 = tertiary.
#[inline]
fn color_tier(color_index: usize) -> usize {
    if color_index % 4 == 0 {
        0 // primary
    } else if color_index % 2 == 0 {
        1 // secondary
    } else {
        2 // tertiary
    }
}

// ── Card categories for the legacy model ──

enum LegacyCardCategory {
    Dye,
    ActionAlum,
    ActionCreamOfTartar,
    ActionGumArabic,
    ActionPotash,
    ActionChalk,
    ActionOther,
    MaterialStarter,
    MaterialColored,
    MaterialDual,
}

fn classify_card(card: Card) -> LegacyCardCategory {
    match card {
        // Dyes (basic + regular)
        Card::BasicRed | Card::BasicYellow | Card::BasicBlue |
        Card::Kermes | Card::Weld | Card::Woad |
        Card::Lac | Card::Brazilwood | Card::Pomegranate |
        Card::Sumac | Card::Elderberry | Card::Turnsole |
        Card::Madder | Card::Turmeric | Card::DyersGreenweed |
        Card::Verdigris | Card::Orchil | Card::Logwood |
        Card::VermilionDye | Card::Saffron | Card::PersianBerries |
        Card::Azurite | Card::IndigoDye | Card::Cochineal => LegacyCardCategory::Dye,

        // Action cards
        Card::Alum => LegacyCardCategory::ActionAlum,
        Card::CreamOfTartar => LegacyCardCategory::ActionCreamOfTartar,
        Card::GumArabic => LegacyCardCategory::ActionGumArabic,
        Card::Potash => LegacyCardCategory::ActionPotash,
        Card::Chalk => LegacyCardCategory::ActionChalk,
        Card::Vinegar | Card::Argol => LegacyCardCategory::ActionOther,

        // Starter materials (no colors, single mat type)
        Card::StarterCeramics | Card::StarterPaintings | Card::StarterTextiles => {
            LegacyCardCategory::MaterialStarter
        }

        // Colored materials
        Card::TerraCotta | Card::OchreWare | Card::CobaltWare |
        Card::CinnabarCanvas | Card::OrpimentCanvas | Card::UltramarineCanvas |
        Card::AlizarinFabric | Card::FusticFabric | Card::PastelFabric => {
            LegacyCardCategory::MaterialColored
        }

        // Dual materials (2+ mat types)
        Card::ClayCanvas | Card::ClayFabric | Card::CanvasFabric => {
            LegacyCardCategory::MaterialDual
        }
    }
}

// ── Module 1: Color Wheel Value ──

fn compute_module1(
    player: &PlayerState,
    params: &[f64],
) -> [f64; 7] {
    let mut outputs = [0.0f64; 7];

    // Per tier log-saturation sum
    // Primary tier (index % 4 == 0): colors 0, 4, 8
    // Secondary tier (index % 2 == 0, not % 4): colors 2, 6, 10
    // Tertiary tier (odd index): colors 1, 3, 5, 7, 9, 11
    for tier in 0..3 {
        let sat_w = params[COLOR_SAT_W + tier];
        let sat_a = params[COLOR_SAT_A + tier];
        let mut sum = 0.0;
        for c in 0..NUM_COLORS {
            if color_tier(c) == tier {
                let count = player.color_wheel.counts[c] as f64;
                sum += sat_w * (1.0 + sat_a * count).ln();
            }
        }
        outputs[tier] = sum;
    }

    // Mix pair total: sum_i(w[MIX_PAIR_W+i] * min(count_a, count_b))
    let mut mix_total = 0.0;
    for (i, &(a, b)) in VALID_MIX_PAIRS.iter().enumerate() {
        let count_a = player.color_wheel.get(a) as f64;
        let count_b = player.color_wheel.get(b) as f64;
        mix_total += params[MIX_PAIR_W + i] * count_a.min(count_b);
    }
    outputs[3] = mix_total;

    // Per tier coverage: w[COVERAGE_W+tier] * sigmoid(w[COVERAGE_A+tier] * num_distinct - w[COVERAGE_B+tier])
    for tier in 0..3 {
        let mut num_distinct = 0.0;
        for c in 0..NUM_COLORS {
            if color_tier(c) == tier && player.color_wheel.counts[c] > 0 {
                num_distinct += 1.0;
            }
        }
        outputs[4 + tier] = params[COVERAGE_W + tier]
            * sigmoid(params[COVERAGE_A + tier] * num_distinct - params[COVERAGE_B + tier]);
    }

    outputs
}

// ── Module 2: Sell Card Alignment ──

fn compute_module2(
    player: &PlayerState,
    sell_card_display: &[SellCardInstance],
    round: u32,
    params: &[f64],
) -> [f64; 5] {
    let mut outputs = [0.0f64; 5];

    let mut alignments: Vec<f64> = Vec::new();

    for sci in sell_card_display.iter() {
        let sell_card = sci.sell_card;
        let ducats = sell_card.ducats();
        let mat_type = sell_card.required_material() as usize;

        // Material match
        let has_mat = if player.materials.get(sell_card.required_material()) > 0 {
            1.0
        } else {
            0.0
        };
        let mat_match = sigmoid(params[SELL_MAT_W + mat_type] * has_mat);

        // Color ratio
        let cost = sell_card.color_cost();
        let cost_len = cost.len() as f64;
        let mut has_count = 0.0;
        for &color in cost {
            if player.color_wheel.get(color) >= 1 {
                has_count += 1.0;
            }
        }
        let color_ratio = has_count / cost_len;

        // Ducat tier: 2-ducat = 0, 3-ducat = 1, 4-ducat = 2
        let ducat_tier = match ducats {
            2 => 0,
            3 => 1,
            _ => 2,
        };

        let weighted_color = params[SELL_DUCAT_W + ducat_tier] * color_ratio;

        let alignment =
            params[SELL_COMBINE_W] * mat_match + params[SELL_COMBINE_W + 1] * weighted_color;
        alignments.push(alignment);
    }

    // Sort descending
    alignments.sort_by(|a, b| b.partial_cmp(a).unwrap());

    // Best, second, sum_rest
    if !alignments.is_empty() {
        outputs[0] = alignments[0];
    }
    if alignments.len() > 1 {
        outputs[1] = alignments[1];
    }
    if alignments.len() > 2 {
        outputs[2] = alignments[2..].iter().sum();
    }

    // Urgency: sigmoid(w[SELL_ROUND_W] * round - w[SELL_ROUND_W+1])
    outputs[3] = sigmoid(params[SELL_ROUND_W] * round as f64 - params[SELL_ROUND_W + 1]);

    // Sold value: w[SELL_SOLD_W] * sold_count * sigmoid(w[SELL_SOLD_W+1] * round - w[SELL_SOLD_W+2])
    let sold_count = player.completed_sell_cards.len() as f64;
    outputs[4] = params[SELL_SOLD_W]
        * sold_count
        * sigmoid(params[SELL_SOLD_W + 1] * round as f64 - params[SELL_SOLD_W + 2]);

    outputs
}

// ── Module 3: Deck Color Profile ──

/// Action card category indices for DECK_ACTION_W:
/// 0 = Dye, 1 = ActionAlum, 2 = ActionCreamOfTartar, 3 = ActionGumArabic, 4 = ActionPotash+ActionChalk+ActionOther
fn action_category_index(cat: &LegacyCardCategory) -> Option<usize> {
    match cat {
        LegacyCardCategory::Dye => Some(0),
        LegacyCardCategory::ActionAlum => Some(1),
        LegacyCardCategory::ActionCreamOfTartar => Some(2),
        LegacyCardCategory::ActionGumArabic => Some(3),
        LegacyCardCategory::ActionPotash
        | LegacyCardCategory::ActionChalk
        | LegacyCardCategory::ActionOther => Some(4),
        _ => None,
    }
}

/// Material card category indices for DECK_MAT_CARD_W:
/// 0 = MaterialStarter, 1 = MaterialColored, 2 = MaterialDual
fn mat_card_category_index(cat: &LegacyCardCategory) -> Option<usize> {
    match cat {
        LegacyCardCategory::MaterialStarter => Some(0),
        LegacyCardCategory::MaterialColored => Some(1),
        LegacyCardCategory::MaterialDual => Some(2),
        _ => None,
    }
}

fn compute_module3(
    player: &PlayerState,
    sell_card_display: &[SellCardInstance],
    card_lookup: &[Card; 256],
    params: &[f64],
) -> [f64; 9] {
    let mut outputs = [0.0f64; 9];

    // Accumulate production counts and card category counts across all 5 zones
    let mut production = [0.0f64; NUM_COLORS];
    let mut action_counts = [0.0f64; 5]; // Dye, Alum, CreamOfTartar, GumArabic, Other
    let mut mat_card_counts = [0.0f64; 3]; // Starter, Colored, Dual
    let mut deck_size = 0.0f64;
    let mut distinct_colors = [false; NUM_COLORS];
    let mut workshopped_count = 0.0f64;

    let zones: [&colori_core::unordered_cards::UnorderedCards; 5] = [
        &player.deck,
        &player.discard,
        &player.workshop_cards,
        &player.workshopped_cards,
        &player.drafted_cards,
    ];

    for (zone_idx, zone) in zones.iter().enumerate() {
        for id in zone.iter() {
            let card = card_lookup[id as usize];
            deck_size += 1.0;

            if zone_idx == 3 {
                // workshopped zone
                workshopped_count += 1.0;
            }

            // Accumulate color production
            for &color in card.colors() {
                let ci = color.index();
                production[ci] += 1.0;
                distinct_colors[ci] = true;
            }

            // Card category counts
            let cat = classify_card(card);
            if let Some(idx) = action_category_index(&cat) {
                action_counts[idx] += 1.0;
            }
            if let Some(idx) = mat_card_category_index(&cat) {
                mat_card_counts[idx] += 1.0;
            }
        }
    }

    // Per-color production with log saturation by tier (same formula as Module 1)
    for tier in 0..3 {
        let sat_w = params[DECK_COLOR_SAT_W + tier];
        let sat_a = params[DECK_COLOR_SAT_A + tier];
        let mut sum = 0.0;
        for c in 0..NUM_COLORS {
            if color_tier(c) == tier {
                sum += sat_w * (1.0 + sat_a * production[c]).ln();
            }
        }
        outputs[tier] = sum;
    }

    // Production-need: for each sell card, fraction of cost colors that have production > 0,
    // weighted by ducat tier
    let mut prod_need = 0.0;
    for sci in sell_card_display.iter() {
        let sell_card = sci.sell_card;
        let cost = sell_card.color_cost();
        let cost_len = cost.len() as f64;
        let mut has_prod = 0.0;
        for &color in cost {
            if production[color.index()] > 0.0 {
                has_prod += 1.0;
            }
        }
        let fraction = has_prod / cost_len;
        let ducat_tier = match sell_card.ducats() {
            2 => 0,
            3 => 1,
            _ => 2,
        };
        prod_need += params[DECK_PROD_NEED_W + ducat_tier] * fraction;
    }
    outputs[3] = prod_need;

    // Action card total: sum(w[DECK_ACTION_W+i] * count[i])
    let mut action_total = 0.0;
    for i in 0..5 {
        action_total += params[DECK_ACTION_W + i] * action_counts[i];
    }
    outputs[4] = action_total;

    // Material card total: sum(w[DECK_MAT_CARD_W+i] * count[i])
    let mut mat_card_total = 0.0;
    for i in 0..3 {
        mat_card_total += params[DECK_MAT_CARD_W + i] * mat_card_counts[i];
    }
    outputs[5] = mat_card_total;

    // Deck size: w * size + w * size^2
    outputs[6] = params[DECK_SIZE_W] * deck_size + params[DECK_SIZE_W + 1] * deck_size * deck_size;

    // Diversity: w * distinct_colors / 12
    let num_distinct = distinct_colors.iter().filter(|&&d| d).count() as f64;
    outputs[7] = params[DECK_DIVERSITY_W] * num_distinct / 12.0;

    // Workshopped count
    outputs[8] = params[DECK_WORKSHOP_W] * workshopped_count;

    outputs
}

// ── Module 4: Material Strategy ──

fn compute_module4(
    player: &PlayerState,
    sell_card_display: &[SellCardInstance],
    params: &[f64],
) -> [f64; 7] {
    let mut outputs = [0.0f64; 7];

    // Material types: 0=Textiles, 1=Ceramics, 2=Paintings
    let stored = [
        player.materials.counts[0] as f64, // Textiles
        player.materials.counts[1] as f64, // Ceramics
        player.materials.counts[2] as f64, // Paintings
    ];

    // Per type sufficiency: sigmoid(w[MAT_SUFF_W+i] * (stored - w[MAT_SUFF_THRESH+i]))
    for i in 0..3 {
        outputs[i] =
            sigmoid(params[MAT_SUFF_W + i] * (stored[i] - params[MAT_SUFF_THRESH + i]));
    }

    // Per type demand: w[MAT_DEMAND_W+i] * sum_demand * sigmoid(stored - 0.5)
    for i in 0..3 {
        let mat_type = ALL_MATERIAL_TYPES[i];
        let mut sum_demand = 0.0;
        for sci in sell_card_display.iter() {
            if sci.sell_card.required_material() == mat_type {
                sum_demand += sci.sell_card.ducats() as f64;
            }
        }
        outputs[3 + i] =
            params[MAT_DEMAND_W + i] * sum_demand * sigmoid(stored[i] - 0.5);
    }

    // Diversity: w[MAT_DIVERSITY_W] if 2+ types have material, + w[MAT_DIVERSITY_W+1] if all 3 have
    let types_with_material = stored.iter().filter(|&&s| s > 0.0).count();
    if types_with_material >= 2 {
        outputs[6] += params[MAT_DIVERSITY_W];
    }
    if types_with_material >= 3 {
        outputs[6] += params[MAT_DIVERSITY_W + 1];
    }

    outputs
}

// ── Feature computation (117 inputs) ──

pub fn legacy_compute_features(
    player: &PlayerState,
    sell_card_display: &[SellCardInstance],
    card_lookup: &[Card; 256],
    round: u32,
    params: &LegacyDiffEvalParams,
) -> [f64; LEGACY_MLP_INPUT_SIZE] {
    let mut inputs = [0.0f64; LEGACY_MLP_INPUT_SIZE];
    let w = &params.weights;

    // Module 1: Color Wheel Value (7 outputs)
    let m1 = compute_module1(player, w);
    inputs[0..7].copy_from_slice(&m1);

    // Module 2: Sell Card Alignment (5 outputs)
    let m2 = compute_module2(player, sell_card_display, round, w);
    inputs[7..12].copy_from_slice(&m2);

    // Module 3: Deck Color Profile (9 outputs)
    let m3 = compute_module3(player, sell_card_display, card_lookup, w);
    inputs[12..21].copy_from_slice(&m3);

    // Module 4: Material Strategy (7 outputs)
    let m4 = compute_module4(player, sell_card_display, w);
    inputs[21..28].copy_from_slice(&m4);

    // Raw features
    inputs[28] = player.cached_score as f64 / 20.0;
    inputs[29] = round as f64 / 20.0;

    // Raw color wheel counts (12)
    for c in 0..NUM_COLORS {
        inputs[30 + c] = player.color_wheel.counts[c] as f64;
    }

    // Raw deck color production (12) — accumulate from all zones
    {
        let mut production = [0.0f64; NUM_COLORS];
        let zones = [
            &player.deck,
            &player.discard,
            &player.workshop_cards,
            &player.workshopped_cards,
            &player.drafted_cards,
        ];
        for zone in &zones {
            for id in zone.iter() {
                let card = card_lookup[id as usize];
                for &color in card.colors() {
                    production[color.index()] += 1.0;
                }
            }
        }
        for c in 0..NUM_COLORS {
            inputs[42 + c] = production[c];
        }
    }

    // Raw sell card color demand (12) — accumulate from sell card display
    {
        let mut demand = [0.0f64; NUM_COLORS];
        for sci in sell_card_display.iter() {
            for &color in sci.sell_card.color_cost() {
                demand[color.index()] += 1.0;
            }
        }
        for c in 0..NUM_COLORS {
            inputs[54 + c] = demand[c];
        }
    }

    // Raw card type counts (46) — accumulate from all zones
    {
        let zones = [
            &player.deck,
            &player.discard,
            &player.workshop_cards,
            &player.workshopped_cards,
            &player.drafted_cards,
        ];
        for zone in &zones {
            for id in zone.iter() {
                let card = card_lookup[id as usize];
                let card_idx = card as usize;
                inputs[66 + card_idx] += 1.0;
            }
        }
    }

    // Raw material counts (3)
    for i in 0..3 {
        inputs[112 + i] = player.materials.counts[i] as f64;
    }

    // Completed sell cards count
    inputs[115] = player.completed_sell_cards.len() as f64;

    // Ducats
    inputs[116] = player.ducats as f64;

    inputs
}

// ── MLP forward pass ──

pub fn legacy_mlp_forward(inputs: &[f64; LEGACY_MLP_INPUT_SIZE], weights: &[f64]) -> f64 {
    // Hidden layer 1: 117 -> 256
    let mut hidden1 = [0.0f64; LEGACY_MLP_HIDDEN_SIZE];
    for row in 0..LEGACY_MLP_HIDDEN_SIZE {
        let mut sum = weights[MLP_B1 + row];
        for col in 0..LEGACY_MLP_INPUT_SIZE {
            sum += weights[MLP_W1 + row * LEGACY_MLP_INPUT_SIZE + col] * inputs[col];
        }
        hidden1[row] = if sum > 0.0 { sum } else { LEAKY_RELU_ALPHA * sum };
    }

    // Hidden layer 2: 256 -> 64
    let mut hidden2 = [0.0f64; LEGACY_MLP_HIDDEN2_SIZE];
    for row in 0..LEGACY_MLP_HIDDEN2_SIZE {
        let mut sum = weights[MLP_B2 + row];
        for col in 0..LEGACY_MLP_HIDDEN_SIZE {
            sum += weights[MLP_W2 + row * LEGACY_MLP_HIDDEN_SIZE + col] * hidden1[col];
        }
        hidden2[row] = if sum > 0.0 { sum } else { LEAKY_RELU_ALPHA * sum };
    }

    // Output layer: 64 -> 1
    let mut output = weights[MLP_B3];
    for i in 0..LEGACY_MLP_HIDDEN2_SIZE {
        output += weights[MLP_W3 + i] * hidden2[i];
    }

    output
}

// ── Combined forward pass ──

pub fn legacy_diff_eval_score(
    player: &PlayerState,
    sell_card_display: &[SellCardInstance],
    card_lookup: &[Card; 256],
    round: u32,
    params: &LegacyDiffEvalParams,
) -> f64 {
    let inputs = legacy_compute_features(player, sell_card_display, card_lookup, round, params);
    legacy_mlp_forward(&inputs, &params.weights)
}
