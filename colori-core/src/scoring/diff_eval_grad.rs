use crate::colors::{PRIMARIES, SECONDARIES, TERTIARIES, VALID_MIX_PAIRS};
use crate::fixed_vec::FixedVec;
use crate::types::*;

use super::diff_eval::*;

/// Accumulated gradients, same shape as DiffEvalParams.
#[derive(Debug, Clone)]
pub struct DiffEvalGradients {
    pub grads: [f64; NUM_PARAMS],
}

impl DiffEvalGradients {
    pub fn zeros() -> Self {
        DiffEvalGradients { grads: [0.0; NUM_PARAMS] }
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

    // ── Forward pass with caching ──

    // Module 1: Color wheel value
    let color_value = color_wheel_value_cached(player, w, g, grad_output);

    // Module 2: Sell card alignment
    let sell_align = sell_card_alignment_cached(player, sell_card_display, round, w, g, grad_output);

    // Module 3: Deck color profile
    let deck_profile = deck_color_profile_cached(player, sell_card_display, card_lookup, w, table, g, grad_output);

    // Module 4: Material strategy
    let material = material_strategy_cached(player, sell_card_display, w, g, grad_output);

    // ── Aggregation MLP forward + backward ──
    let mlp_inputs = [
        player.cached_score as f64 / 20.0,
        color_value,
        sell_align,
        deck_profile,
        material,
        round as f64 / 20.0,
    ];

    // Forward: hidden = ReLU(W1 * x + b1)
    let mut pre_relu = [0.0f64; 16];
    let mut hidden = [0.0f64; 16];
    for row in 0..16 {
        let mut sum = w[MLP_B1 + row];
        for col in 0..6 {
            sum += w[MLP_W1 + row * 6 + col] * mlp_inputs[col];
        }
        pre_relu[row] = sum;
        hidden[row] = sum.max(0.0);
    }

    // Forward: output = W2 * hidden + b2
    // (output value not needed, we already have grad_output)

    // Backward through output layer: d_output/d_W2[i] = hidden[i], d_output/d_b2 = 1
    g[MLP_B2] += grad_output;
    for i in 0..16 {
        g[MLP_W2 + i] += grad_output * hidden[i];
    }

    // Backward through hidden layer
    let mut d_hidden = [0.0f64; 16];
    for i in 0..16 {
        d_hidden[i] = grad_output * w[MLP_W2 + i];
    }

    // Backward through ReLU
    let mut d_pre_relu = [0.0f64; 16];
    for i in 0..16 {
        d_pre_relu[i] = if pre_relu[i] > 0.0 { d_hidden[i] } else { 0.0 };
    }

    // Backward through W1 * x + b1
    for row in 0..16 {
        g[MLP_B1 + row] += d_pre_relu[row];
        for col in 0..6 {
            g[MLP_W1 + row * 6 + col] += d_pre_relu[row] * mlp_inputs[col];
        }
    }

    // Gradient w.r.t. mlp_inputs (to propagate to modules)
    let mut d_inputs = [0.0f64; 6];
    for col in 0..6 {
        for row in 0..16 {
            d_inputs[col] += d_pre_relu[row] * w[MLP_W1 + row * 6 + col];
        }
    }

    // Now we need to propagate d_inputs back through the modules.
    // d_inputs[0] = d_loss/d_(score/20) — no learnable params, skip
    // d_inputs[1] = d_loss/d_color_value — already accumulated in color_wheel_value_cached
    // d_inputs[2] = d_loss/d_sell_align — already accumulated
    // d_inputs[3] = d_loss/d_deck_profile — already accumulated
    // d_inputs[4] = d_loss/d_material — already accumulated
    // d_inputs[5] = d_loss/d_(round/20) — no learnable params, skip

    // Wait — the module gradient functions above used grad_output directly,
    // but they should use d_inputs[module_idx] * grad_output is wrong.
    // Actually, we need to multiply the module gradients by the MLP input gradient.
    //
    // Let me restructure: the module functions should compute their output AND
    // accumulate gradients scaled by the upstream gradient (d_inputs[i]).
    //
    // Since I already accumulated with grad_output, I need to rescale.
    // The correct upstream gradient for each module is d_inputs[i], not grad_output.
    //
    // Let me fix this by using a two-pass approach:
    // 1. Forward: compute module outputs (no gradient accumulation)
    // 2. MLP forward + backward: compute d_inputs
    // 3. Backward: accumulate module gradients using d_inputs

    // Reset module gradients (we incorrectly accumulated them with grad_output)
    // Actually, let me restructure to not accumulate in the forward pass at all.

    // Re-zero the module parameter gradients (indices 0..72)
    for i in 0..72 {
        g[i] = 0.0;
    }

    // Now properly accumulate with correct upstream gradients
    color_wheel_backward(player, w, g, d_inputs[1]);
    sell_card_alignment_backward(player, sell_card_display, round, w, g, d_inputs[2]);
    deck_color_profile_backward(player, sell_card_display, card_lookup, w, table, g, d_inputs[3]);
    material_strategy_backward(player, sell_card_display, w, g, d_inputs[4]);

    grads
}

// ── Module forward passes (return output only) ──

fn color_wheel_value_cached(player: &PlayerState, w: &[f64; NUM_PARAMS], _g: &mut [f64; NUM_PARAMS], _grad: f64) -> f64 {
    let mut value = 0.0;
    for (tier, colors) in [&PRIMARIES[..], &SECONDARIES[..], &TERTIARIES[..]].iter().enumerate() {
        let sat_w = w[COLOR_SAT_W + tier];
        let sat_a = w[COLOR_SAT_A + tier];
        for &c in *colors {
            let count = player.color_wheel.get(c) as f64;
            value += sat_w * (1.0 + sat_a * count).ln();
        }
    }
    for (i, &(a, b)) in VALID_MIX_PAIRS.iter().enumerate() {
        let count_a = player.color_wheel.get(a) as f64;
        let count_b = player.color_wheel.get(b) as f64;
        value += w[MIX_PAIR_W + i] * count_a.min(count_b);
    }
    for (tier, colors) in [&PRIMARIES[..], &SECONDARIES[..], &TERTIARIES[..]].iter().enumerate() {
        let num_distinct = colors.iter().filter(|&&c| player.color_wheel.get(c) > 0).count() as f64;
        let x = w[COVERAGE_A + tier] * num_distinct - w[COVERAGE_B + tier];
        value += w[COVERAGE_W + tier] * sigmoid(x);
    }
    value
}

fn sell_card_alignment_cached(
    player: &PlayerState,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    round: u32,
    w: &[f64; NUM_PARAMS],
    _g: &mut [f64; NUM_PARAMS],
    _grad: f64,
) -> f64 {
    // Same as forward pass in diff_eval.rs
    let n = sell_card_display.len();
    let mut alignments = [0.0f64; MAX_SELL_CARD_DISPLAY];
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
    let best = if n > 0 { sorted[0] } else { 0.0 };
    let second = if n > 1 { sorted[1] } else { 0.0 };
    let rest: f64 = if n > 2 { sorted[2..n].iter().sum() } else { 0.0 };
    let aggregated = w[SELL_AGG_W] * best + w[SELL_AGG_W + 1] * second + w[SELL_AGG_W + 2] * rest;
    let round_f = round as f64;
    let urgency = sigmoid(w[SELL_ROUND_W] * round_f - w[SELL_ROUND_W + 1]);
    let alignment_value = aggregated * urgency;
    let sold_count = player.completed_sell_cards.len() as f64;
    let sold_value = w[SELL_SOLD_W] * sold_count * sigmoid(w[SELL_SOLD_W + 1] * round_f - w[SELL_SOLD_W + 2]);
    alignment_value + sold_value
}

fn deck_color_profile_cached(
    player: &PlayerState,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    card_lookup: &[Card; 256],
    w: &[f64; NUM_PARAMS],
    table: &DiffEvalTable,
    _g: &mut [f64; NUM_PARAMS],
    _grad: f64,
) -> f64 {
    let mut value = 0.0;
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
                _ => {}
            }
        }
    }
    let mut distinct_colors = 0u32;
    for c in 0..NUM_COLORS {
        let count = production[c] as f64;
        if count > 0.0 { distinct_colors += 1; }
        let tier = color_tier(Color::from_index(c));
        value += w[DECK_COLOR_SAT_W + tier] * (1.0 + w[DECK_COLOR_SAT_A + tier] * count).ln();
    }
    for bi in sell_card_display.iter() {
        let cost = bi.sell_card.color_cost();
        let producible = cost.iter().filter(|&&c| production[c.index()] > 0).count() as f64;
        let cost_len = cost.len() as f64;
        let fraction = if cost_len > 0.0 { producible / cost_len } else { 0.0 };
        let ducat_tier = match bi.sell_card.ducats() { 2 => 0, 3 => 1, _ => 2 };
        value += w[DECK_PROD_NEED_W + ducat_tier] * fraction;
    }
    for i in 0..5 { value += w[DECK_ACTION_W + i] * action_counts[i] as f64; }
    for i in 0..3 { value += w[DECK_MAT_CARD_W + i] * mat_card_counts[i] as f64; }
    let size = card_count as f64;
    value += w[DECK_SIZE_W] * size + w[DECK_SIZE_W + 1] * size * size;
    value += w[DECK_DIVERSITY_W] * distinct_colors as f64 / 12.0;
    value += w[DECK_WORKSHOP_W] * workshopped_count as f64;
    value
}

fn material_strategy_cached(
    player: &PlayerState,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    w: &[f64; NUM_PARAMS],
    _g: &mut [f64; NUM_PARAMS],
    _grad: f64,
) -> f64 {
    let mut value = 0.0;
    let mut types_with_material = 0u32;
    for i in 0..3 {
        let stored = player.materials.counts[i] as f64;
        let x = w[MAT_SUFF_W + i] * (stored - w[MAT_SUFF_THRESH + i]);
        value += sigmoid(x);
        if stored > 0.0 { types_with_material += 1; }
        let mut demand = 0.0;
        for bi in sell_card_display.iter() {
            if bi.sell_card.required_material() as usize == i {
                demand += bi.sell_card.ducats() as f64;
            }
        }
        let availability = sigmoid(stored - 0.5);
        value += w[MAT_DEMAND_W + i] * demand * availability;
    }
    if types_with_material >= 2 { value += w[MAT_DIVERSITY_W]; }
    if types_with_material >= 3 { value += w[MAT_DIVERSITY_W + 1]; }
    value
}

// ── Module backward passes ──

fn color_wheel_backward(player: &PlayerState, w: &[f64; NUM_PARAMS], g: &mut [f64; NUM_PARAMS], upstream: f64) {
    // d/d(sat_w) [sat_w * ln(1 + sat_a * count)] = ln(1 + sat_a * count)
    // d/d(sat_a) [sat_w * ln(1 + sat_a * count)] = sat_w * count / (1 + sat_a * count)
    for (tier, colors) in [&PRIMARIES[..], &SECONDARIES[..], &TERTIARIES[..]].iter().enumerate() {
        let sat_w = w[COLOR_SAT_W + tier];
        let sat_a = w[COLOR_SAT_A + tier];
        for &c in *colors {
            let count = player.color_wheel.get(c) as f64;
            let inner = 1.0 + sat_a * count;
            g[COLOR_SAT_W + tier] += upstream * inner.ln();
            g[COLOR_SAT_A + tier] += upstream * sat_w * count / inner;
        }
    }

    // d/d(mix_w[i]) [mix_w[i] * min(a, b)] = min(a, b)
    for (i, &(a, b)) in VALID_MIX_PAIRS.iter().enumerate() {
        let count_a = player.color_wheel.get(a) as f64;
        let count_b = player.color_wheel.get(b) as f64;
        g[MIX_PAIR_W + i] += upstream * count_a.min(count_b);
    }

    // Coverage: d/d(cov_w) [cov_w * sigmoid(cov_a * n - cov_b)] = sigmoid(...)
    // d/d(cov_a) = cov_w * sig' * n
    // d/d(cov_b) = cov_w * sig' * (-1)
    for (tier, colors) in [&PRIMARIES[..], &SECONDARIES[..], &TERTIARIES[..]].iter().enumerate() {
        let num_distinct = colors.iter().filter(|&&c| player.color_wheel.get(c) > 0).count() as f64;
        let x = w[COVERAGE_A + tier] * num_distinct - w[COVERAGE_B + tier];
        let sig = sigmoid(x);
        let sig_deriv = sig * (1.0 - sig);

        g[COVERAGE_W + tier] += upstream * sig;
        g[COVERAGE_A + tier] += upstream * w[COVERAGE_W + tier] * sig_deriv * num_distinct;
        g[COVERAGE_B + tier] += upstream * w[COVERAGE_W + tier] * sig_deriv * (-1.0);
    }
}

fn sell_card_alignment_backward(
    player: &PlayerState,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    round: u32,
    w: &[f64; NUM_PARAMS],
    g: &mut [f64; NUM_PARAMS],
    upstream: f64,
) {
    let n = sell_card_display.len();
    let round_f = round as f64;

    // Recompute per-card alignments (needed for backward)
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

    // Forward recompute
    let best = best_idx_opt.map_or(0.0, |i| alignments[i]);
    let second = second_idx_opt.map_or(0.0, |i| alignments[i]);
    let rest: f64 = sorted_indices.iter().skip(2).map(|&i| alignments[i]).sum();
    let aggregated = w[SELL_AGG_W] * best + w[SELL_AGG_W + 1] * second + w[SELL_AGG_W + 2] * rest;

    let urgency_arg = w[SELL_ROUND_W] * round_f - w[SELL_ROUND_W + 1];
    let urgency = sigmoid(urgency_arg);
    let urgency_deriv = urgency * (1.0 - urgency);

    // Backward through alignment_value = aggregated * urgency
    let d_aggregated = upstream * urgency;
    let d_urgency = upstream * aggregated;

    // Backward through urgency = sigmoid(w_round * round - b_round)
    g[SELL_ROUND_W] += d_urgency * urgency_deriv * round_f;
    g[SELL_ROUND_W + 1] += d_urgency * urgency_deriv * (-1.0);

    // Backward through aggregation weights
    g[SELL_AGG_W] += d_aggregated * best;
    g[SELL_AGG_W + 1] += d_aggregated * second;
    g[SELL_AGG_W + 2] += d_aggregated * rest;

    // d_alignment[i] depends on position in sorted order
    let mut d_alignments = [0.0f64; MAX_SELL_CARD_DISPLAY];
    if let Some(i) = best_idx_opt { d_alignments[i] += d_aggregated * w[SELL_AGG_W]; }
    if let Some(i) = second_idx_opt { d_alignments[i] += d_aggregated * w[SELL_AGG_W + 1]; }
    for &i in sorted_indices.iter().skip(2) { d_alignments[i] += d_aggregated * w[SELL_AGG_W + 2]; }

    // Backward through per-card alignment computation
    for i in 0..n {
        let d_a = d_alignments[i];
        if d_a == 0.0 { continue; }

        // alignment[i] = w_combine * mat_match + w_color * weighted_color
        // where weighted_color = w_ducat[tier] * color_ratio
        let weighted_color = w[SELL_DUCAT_W + ducat_tiers[i]] * color_ratios[i];

        g[SELL_COMBINE_W] += d_a * mat_matches[i];
        g[SELL_COMBINE_W + 1] += d_a * weighted_color;

        // d/d(w_ducat[tier])
        g[SELL_DUCAT_W + ducat_tiers[i]] += d_a * w[SELL_COMBINE_W + 1] * color_ratios[i];

        // d/d(sell_mat_w[mt]) through sigmoid
        let sig = mat_matches[i];
        let sig_deriv = sig * (1.0 - sig);
        g[SELL_MAT_W + mat_types[i]] += d_a * w[SELL_COMBINE_W] * sig_deriv * has_mats[i];
    }

    // Backward through sold_value = w_sold * sold_count * sigmoid(a_sold * round - b_sold)
    let sold_count = player.completed_sell_cards.len() as f64;
    let sold_arg = w[SELL_SOLD_W + 1] * round_f - w[SELL_SOLD_W + 2];
    let sold_sig = sigmoid(sold_arg);
    let sold_sig_deriv = sold_sig * (1.0 - sold_sig);

    g[SELL_SOLD_W] += upstream * sold_count * sold_sig;
    g[SELL_SOLD_W + 1] += upstream * w[SELL_SOLD_W] * sold_count * sold_sig_deriv * round_f;
    g[SELL_SOLD_W + 2] += upstream * w[SELL_SOLD_W] * sold_count * sold_sig_deriv * (-1.0);
}

fn deck_color_profile_backward(
    player: &PlayerState,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    card_lookup: &[Card; 256],
    w: &[f64; NUM_PARAMS],
    table: &DiffEvalTable,
    g: &mut [f64; NUM_PARAMS],
    upstream: f64,
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
                _ => {}
            }
        }
    }

    // Gradients for per-color log-saturation
    let mut distinct_colors = 0u32;
    for c in 0..NUM_COLORS {
        let count = production[c] as f64;
        if count > 0.0 { distinct_colors += 1; }
        let tier = color_tier(Color::from_index(c));
        let sat_w = w[DECK_COLOR_SAT_W + tier];
        let sat_a = w[DECK_COLOR_SAT_A + tier];
        let inner = 1.0 + sat_a * count;
        g[DECK_COLOR_SAT_W + tier] += upstream * inner.ln();
        g[DECK_COLOR_SAT_A + tier] += upstream * sat_w * count / inner;
    }

    // Gradients for production-need interaction (linear in weights)
    for bi in sell_card_display.iter() {
        let cost = bi.sell_card.color_cost();
        let producible = cost.iter().filter(|&&c| production[c.index()] > 0).count() as f64;
        let cost_len = cost.len() as f64;
        let fraction = if cost_len > 0.0 { producible / cost_len } else { 0.0 };
        let ducat_tier = match bi.sell_card.ducats() { 2 => 0, 3 => 1, _ => 2 };
        g[DECK_PROD_NEED_W + ducat_tier] += upstream * fraction;
    }

    // Linear terms
    for i in 0..5 { g[DECK_ACTION_W + i] += upstream * action_counts[i] as f64; }
    for i in 0..3 { g[DECK_MAT_CARD_W + i] += upstream * mat_card_counts[i] as f64; }
    let size = card_count as f64;
    g[DECK_SIZE_W] += upstream * size;
    g[DECK_SIZE_W + 1] += upstream * size * size;
    g[DECK_DIVERSITY_W] += upstream * distinct_colors as f64 / 12.0;
    g[DECK_WORKSHOP_W] += upstream * workshopped_count as f64;
}

fn material_strategy_backward(
    player: &PlayerState,
    sell_card_display: &FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY>,
    w: &[f64; NUM_PARAMS],
    g: &mut [f64; NUM_PARAMS],
    upstream: f64,
) {
    let mut types_with_material = 0u32;
    for i in 0..3 {
        let stored = player.materials.counts[i] as f64;
        let x = w[MAT_SUFF_W + i] * (stored - w[MAT_SUFF_THRESH + i]);
        let sig = sigmoid(x);
        let sig_deriv = sig * (1.0 - sig);

        // d/d(suff_w[i]) = sig' * (stored - thresh)
        g[MAT_SUFF_W + i] += upstream * sig_deriv * (stored - w[MAT_SUFF_THRESH + i]);
        // d/d(thresh[i]) = sig' * (-suff_w)
        g[MAT_SUFF_THRESH + i] += upstream * sig_deriv * (-w[MAT_SUFF_W + i]);

        if stored > 0.0 { types_with_material += 1; }

        // demand * availability interaction
        let mut demand = 0.0;
        for bi in sell_card_display.iter() {
            if bi.sell_card.required_material() as usize == i {
                demand += bi.sell_card.ducats() as f64;
            }
        }
        let availability = sigmoid(stored - 0.5);
        // d/d(demand_w[i]) = demand * availability
        g[MAT_DEMAND_W + i] += upstream * demand * availability;
        // (availability has no learnable params — stored and 0.5 are fixed)
    }

    // Diversity (step-function-like, no smooth gradient, but accumulate for the weight)
    if types_with_material >= 2 { g[MAT_DIVERSITY_W] += upstream; }
    if types_with_material >= 3 { g[MAT_DIVERSITY_W + 1] += upstream; }
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
        let table = DiffEvalTable::new();
        let card_lookup = [Card::BasicRed; 256];
        let player = make_test_player();
        let display = make_test_display();
        let round = 3;

        // Compute analytical gradients
        let grads = diff_eval_backward(&player, &display, &card_lookup, round, &params, &table, 1.0);

        // Compare against finite differences for each parameter
        let eps = 1e-5;
        let mut max_rel_error = 0.0f64;
        let mut worst_param = 0;

        for i in 0..NUM_PARAMS {
            // Skip non-differentiable control params
            if i >= 201 { continue; }

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

        eprintln!("Max relative gradient error: {:.6} at param {}", max_rel_error, worst_param);
        assert!(max_rel_error < 0.01, "Maximum relative error {:.6} exceeds 1%", max_rel_error);
    }
}
