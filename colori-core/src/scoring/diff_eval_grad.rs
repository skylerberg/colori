use crate::colors::{PRIMARIES, SECONDARIES, TERTIARIES, VALID_MIX_PAIRS};
use crate::fixed_vec::FixedVec;
use crate::types::*;

use super::diff_eval::*;

/// Accumulated gradients, same shape as DiffEvalParams.
/// Heap-allocated to avoid stack overflows with large parameter arrays.
#[derive(Debug, Clone)]
pub struct DiffEvalGradients {
    pub grads: Box<[f64; NUM_PARAMS]>,
}

impl DiffEvalGradients {
    pub fn zeros() -> Self {
        let grads: Box<[f64; NUM_PARAMS]> = vec![0.0f64; NUM_PARAMS].into_boxed_slice().try_into()
            .unwrap_or_else(|_| unreachable!());
        DiffEvalGradients { grads }
    }

    pub fn accumulate(&mut self, other: &DiffEvalGradients) {
        for i in 0..NUM_PARAMS {
            self.grads[i] += other.grads[i];
        }
    }

    pub fn scale(&mut self, factor: f64) {
        for g in self.grads.iter_mut() {
            *g *= factor;
        }
    }
}

#[inline]
fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

#[inline]
fn color_tier(color: Color) -> usize {
    let idx = color.index();
    if idx % 4 == 0 { 0 }
    else if idx % 2 == 0 { 1 }
    else { 2 }
}

/// Compute forward pass and cache intermediates, then compute backward pass.
/// Returns gradients w.r.t. all parameters for a single player evaluation.
/// `grad_output` is the gradient flowing back from the loss (typically from softmax+CE).
pub fn diff_eval_backward(
    player: &PlayerState,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    card_lookup: &[Card; 256],
    round: u32,
    params: &DiffEvalParams,
    table: &DiffEvalTable,
    grad_output: f64,
) -> DiffEvalGradients {
    let w = &params.weights;
    let mut grads = DiffEvalGradients::zeros();
    let g = &mut grads.grads;

    // ── Step 1: Forward pass to get module outputs + raw features ──

    let color_out = color_wheel_value_forward(player, w);
    let sell_out = sell_card_alignment_forward(player, sell_card_display, round, w);
    let deck_out = deck_color_profile_forward(player, sell_card_display, card_lookup, w, table);
    let mat_out = material_strategy_forward(player, sell_card_display, w);
    let raw = extract_raw_features_forward(player, sell_card_display, card_lookup, table);

    // ── Step 2: Assemble MLP inputs ──

    let mut mlp_inputs = [0.0f64; MLP_INPUT_SIZE];

    // Module 1: Color Wheel Value (7 outputs) -> [0..7]
    for i in 0..7 { mlp_inputs[i] = color_out[i]; }

    // Module 2: Sell Card Alignment (5 outputs) -> [7..12]
    for i in 0..5 { mlp_inputs[7 + i] = sell_out[i]; }

    // Module 3: Deck Color Profile (9 outputs) -> [12..21]
    for i in 0..9 { mlp_inputs[12 + i] = deck_out[i]; }

    // Module 4: Material Strategy (7 outputs) -> [21..28]
    for i in 0..7 { mlp_inputs[21 + i] = mat_out[i]; }

    // Direct inputs -> [28..30]
    mlp_inputs[28] = player.cached_score as f64 / 20.0;
    mlp_inputs[29] = round as f64 / 20.0;

    // Raw features -> [30..117]
    for i in 0..87 { mlp_inputs[30 + i] = raw[i]; }

    // ── Step 3: MLP forward to cache pre-activations and hidden outputs ──

    const LEAKY_ALPHA: f64 = 0.01;

    // Hidden layer 1: MLP_INPUT_SIZE → MLP_HIDDEN_SIZE (LeakyReLU)
    let mut pre_act1 = [0.0f64; MLP_HIDDEN_SIZE];
    let mut hidden1 = [0.0f64; MLP_HIDDEN_SIZE];
    for row in 0..MLP_HIDDEN_SIZE {
        let mut sum = w[MLP_B1 + row];
        for col in 0..MLP_INPUT_SIZE {
            sum += w[MLP_W1 + row * MLP_INPUT_SIZE + col] * mlp_inputs[col];
        }
        pre_act1[row] = sum;
        hidden1[row] = if sum > 0.0 { sum } else { LEAKY_ALPHA * sum };
    }

    // Hidden layer 2: MLP_HIDDEN_SIZE → MLP_HIDDEN2_SIZE (LeakyReLU)
    let mut pre_act2 = [0.0f64; MLP_HIDDEN2_SIZE];
    let mut hidden2 = [0.0f64; MLP_HIDDEN2_SIZE];
    for row in 0..MLP_HIDDEN2_SIZE {
        let mut sum = w[MLP_B2 + row];
        for col in 0..MLP_HIDDEN_SIZE {
            sum += w[MLP_W2 + row * MLP_HIDDEN_SIZE + col] * hidden1[col];
        }
        pre_act2[row] = sum;
        hidden2[row] = if sum > 0.0 { sum } else { LEAKY_ALPHA * sum };
    }

    // ── Step 4: MLP backward ──

    // Output layer: output = W3 * hidden2 + B3
    g[MLP_B3] += grad_output;
    for i in 0..MLP_HIDDEN2_SIZE {
        g[MLP_W3 + i] += grad_output * hidden2[i];
    }

    // Backward through hidden layer 2
    let mut d_hidden2 = [0.0f64; MLP_HIDDEN2_SIZE];
    for i in 0..MLP_HIDDEN2_SIZE {
        d_hidden2[i] = grad_output * w[MLP_W3 + i];
    }

    // Backward through LeakyReLU 2
    let mut d_pre_act2 = [0.0f64; MLP_HIDDEN2_SIZE];
    for i in 0..MLP_HIDDEN2_SIZE {
        d_pre_act2[i] = if pre_act2[i] > 0.0 { d_hidden2[i] } else { LEAKY_ALPHA * d_hidden2[i] };
    }

    // Backward through W2 * hidden1 + B2
    for row in 0..MLP_HIDDEN2_SIZE {
        g[MLP_B2 + row] += d_pre_act2[row];
        for col in 0..MLP_HIDDEN_SIZE {
            g[MLP_W2 + row * MLP_HIDDEN_SIZE + col] += d_pre_act2[row] * hidden1[col];
        }
    }

    // Gradient w.r.t. hidden1
    let mut d_hidden1 = [0.0f64; MLP_HIDDEN_SIZE];
    for col in 0..MLP_HIDDEN_SIZE {
        for row in 0..MLP_HIDDEN2_SIZE {
            d_hidden1[col] += d_pre_act2[row] * w[MLP_W2 + row * MLP_HIDDEN_SIZE + col];
        }
    }

    // Backward through LeakyReLU 1
    let mut d_pre_act1 = [0.0f64; MLP_HIDDEN_SIZE];
    for i in 0..MLP_HIDDEN_SIZE {
        d_pre_act1[i] = if pre_act1[i] > 0.0 { d_hidden1[i] } else { LEAKY_ALPHA * d_hidden1[i] };
    }

    // Backward through W1 * x + B1
    for row in 0..MLP_HIDDEN_SIZE {
        g[MLP_B1 + row] += d_pre_act1[row];
        for col in 0..MLP_INPUT_SIZE {
            g[MLP_W1 + row * MLP_INPUT_SIZE + col] += d_pre_act1[row] * mlp_inputs[col];
        }
    }

    // Gradient w.r.t. mlp_inputs
    let mut d_inputs = [0.0f64; MLP_INPUT_SIZE];
    for col in 0..MLP_INPUT_SIZE {
        for row in 0..MLP_HIDDEN_SIZE {
            d_inputs[col] += d_pre_act1[row] * w[MLP_W1 + row * MLP_INPUT_SIZE + col];
        }
    }

    // ── Step 5: Extract upstream gradients for each module from d_inputs ──
    // d_inputs[0..7]   → color_wheel upstream
    // d_inputs[7..12]  → sell_card upstream
    // d_inputs[12..21] → deck_profile upstream
    // d_inputs[21..28] → material upstream
    // d_inputs[28..30] → score/20 and round/20 (no params, ignore)
    // d_inputs[30..117] → raw features (no params, ignore)

    let mut d_color: [f64; 7] = [0.0; 7];
    for i in 0..7 { d_color[i] = d_inputs[i]; }

    let mut d_sell: [f64; 5] = [0.0; 5];
    for i in 0..5 { d_sell[i] = d_inputs[7 + i]; }

    let mut d_deck: [f64; 9] = [0.0; 9];
    for i in 0..9 { d_deck[i] = d_inputs[12 + i]; }

    let mut d_mat: [f64; 7] = [0.0; 7];
    for i in 0..7 { d_mat[i] = d_inputs[21 + i]; }

    // ── Step 6: Call module backward functions ──

    color_wheel_backward(player, w, g, &d_color);
    sell_card_alignment_backward(player, sell_card_display, round, w, g, &d_sell);
    deck_color_profile_backward(player, sell_card_display, card_lookup, w, table, g, &d_deck);
    material_strategy_backward(player, sell_card_display, w, g, &d_mat);

    grads
}

// ── Module forward passes (return output arrays only, no gradient accumulation) ──

fn color_wheel_value_forward(player: &PlayerState, w: &[f64; NUM_PARAMS]) -> [f64; 7] {
    let mut out = [0.0f64; 7];

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

    let mut mix_total = 0.0;
    for (i, &(a, b)) in VALID_MIX_PAIRS.iter().enumerate() {
        let count_a = player.color_wheel.get(a) as f64;
        let count_b = player.color_wheel.get(b) as f64;
        mix_total += w[MIX_PAIR_W + i] * count_a.min(count_b);
    }
    out[3] = mix_total;

    for (tier, colors) in [&PRIMARIES[..], &SECONDARIES[..], &TERTIARIES[..]].iter().enumerate() {
        let num_distinct = colors.iter().filter(|&&c| player.color_wheel.get(c) > 0).count() as f64;
        let x = w[COVERAGE_A + tier] * num_distinct - w[COVERAGE_B + tier];
        out[4 + tier] = w[COVERAGE_W + tier] * sigmoid(x);
    }

    out
}

fn sell_card_alignment_forward(
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
        let has_mat = if player.materials.get(mat_type) > 0 { 1.0 } else { 0.0 };
        let mat_match = sigmoid(w[SELL_MAT_W + mat_type as usize] * has_mat);
        let cost = sell_card.color_cost();
        let cost_len = cost.len() as f64;
        let color_matches: f64 = cost.iter().map(|&c| (player.color_wheel.get(c) as f64).min(1.0)).sum();
        let color_ratio = if cost_len > 0.0 { color_matches / cost_len } else { 0.0 };
        let ducat_tier = match ducats { 2 => 0, 3 => 1, _ => 2 };
        let weighted_color = w[SELL_DUCAT_W + ducat_tier] * color_ratio;
        alignments[i] = w[SELL_COMBINE_W] * mat_match + w[SELL_COMBINE_W + 1] * weighted_color;
    }

    let mut sorted = [0.0f64; MAX_SELL_CARD_DISPLAY];
    sorted[..n].copy_from_slice(&alignments[..n]);
    sorted[..n].sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

    out[0] = if n > 0 { sorted[0] } else { 0.0 };
    out[1] = if n > 1 { sorted[1] } else { 0.0 };
    out[2] = if n > 2 { sorted[2..n].iter().sum() } else { 0.0 };

    let round_f = round as f64;
    out[3] = sigmoid(w[SELL_ROUND_W] * round_f - w[SELL_ROUND_W + 1]);

    let sold_count = player.completed_sell_cards.len() as f64;
    out[4] = w[SELL_SOLD_W] * sold_count * sigmoid(w[SELL_SOLD_W + 1] * round_f - w[SELL_SOLD_W + 2]);

    out
}

fn deck_color_profile_forward(
    player: &PlayerState,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    card_lookup: &[Card; 256],
    w: &[f64; NUM_PARAMS],
    table: &DiffEvalTable,
) -> [f64; 9] {
    let mut out = [0.0f64; 9];

    let mut production = [0u32; NUM_COLORS];
    let mut card_count = 0u32;
    let mut workshopped_count = 0u32;
    let mut action_counts = [0u32; 5];
    let mut mat_card_counts = [0u32; 3];
    let card_sets = [&player.deck, &player.discard, &player.workshop_cards, &player.workshopped_cards, &player.drafted_cards];
    for (set_idx, cards) in card_sets.iter().enumerate() {
        for id in cards.iter() {
            let card = card_lookup[id as usize];
            let card_idx = card as usize;
            card_count += 1;
            if set_idx == 3 { workshopped_count += 1; }
            for c in 0..NUM_COLORS { production[c] += table.color_production[card_idx][c] as u32; }
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

    let mut distinct_colors = 0u32;
    let mut tier_sums = [0.0f64; 3];
    for c in 0..NUM_COLORS {
        let count = production[c] as f64;
        if count > 0.0 { distinct_colors += 1; }
        let tier = color_tier(Color::from_index(c));
        let sat_w = w[DECK_COLOR_SAT_W + tier];
        let sat_a = w[DECK_COLOR_SAT_A + tier];
        tier_sums[tier] += sat_w * (1.0 + sat_a * count).ln();
    }
    out[0] = tier_sums[0];
    out[1] = tier_sums[1];
    out[2] = tier_sums[2];

    let mut prod_need_total = 0.0;
    for bi in sell_card_display.iter() {
        let cost = bi.sell_card.color_cost();
        let producible = cost.iter().filter(|&&c| production[c.index()] > 0).count() as f64;
        let cost_len = cost.len() as f64;
        let fraction = if cost_len > 0.0 { producible / cost_len } else { 0.0 };
        let ducat_tier = match bi.sell_card.ducats() { 2 => 0, 3 => 1, _ => 2 };
        prod_need_total += w[DECK_PROD_NEED_W + ducat_tier] * fraction;
    }
    out[3] = prod_need_total;

    let mut action_total = 0.0;
    for i in 0..5 { action_total += w[DECK_ACTION_W + i] * action_counts[i] as f64; }
    out[4] = action_total;

    let mut mat_card_total = 0.0;
    for i in 0..3 { mat_card_total += w[DECK_MAT_CARD_W + i] * mat_card_counts[i] as f64; }
    out[5] = mat_card_total;

    let size = card_count as f64;
    out[6] = w[DECK_SIZE_W] * size + w[DECK_SIZE_W + 1] * size * size;

    out[7] = w[DECK_DIVERSITY_W] * distinct_colors as f64 / 12.0;

    out[8] = w[DECK_WORKSHOP_W] * workshopped_count as f64;

    out
}

fn material_strategy_forward(
    player: &PlayerState,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    w: &[f64; NUM_PARAMS],
) -> [f64; 7] {
    let mut out = [0.0f64; 7];

    let mut types_with_material = 0u32;
    for i in 0..3 {
        let stored = player.materials.counts[i] as f64;
        let x = w[MAT_SUFF_W + i] * (stored - w[MAT_SUFF_THRESH + i]);
        out[i] = sigmoid(x);

        if stored > 0.0 { types_with_material += 1; }

        let mut demand = 0.0;
        for bi in sell_card_display.iter() {
            if bi.sell_card.required_material() as usize == i {
                demand += bi.sell_card.ducats() as f64;
            }
        }
        let availability = sigmoid(stored - 0.5);
        out[3 + i] = w[MAT_DEMAND_W + i] * demand * availability;
    }

    let mut diversity = 0.0;
    if types_with_material >= 2 { diversity += w[MAT_DIVERSITY_W]; }
    if types_with_material >= 3 { diversity += w[MAT_DIVERSITY_W + 1]; }
    out[6] = diversity;

    out
}

fn extract_raw_features_forward(
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

    // Raw deck color production [12..24] and card type counts [36..82]
    let mut production = [0u32; NUM_COLORS];
    let mut card_type_counts = [0u32; 46];

    let card_sets: [&crate::unordered_cards::UnorderedCards; 5] = [
        &player.deck, &player.discard, &player.workshop_cards, &player.workshopped_cards, &player.drafted_cards,
    ];

    for cards in card_sets.iter() {
        for id in cards.iter() {
            let card = card_lookup[id as usize];
            let card_idx = card as usize;
            for c in 0..NUM_COLORS {
                production[c] += table.color_production[card_idx][c] as u32;
            }
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

// ── Module backward passes ──

/// Color wheel backward. upstream is [f64; 7]:
/// [0..3] per-tier saturation, [3] mix-pair total, [4..7] per-tier coverage
fn color_wheel_backward(player: &PlayerState, w: &[f64; NUM_PARAMS], g: &mut [f64; NUM_PARAMS], upstream: &[f64; 7]) {
    // Per-tier saturation: out[tier] = sum_c sat_w * ln(1 + sat_a * count)
    for (tier, colors) in [&PRIMARIES[..], &SECONDARIES[..], &TERTIARIES[..]].iter().enumerate() {
        let sat_w = w[COLOR_SAT_W + tier];
        let sat_a = w[COLOR_SAT_A + tier];
        for &c in *colors {
            let count = player.color_wheel.get(c) as f64;
            let inner = 1.0 + sat_a * count;
            g[COLOR_SAT_W + tier] += upstream[tier] * inner.ln();
            g[COLOR_SAT_A + tier] += upstream[tier] * sat_w * count / inner;
        }
    }

    // Mix-pair: out[3] = sum_i mix_w[i] * min(a, b)
    for (i, &(a, b)) in VALID_MIX_PAIRS.iter().enumerate() {
        let count_a = player.color_wheel.get(a) as f64;
        let count_b = player.color_wheel.get(b) as f64;
        g[MIX_PAIR_W + i] += upstream[3] * count_a.min(count_b);
    }

    // Coverage: out[4+tier] = cov_w * sigmoid(cov_a * n - cov_b)
    for (tier, colors) in [&PRIMARIES[..], &SECONDARIES[..], &TERTIARIES[..]].iter().enumerate() {
        let num_distinct = colors.iter().filter(|&&c| player.color_wheel.get(c) > 0).count() as f64;
        let x = w[COVERAGE_A + tier] * num_distinct - w[COVERAGE_B + tier];
        let sig = sigmoid(x);
        let sig_deriv = sig * (1.0 - sig);

        g[COVERAGE_W + tier] += upstream[4 + tier] * sig;
        g[COVERAGE_A + tier] += upstream[4 + tier] * w[COVERAGE_W + tier] * sig_deriv * num_distinct;
        g[COVERAGE_B + tier] += upstream[4 + tier] * w[COVERAGE_W + tier] * sig_deriv * (-1.0);
    }
}

/// Sell card alignment backward. upstream is [f64; 5]:
/// [0] best, [1] second, [2] rest, [3] urgency, [4] sold_value
fn sell_card_alignment_backward(
    player: &PlayerState,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    round: u32,
    w: &[f64; NUM_PARAMS],
    g: &mut [f64; NUM_PARAMS],
    upstream: &[f64; 5],
) {
    let n = sell_card_display.len();
    let round_f = round as f64;

    // Recompute per-card alignments
    let mut alignments = [0.0f64; MAX_SELL_CARD_DISPLAY];
    let mut mat_matches = [0.0f64; MAX_SELL_CARD_DISPLAY];
    let mut color_ratios = [0.0f64; MAX_SELL_CARD_DISPLAY];
    let mut ducat_tiers = [0usize; MAX_SELL_CARD_DISPLAY];
    let mut mat_types = [0usize; MAX_SELL_CARD_DISPLAY];
    let mut has_mats = [0.0f64; MAX_SELL_CARD_DISPLAY];

    for (i, bi) in sell_card_display.iter().enumerate() {
        let sell_card = bi.sell_card;
        let ducats = sell_card.ducats();
        let mat_type = sell_card.required_material();
        mat_types[i] = mat_type as usize;
        let has_mat = if player.materials.get(mat_type) > 0 { 1.0 } else { 0.0 };
        has_mats[i] = has_mat;
        mat_matches[i] = sigmoid(w[SELL_MAT_W + mat_type as usize] * has_mat);
        let cost = sell_card.color_cost();
        let cost_len = cost.len() as f64;
        let color_match_sum: f64 = cost.iter().map(|&c| (player.color_wheel.get(c) as f64).min(1.0)).sum();
        color_ratios[i] = if cost_len > 0.0 { color_match_sum / cost_len } else { 0.0 };
        ducat_tiers[i] = match ducats { 2 => 0, 3 => 1, _ => 2 };
        let weighted_color = w[SELL_DUCAT_W + ducat_tiers[i]] * color_ratios[i];
        alignments[i] = w[SELL_COMBINE_W] * mat_matches[i] + w[SELL_COMBINE_W + 1] * weighted_color;
    }

    // Sort and track which original indices correspond to best/second/rest
    let mut sorted_indices: Vec<usize> = (0..n).collect();
    sorted_indices.sort_by(|&a, &b| alignments[b].partial_cmp(&alignments[a]).unwrap_or(std::cmp::Ordering::Equal));

    let best_idx_opt = sorted_indices.first().copied();
    let second_idx_opt = sorted_indices.get(1).copied();

    // Upstream for out[0] = best alignment, out[1] = second, out[2] = rest
    // d_alignment[i] depends on position in sorted order
    let mut d_alignments = [0.0f64; MAX_SELL_CARD_DISPLAY];
    if let Some(i) = best_idx_opt { d_alignments[i] += upstream[0]; }
    if let Some(i) = second_idx_opt { d_alignments[i] += upstream[1]; }
    for &i in sorted_indices.iter().skip(2) { d_alignments[i] += upstream[2]; }

    // Backward through per-card alignment computation
    for i in 0..n {
        let d_a = d_alignments[i];
        if d_a == 0.0 { continue; }

        let weighted_color = w[SELL_DUCAT_W + ducat_tiers[i]] * color_ratios[i];

        g[SELL_COMBINE_W] += d_a * mat_matches[i];
        g[SELL_COMBINE_W + 1] += d_a * weighted_color;

        g[SELL_DUCAT_W + ducat_tiers[i]] += d_a * w[SELL_COMBINE_W + 1] * color_ratios[i];

        let sig = mat_matches[i];
        let sig_deriv = sig * (1.0 - sig);
        g[SELL_MAT_W + mat_types[i]] += d_a * w[SELL_COMBINE_W] * sig_deriv * has_mats[i];
    }

    // Backward through urgency: out[3] = sigmoid(w_round * round - b_round)
    let urgency_arg = w[SELL_ROUND_W] * round_f - w[SELL_ROUND_W + 1];
    let urgency = sigmoid(urgency_arg);
    let urgency_deriv = urgency * (1.0 - urgency);

    g[SELL_ROUND_W] += upstream[3] * urgency_deriv * round_f;
    g[SELL_ROUND_W + 1] += upstream[3] * urgency_deriv * (-1.0);

    // Backward through sold_value: out[4] = w_sold * sold_count * sigmoid(a_sold * round - b_sold)
    let sold_count = player.completed_sell_cards.len() as f64;
    let sold_arg = w[SELL_SOLD_W + 1] * round_f - w[SELL_SOLD_W + 2];
    let sold_sig = sigmoid(sold_arg);
    let sold_sig_deriv = sold_sig * (1.0 - sold_sig);

    g[SELL_SOLD_W] += upstream[4] * sold_count * sold_sig;
    g[SELL_SOLD_W + 1] += upstream[4] * w[SELL_SOLD_W] * sold_count * sold_sig_deriv * round_f;
    g[SELL_SOLD_W + 2] += upstream[4] * w[SELL_SOLD_W] * sold_count * sold_sig_deriv * (-1.0);
}

/// Deck color profile backward. upstream is [f64; 9]:
/// [0-2] per-tier saturation, [3] prod-need, [4] action, [5] mat-card,
/// [6] deck-size, [7] diversity, [8] workshopped
fn deck_color_profile_backward(
    player: &PlayerState,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    card_lookup: &[Card; 256],
    w: &[f64; NUM_PARAMS],
    table: &DiffEvalTable,
    g: &mut [f64; NUM_PARAMS],
    upstream: &[f64; 9],
) {
    // Recompute intermediates
    let mut production = [0u32; NUM_COLORS];
    let mut card_count = 0u32;
    let mut workshopped_count = 0u32;
    let mut action_counts = [0u32; 5];
    let mut mat_card_counts = [0u32; 3];
    let card_sets = [&player.deck, &player.discard, &player.workshop_cards, &player.workshopped_cards, &player.drafted_cards];
    for (set_idx, cards) in card_sets.iter().enumerate() {
        for id in cards.iter() {
            let card = card_lookup[id as usize];
            let card_idx = card as usize;
            card_count += 1;
            if set_idx == 3 { workshopped_count += 1; }
            for c in 0..NUM_COLORS { production[c] += table.color_production[card_idx][c] as u32; }
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

    // Per-color saturation gradients (grouped by tier)
    let mut distinct_colors = 0u32;
    for c in 0..NUM_COLORS {
        let count = production[c] as f64;
        if count > 0.0 { distinct_colors += 1; }
        let tier = color_tier(Color::from_index(c));
        let sat_w = w[DECK_COLOR_SAT_W + tier];
        let sat_a = w[DECK_COLOR_SAT_A + tier];
        let inner = 1.0 + sat_a * count;
        g[DECK_COLOR_SAT_W + tier] += upstream[tier] * inner.ln();
        g[DECK_COLOR_SAT_A + tier] += upstream[tier] * sat_w * count / inner;
    }

    // Production-need interaction
    for bi in sell_card_display.iter() {
        let cost = bi.sell_card.color_cost();
        let producible = cost.iter().filter(|&&c| production[c.index()] > 0).count() as f64;
        let cost_len = cost.len() as f64;
        let fraction = if cost_len > 0.0 { producible / cost_len } else { 0.0 };
        let ducat_tier = match bi.sell_card.ducats() { 2 => 0, 3 => 1, _ => 2 };
        g[DECK_PROD_NEED_W + ducat_tier] += upstream[3] * fraction;
    }

    // Action card weights
    for i in 0..5 { g[DECK_ACTION_W + i] += upstream[4] * action_counts[i] as f64; }

    // Material card weights
    for i in 0..3 { g[DECK_MAT_CARD_W + i] += upstream[5] * mat_card_counts[i] as f64; }

    // Deck size
    let size = card_count as f64;
    g[DECK_SIZE_W] += upstream[6] * size;
    g[DECK_SIZE_W + 1] += upstream[6] * size * size;

    // Diversity
    g[DECK_DIVERSITY_W] += upstream[7] * distinct_colors as f64 / 12.0;

    // Workshopped
    g[DECK_WORKSHOP_W] += upstream[8] * workshopped_count as f64;
}

/// Material strategy backward. upstream is [f64; 7]:
/// [0-2] per-type sufficiency, [3-5] per-type demand, [6] diversity
fn material_strategy_backward(
    player: &PlayerState,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    w: &[f64; NUM_PARAMS],
    g: &mut [f64; NUM_PARAMS],
    upstream: &[f64; 7],
) {
    let mut types_with_material = 0u32;
    for i in 0..3 {
        let stored = player.materials.counts[i] as f64;
        let x = w[MAT_SUFF_W + i] * (stored - w[MAT_SUFF_THRESH + i]);
        let sig = sigmoid(x);
        let sig_deriv = sig * (1.0 - sig);

        // out[i] = sigmoid(suff_w * (stored - thresh))
        g[MAT_SUFF_W + i] += upstream[i] * sig_deriv * (stored - w[MAT_SUFF_THRESH + i]);
        g[MAT_SUFF_THRESH + i] += upstream[i] * sig_deriv * (-w[MAT_SUFF_W + i]);

        if stored > 0.0 { types_with_material += 1; }

        // out[3+i] = demand_w * demand * availability
        let mut demand = 0.0;
        for bi in sell_card_display.iter() {
            if bi.sell_card.required_material() as usize == i {
                demand += bi.sell_card.ducats() as f64;
            }
        }
        let availability = sigmoid(stored - 0.5);
        g[MAT_DEMAND_W + i] += upstream[3 + i] * demand * availability;
    }

    // Diversity (step-function-like, no smooth gradient for the condition,
    // but accumulate for the weight)
    if types_with_material >= 2 { g[MAT_DIVERSITY_W] += upstream[6]; }
    if types_with_material >= 3 { g[MAT_DIVERSITY_W + 1] += upstream[6]; }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoring::diff_eval::{DiffEvalParams, DiffEvalTable, diff_eval_score};
    use crate::unordered_cards::UnorderedCards;
    use smallvec::SmallVec;

    fn make_test_player() -> PlayerState {
        let mut p = PlayerState {
            deck: UnorderedCards::new(),
            discard: UnorderedCards::new(),
            workshopped_cards: UnorderedCards::new(),
            workshop_cards: UnorderedCards::new(),
            drafted_cards: UnorderedCards::new(),
            color_wheel: ColorWheel::new(),
            materials: Materials::new(),
            completed_sell_cards: SmallVec::new(),
            completed_glass: SmallVec::new(),
            ducats: 3,
            cached_score: 3,
        };
        p.color_wheel.set(Color::Red, 2);
        p.color_wheel.set(Color::Yellow, 1);
        p.color_wheel.set(Color::Orange, 1);
        p.materials.counts[0] = 1; // Textiles
        p
    }

    fn make_test_display() -> FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY> {
        let mut display = FixedVec::new();
        display.push(SellCardInstance { instance_id: 0, sell_card: SellCard::Textiles2Vermilion });
        display.push(SellCardInstance { instance_id: 1, sell_card: SellCard::Ceramics3VermilionRed });
        display
    }

    /// Verify hand-rolled gradients match finite-difference approximation.
    #[test]
    fn test_gradient_vs_finite_difference() {
        let params = DiffEvalParams::default();
        let table = DiffEvalTable::new(&params);
        let card_lookup = [Card::BasicRed; 256];
        let player = make_test_player();
        let display = make_test_display();
        let round = 3;

        // Compute analytical gradients
        let grads = diff_eval_backward(&player, &display, &card_lookup, round, &params, &table, 1.0);

        // Compare against finite differences.
        // Check all module params (0..72) plus a sampling of MLP params.
        let eps = 1e-5;
        let mut max_rel_error = 0.0f64;
        let mut worst_param = 0;

        let mut params_to_check: Vec<usize> = (0..72).collect();
        // Sample every 100th MLP param
        let mut i = 72;
        while i < NUM_DIFF_PARAMS {
            params_to_check.push(i);
            i += 100;
        }
        // Always include the last few params (W2 tail, B2)
        if NUM_DIFF_PARAMS > 3 {
            for j in (NUM_DIFF_PARAMS - 3)..NUM_DIFF_PARAMS {
                if !params_to_check.contains(&j) {
                    params_to_check.push(j);
                }
            }
        }

        for &i in &params_to_check {
            let mut params_plus = params.clone();
            params_plus.weights[i] += eps;
            let score_plus = diff_eval_score(&player, &display, &card_lookup, round, &params_plus, &table);

            let mut params_minus = params.clone();
            params_minus.weights[i] -= eps;
            let score_minus = diff_eval_score(&player, &display, &card_lookup, round, &params_minus, &table);

            let fd_grad = (score_plus - score_minus) / (2.0 * eps);
            let analytical_grad = grads.grads[i];

            let abs_diff = (fd_grad - analytical_grad).abs();
            let denom = fd_grad.abs().max(analytical_grad.abs()).max(1e-8);
            let rel_error = abs_diff / denom;

            if rel_error > max_rel_error {
                max_rel_error = rel_error;
                worst_param = i;
            }

            // Allow small absolute errors for near-zero gradients
            if abs_diff > 1e-4 {
                assert!(
                    rel_error < 0.01,
                    "Gradient mismatch at param {}: analytical={:.8}, fd={:.8}, rel_error={:.6}",
                    i, analytical_grad, fd_grad, rel_error
                );
            }
        }

        eprintln!("Checked {} params. Max relative gradient error: {:.6} at param {}", params_to_check.len(), max_rel_error, worst_param);
        assert!(max_rel_error < 0.01, "Maximum relative error {:.6} exceeds 1%", max_rel_error);
    }
}
