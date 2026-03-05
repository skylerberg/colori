use crate::action_phase::can_afford_buyer;
use crate::colors::{mix_result, VALID_MIX_PAIRS};
use crate::types::*;
use crate::unordered_cards::UnorderedCards;
use rand::Rng;
use rand::RngExt;
use serde::{Deserialize, Serialize};

// ── Weight layout ──
// Draft:        8 weights  [0..8)
// Action card: 10 weights  [8..18)
// Buyer:        4 weights  [18..22)
// Mix:          4 weights  [22..26)
// Gain color:   3 weights  [26..29)

pub const DRAFT_OFFSET: usize = 0;
pub const DRAFT_WEIGHTS: usize = 8;
pub const ACTION_OFFSET: usize = 8;
pub const ACTION_WEIGHTS: usize = 10;
pub const BUYER_OFFSET: usize = 18;
pub const BUYER_WEIGHTS: usize = 4;
pub const MIX_OFFSET: usize = 22;
pub const MIX_WEIGHTS: usize = 4;
pub const GAIN_OFFSET: usize = 26;
pub const GAIN_WEIGHTS: usize = 3;
pub const TOTAL_WEIGHTS: usize = 29;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RolloutPolicy {
    pub weights: [f64; TOTAL_WEIGHTS],
    pub temperature: f64,
}

impl RolloutPolicy {
    pub fn uniform() -> Self {
        RolloutPolicy {
            weights: [0.0; TOTAL_WEIGHTS],
            temperature: 1.0,
        }
    }
}

// ── Softmax sampling ──

const MAX_OPTIONS: usize = 16;

#[inline]
fn softmax_sample<R: Rng>(scores: &[f64], temperature: f64, rng: &mut R) -> usize {
    let n = scores.len();
    debug_assert!(n > 0 && n <= MAX_OPTIONS);
    if n == 1 {
        return 0;
    }

    // Find max for numerical stability
    let mut max_score = scores[0];
    for i in 1..n {
        if scores[i] > max_score {
            max_score = scores[i];
        }
    }

    // Compute exp((score - max) / temperature) and cumulative sum
    let inv_temp = 1.0 / temperature;
    let mut cumulative = [0.0f64; MAX_OPTIONS];
    cumulative[0] = ((scores[0] - max_score) * inv_temp).exp();
    for i in 1..n {
        cumulative[i] = cumulative[i - 1] + ((scores[i] - max_score) * inv_temp).exp();
    }

    // Sample
    let total = cumulative[n - 1];
    let r = rng.random_range(0.0..total);
    for i in 0..n {
        if r < cumulative[i] {
            return i;
        }
    }
    n - 1
}

// ── Helper: count buyer colors needed ──

#[inline]
fn buyer_needs_color(buyer: &BuyerCard, color: Color) -> bool {
    buyer.color_cost().iter().any(|&c| c == color)
}

#[inline]
fn count_buyers_needing_color(buyer_display: &[BuyerInstance], color: Color) -> f64 {
    let mut count = 0u32;
    for b in buyer_display {
        if buyer_needs_color(&b.buyer, color) {
            count += 1;
        }
    }
    count as f64
}

// ── Draft pick scoring ──

pub fn score_draft_picks<R: Rng>(
    hand: &UnorderedCards,
    player: &PlayerState,
    buyer_display: &[BuyerInstance],
    card_lookup: &[Card; 256],
    weights: &[f64; TOTAL_WEIGHTS],
    temperature: f64,
    rng: &mut R,
) -> u8 {
    let w = &weights[DRAFT_OFFSET..DRAFT_OFFSET + DRAFT_WEIGHTS];
    let mut ids = [0u8; MAX_OPTIONS];
    let mut scores = [0.0f64; MAX_OPTIONS];
    let mut count = 0usize;

    for id in hand.iter() {
        if count >= MAX_OPTIONS {
            break;
        }
        let card = card_lookup[id as usize];
        let pips = card.pips();

        // Feature 0: Total pip count
        let mut score = w[0] * pips.len() as f64;

        // Feature 1: Pips matching colors needed by visible buyers that the player lacks
        let mut matching_needed = 0u32;
        for &pip_color in pips {
            if player.color_wheel.get(pip_color) == 0 {
                for b in buyer_display.iter() {
                    if buyer_needs_color(&b.buyer, pip_color) {
                        matching_needed += 1;
                        break;
                    }
                }
            }
        }
        score += w[1] * matching_needed as f64;

        // Feature 2: Card provides material type matching any visible buyer
        let mat_types = card.material_types();
        let mut matches_buyer_mat = false;
        for mt in mat_types {
            for b in buyer_display.iter() {
                if b.buyer.required_material() == *mt {
                    matches_buyer_mat = true;
                    break;
                }
            }
            if matches_buyer_mat {
                break;
            }
        }
        score += w[2] * matches_buyer_mat as u32 as f64;

        // Features 3-7: Ability type flags
        let ability = card.ability();
        score += w[3] * matches!(ability, Ability::Sell) as u32 as f64;
        score += w[4] * matches!(ability, Ability::MixColors { .. }) as u32 as f64;
        score += w[5] * matches!(ability, Ability::Workshop { .. }) as u32 as f64;
        score += w[6] * matches!(ability, Ability::DestroyCards) as u32 as f64;
        let is_other = matches!(
            ability,
            Ability::DrawCards { .. }
                | Ability::GainDucats { .. }
                | Ability::GainPrimary
                | Ability::GainSecondary
                | Ability::ChangeTertiary
        );
        score += w[7] * is_other as u32 as f64;

        ids[count] = id;
        scores[count] = score;
        count += 1;
    }

    if count == 0 {
        return hand.pick_random(rng).unwrap();
    }

    let idx = softmax_sample(&scores[..count], temperature, rng);
    ids[idx]
}

// ── Action card scoring ──

pub fn score_action_cards<R: Rng>(
    drafted: &UnorderedCards,
    player: &PlayerState,
    buyer_display: &[BuyerInstance],
    card_lookup: &[Card; 256],
    weights: &[f64; TOTAL_WEIGHTS],
    temperature: f64,
    rng: &mut R,
) -> u8 {
    let w = &weights[ACTION_OFFSET..ACTION_OFFSET + ACTION_WEIGHTS];
    let mut ids = [0u8; MAX_OPTIONS];
    let mut scores = [0.0f64; MAX_OPTIONS];
    let mut count = 0usize;

    for id in drafted.iter() {
        if count >= MAX_OPTIONS {
            break;
        }
        let card = card_lookup[id as usize];
        let ability = card.ability();
        let mut score = 0.0f64;

        // Feature 0: Card has Sell ability AND player can afford a buyer
        let is_sell = matches!(ability, Ability::Sell);
        let can_afford_any = is_sell
            && buyer_display
                .iter()
                .any(|b| can_afford_buyer(player, &b.buyer));
        score += w[0] * can_afford_any as u32 as f64;

        // Feature 1: Best affordable buyer star value if Sell ability
        if is_sell {
            let mut best_stars = 0u32;
            for b in buyer_display.iter() {
                if can_afford_buyer(player, &b.buyer) && b.buyer.stars() > best_stars {
                    best_stars = b.buyer.stars();
                }
            }
            score += w[1] * best_stars as f64;
        }

        // Feature 2: Card has MixColors AND player has mixable pairs
        let is_mix = matches!(ability, Ability::MixColors { .. });
        let has_mixable = is_mix
            && VALID_MIX_PAIRS
                .iter()
                .any(|&(a, b)| player.color_wheel.get(a) > 0 && player.color_wheel.get(b) > 0);
        score += w[2] * has_mixable as u32 as f64;

        // Feature 3: Number of mixable outputs that are needed by visible buyers
        if is_mix {
            let mut mixable_needed = 0u32;
            for &(a, b) in &VALID_MIX_PAIRS {
                if player.color_wheel.get(a) > 0 && player.color_wheel.get(b) > 0 {
                    let result = mix_result(a, b);
                    for bi in buyer_display.iter() {
                        if buyer_needs_color(&bi.buyer, result) {
                            mixable_needed += 1;
                            break;
                        }
                    }
                }
            }
            score += w[3] * mixable_needed as f64;
        }

        // Feature 4: Card has Workshop AND player has workshop cards
        let is_workshop = matches!(ability, Ability::Workshop { .. });
        let has_workshop_cards = !player.workshop_cards.is_empty();
        score += w[4] * (is_workshop && has_workshop_cards) as u32 as f64;

        // Feature 5: Number of workshop cards available
        if is_workshop {
            score += w[5] * player.workshop_cards.len() as f64;
        }

        // Feature 6: Card has DestroyCards AND player has workshop cards
        let is_destroy = matches!(ability, Ability::DestroyCards);
        score += w[6] * (is_destroy && has_workshop_cards) as u32 as f64;

        // Feature 7: Total pip count
        score += w[7] * card.pips().len() as f64;

        // Feature 8: Card contributes material when workshopped
        let has_materials = !card.material_types().is_empty();
        score += w[8] * has_materials as u32 as f64;

        // Feature 9: Bias
        score += w[9];

        ids[count] = id;
        scores[count] = score;
        count += 1;
    }

    if count == 0 {
        panic!("No drafted cards to score");
    }

    let idx = softmax_sample(&scores[..count], temperature, rng);
    ids[idx]
}

// ── Buyer scoring ──

pub fn score_buyers<R: Rng>(
    player: &PlayerState,
    buyer_display: &[BuyerInstance],
    weights: &[f64; TOTAL_WEIGHTS],
    temperature: f64,
    rng: &mut R,
) -> Option<u32> {
    let w = &weights[BUYER_OFFSET..BUYER_OFFSET + BUYER_WEIGHTS];
    let mut buyer_ids = [0u32; MAX_OPTIONS];
    let mut scores = [0.0f64; MAX_OPTIONS];
    let mut count = 0usize;

    for buyer in buyer_display.iter() {
        if count >= MAX_OPTIONS {
            break;
        }
        if !can_afford_buyer(player, &buyer.buyer) {
            continue;
        }

        let stars = buyer.buyer.stars();
        let mut score = 0.0f64;

        // Feature 0: Star value
        score += w[0] * stars as f64;

        // Feature 1: Is 4-star buyer
        score += w[1] * (stars == 4) as u32 as f64;

        // Feature 2: Number of excess colors remaining after paying cost
        let cost = buyer.buyer.color_cost();
        let mut excess = 0u32;
        let mut used = [0u32; 12];
        for &c in cost {
            used[c.index()] += 1;
        }
        for i in 0..12 {
            let have = player.color_wheel.counts[i];
            if have > used[i] {
                excess += have - used[i];
            }
        }
        score += w[2] * excess as f64;

        // Feature 3: Bias
        score += w[3];

        buyer_ids[count] = buyer.instance_id;
        scores[count] = score;
        count += 1;
    }

    if count == 0 {
        return None;
    }

    let idx = softmax_sample(&scores[..count], temperature, rng);
    Some(buyer_ids[idx])
}

// ── Mix scoring ──

pub fn score_mixes<R: Rng>(
    wheel: &ColorWheel,
    remaining: u32,
    buyer_display: &[BuyerInstance],
    weights: &[f64; TOTAL_WEIGHTS],
    temperature: f64,
    rng: &mut R,
) -> ([(Color, Color); 2], usize) {
    let w = &weights[MIX_OFFSET..MIX_OFFSET + MIX_WEIGHTS];
    let mut mixes = [(Color::Red, Color::Red); 2];
    let mut mix_count = 0usize;
    let mut sim_wheel = wheel.clone();

    for _ in 0..remaining {
        if mix_count >= 2 {
            break;
        }

        // Gather valid pairs
        let mut pairs = [(Color::Red, Color::Red); 9];
        let mut pair_count = 0usize;
        let mut scores = [0.0f64; MAX_OPTIONS]; // up to 9 pairs + 1 stop = 10

        for &(a, b) in &VALID_MIX_PAIRS {
            if sim_wheel.get(a) > 0 && sim_wheel.get(b) > 0 {
                let result = mix_result(a, b);

                let mut score = 0.0f64;

                // Feature 0: Output color needed by a visible buyer
                let needed = buyer_display
                    .iter()
                    .any(|bi| buyer_needs_color(&bi.buyer, result));
                score += w[0] * needed as u32 as f64;

                // Feature 1: Number of visible buyers needing output color
                score += w[1] * count_buyers_needing_color(buyer_display, result);

                // Feature 2: Is "stop mixing" option - 0 for actual mixes
                // score += w[2] * 0.0;

                // Feature 3: Bias
                score += w[3];

                pairs[pair_count] = (a, b);
                scores[pair_count] = score;
                pair_count += 1;
            }
        }

        if pair_count == 0 {
            break;
        }

        // Add "stop" option
        let stop_idx = pair_count;
        let total_options = pair_count + 1;
        scores[stop_idx] = w[2] * 1.0 + w[3]; // stop feature + bias

        let idx = softmax_sample(&scores[..total_options], temperature, rng);
        if idx == stop_idx {
            break;
        }

        let (a, b) = pairs[idx];
        mixes[mix_count] = (a, b);
        mix_count += 1;
        crate::colors::perform_mix_unchecked(&mut sim_wheel, a, b);
    }

    (mixes, mix_count)
}

// ── Gain color scoring ──

pub fn score_gain_color<R: Rng>(
    options: &[Color],
    buyer_display: &[BuyerInstance],
    weights: &[f64; TOTAL_WEIGHTS],
    temperature: f64,
    rng: &mut R,
) -> Color {
    let w = &weights[GAIN_OFFSET..GAIN_OFFSET + GAIN_WEIGHTS];
    let mut scores = [0.0f64; MAX_OPTIONS];
    let n = options.len().min(MAX_OPTIONS);

    for i in 0..n {
        let color = options[i];
        let mut score = 0.0f64;

        // Feature 0: Color needed by a visible buyer
        let needed = buyer_display
            .iter()
            .any(|bi| buyer_needs_color(&bi.buyer, color));
        score += w[0] * needed as u32 as f64;

        // Feature 1: Number of visible buyers needing this color
        score += w[1] * count_buyers_needing_color(buyer_display, color);

        // Feature 2: Bias
        score += w[2];

        scores[i] = score;
    }

    let idx = softmax_sample(&scores[..n], temperature, rng);
    options[idx]
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn test_softmax_sample_single_option() {
        let mut rng = wyrand::WyRand::seed_from_u64(42);
        let scores = [1.0];
        assert_eq!(softmax_sample(&scores, 1.0, &mut rng), 0);
    }

    #[test]
    fn test_softmax_sample_uniform() {
        let mut rng = wyrand::WyRand::seed_from_u64(42);
        let scores = [0.0, 0.0, 0.0];
        let mut counts = [0u32; 3];
        for _ in 0..3000 {
            let idx = softmax_sample(&scores, 1.0, &mut rng);
            counts[idx] += 1;
        }
        // Each should be roughly 1000 (within reasonable variance)
        for &c in &counts {
            assert!(c > 700 && c < 1300, "count {} outside expected range", c);
        }
    }

    #[test]
    fn test_softmax_sample_strongly_favors_high_score() {
        let mut rng = wyrand::WyRand::seed_from_u64(42);
        let scores = [0.0, 0.0, 10.0];
        let mut counts = [0u32; 3];
        for _ in 0..1000 {
            let idx = softmax_sample(&scores, 1.0, &mut rng);
            counts[idx] += 1;
        }
        // Option 2 should be overwhelmingly chosen
        assert!(counts[2] > 950, "high score option chosen {} times", counts[2]);
    }

    #[test]
    fn test_uniform_policy_equivalent_to_random() {
        let policy = RolloutPolicy::uniform();
        assert_eq!(policy.weights, [0.0; TOTAL_WEIGHTS]);
        assert_eq!(policy.temperature, 1.0);
        // All-zero weights produce equal scores, so softmax is uniform
        let mut rng = wyrand::WyRand::seed_from_u64(42);
        let scores = [0.0; 4];
        let mut counts = [0u32; 4];
        for _ in 0..4000 {
            let idx = softmax_sample(&scores, 1.0, &mut rng);
            counts[idx] += 1;
        }
        for &c in &counts {
            assert!(c > 700 && c < 1300, "count {} outside expected range", c);
        }
    }
}
