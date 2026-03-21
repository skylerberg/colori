use serde::{Deserialize, Serialize};

use crate::types::*;
use crate::unordered_cards::UnorderedCards;

/// Number of draftable card types (all cards that appear in the draft deck).
pub const NUM_DRAFT_CARD_TYPES: usize = 37;
/// Number of colors on the color wheel.
const NUM_COLORS: usize = 12;
/// Number of action card types in the draft deck.
const NUM_ACTION_TYPES: usize = 4;
/// Symmetric upper triangle size for color synergy matrix.
const COLOR_SYNERGY_SIZE: usize = NUM_COLORS * (NUM_COLORS + 1) / 2; // 78
/// Symmetric upper triangle size for action interaction matrix.
const ACTION_INTERACTION_SIZE: usize = NUM_ACTION_TYPES * (NUM_ACTION_TYPES + 1) / 2; // 10

/// Total number of optimizable parameters.
pub const NUM_OPENING_BOOK_PARAMS: usize =
    NUM_DRAFT_CARD_TYPES + COLOR_SYNERGY_SIZE + 2 + ACTION_INTERACTION_SIZE; // 37 + 78 + 2 + 10 = 127

// Parameter layout indices
const BASE_PRIORITY_START: usize = 0;
const COLOR_SYNERGY_START: usize = BASE_PRIORITY_START + NUM_DRAFT_CARD_TYPES; // 37
const MATERIAL_WEIGHT_IDX: usize = COLOR_SYNERGY_START + COLOR_SYNERGY_SIZE; // 115
const COLOR_SELL_WEIGHT_IDX: usize = MATERIAL_WEIGHT_IDX + 1; // 116
const ACTION_INTERACTION_START: usize = COLOR_SELL_WEIGHT_IDX + 1; // 117

/// The 37 card types that appear in the draft deck, in a fixed order.
/// This order defines the index used in base_priority.
const DRAFT_CARDS: [Card; NUM_DRAFT_CARD_TYPES] = [
    // Dyes (21)
    Card::Kermes, Card::Weld, Card::Woad,
    Card::Lac, Card::Brazilwood, Card::Pomegranate,
    Card::Sumac, Card::Elderberry, Card::Turnsole,
    Card::Madder, Card::Turmeric, Card::DyersGreenweed,
    Card::Verdigris, Card::Orchil, Card::Logwood,
    Card::VermilionDye, Card::Saffron, Card::PersianBerries,
    Card::Azurite, Card::IndigoDye, Card::Cochineal,
    // Materials (12)
    Card::TerraCotta, Card::OchreWare, Card::CobaltWare,
    Card::CinnabarCanvas, Card::OrpimentCanvas, Card::UltramarineCanvas,
    Card::AlizarinFabric, Card::FusticFabric, Card::PastelFabric,
    Card::ClayCanvas, Card::ClayFabric, Card::CanvasFabric,
    // Actions (4)
    Card::Alum, Card::CreamOfTartar, Card::GumArabic, Card::Potash,
];

/// The 4 action card types, in index order matching DRAFT_CARDS[33..37].
const ACTION_CARDS: [Card; NUM_ACTION_TYPES] = [
    Card::Alum, Card::CreamOfTartar, Card::GumArabic, Card::Potash,
];

/// Map Card variant to draft card index (0..37), or None if not a draft card.
fn draft_card_index(card: Card) -> Option<usize> {
    DRAFT_CARDS.iter().position(|&c| c == card)
}

/// Map Card variant to action card index (0..4), or None if not an action card in draft.
fn action_card_index(card: Card) -> Option<usize> {
    ACTION_CARDS.iter().position(|&c| c == card)
}

/// Index into the symmetric upper triangle for a pair (i, j) where i <= j.
#[inline]
fn symmetric_index(i: usize, j: usize, _n: usize) -> usize {
    let (lo, hi) = if i <= j { (i, j) } else { (j, i) };
    hi * (hi + 1) / 2 + lo
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpeningBookParams {
    pub weights: Vec<f64>,
}

impl Default for OpeningBookParams {
    fn default() -> Self {
        OpeningBookParams {
            weights: vec![0.0; NUM_OPENING_BOOK_PARAMS],
        }
    }
}

impl OpeningBookParams {
    pub fn from_vec(v: &[f64]) -> Self {
        assert!(v.len() >= NUM_OPENING_BOOK_PARAMS);
        OpeningBookParams {
            weights: v[..NUM_OPENING_BOOK_PARAMS].to_vec(),
        }
    }

    pub fn to_vec(&self) -> Vec<f64> {
        self.weights.clone()
    }

    #[inline]
    fn base_priority(&self, card_idx: usize) -> f64 {
        self.weights[BASE_PRIORITY_START + card_idx]
    }

    #[inline]
    fn color_synergy(&self, c1: usize, c2: usize) -> f64 {
        self.weights[COLOR_SYNERGY_START + symmetric_index(c1, c2, NUM_COLORS)]
    }

    #[inline]
    fn material_weight(&self) -> f64 {
        self.weights[MATERIAL_WEIGHT_IDX]
    }

    #[inline]
    fn color_sell_weight(&self) -> f64 {
        self.weights[COLOR_SELL_WEIGHT_IDX]
    }

    #[inline]
    fn action_interaction(&self, a1: usize, a2: usize) -> f64 {
        self.weights[ACTION_INTERACTION_START + symmetric_index(a1, a2, NUM_ACTION_TYPES)]
    }
}

/// Score a candidate card for drafting given already-drafted cards and sell card display.
pub fn score_draft_card(
    candidate: Card,
    drafted_cards: &UnorderedCards,
    sell_card_display: &[SellCardInstance],
    card_lookup: &[Card; 256],
    params: &OpeningBookParams,
) -> f64 {
    let candidate_idx = match draft_card_index(candidate) {
        Some(idx) => idx,
        None => return f64::NEG_INFINITY,
    };

    let mut score = params.base_priority(candidate_idx);

    let candidate_colors = candidate.colors();
    let candidate_materials = candidate.material_types();
    let candidate_action_idx = action_card_index(candidate);

    // Color synergy with already-drafted cards
    for drafted_id in drafted_cards.iter() {
        let drafted_card = card_lookup[drafted_id as usize];
        let drafted_colors = drafted_card.colors();

        for &c1 in candidate_colors {
            for &c2 in drafted_colors {
                score += params.color_synergy(c1.index(), c2.index());
            }
        }

        // Action-action interaction
        if let Some(cand_ai) = candidate_action_idx {
            if let Some(draft_ai) = action_card_index(drafted_card) {
                score += params.action_interaction(cand_ai, draft_ai);
            }
        }
    }

    // Sell card alignment: material demand
    for &mt in candidate_materials {
        for sc in sell_card_display {
            if sc.sell_card.required_material() == mt {
                score += params.material_weight() * sc.sell_card.ducats() as f64;
            }
        }
    }

    // Sell card alignment: color demand
    for &color in candidate_colors {
        for sc in sell_card_display {
            for &cost_color in sc.sell_card.color_cost() {
                if cost_color == color {
                    score += params.color_sell_weight() * sc.sell_card.ducats() as f64;
                }
            }
        }
    }

    score
}

/// Pick the best card from a draft hand using the opening book.
/// Returns the Card type to pick. Deduplicates by card type (same as ISMCTS draft choices).
pub fn opening_book_pick(
    hand: &UnorderedCards,
    drafted_cards: &UnorderedCards,
    sell_card_display: &[SellCardInstance],
    card_lookup: &[Card; 256],
    params: &OpeningBookParams,
) -> Card {
    let mut best_card: Option<Card> = None;
    let mut best_score = f64::NEG_INFINITY;
    let mut seen: u64 = 0;

    for id in hand.iter() {
        let card = card_lookup[id as usize];
        let bit = 1u64 << (card as u64);
        if seen & bit != 0 {
            continue;
        }
        seen |= bit;

        let card_score = score_draft_card(
            card,
            drafted_cards,
            sell_card_display,
            card_lookup,
            params,
        );

        if card_score > best_score {
            best_score = card_score;
            best_card = Some(card);
        }
    }

    best_card.expect("Hand should not be empty")
}

/// Indices of frozen genes (cards not in the draft deck).
pub fn frozen_gene_indices() -> Vec<usize> {
    let frozen = Vec::new();
    // Vinegar and Argol are not in the draft deck but are in the Card enum.
    // They don't appear in DRAFT_CARDS, so they don't have base_priority indices.
    // All 37 entries in DRAFT_CARDS are actual draft cards — nothing to freeze.
    frozen
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixed_vec::FixedVec;

    #[test]
    fn test_default_params_produce_finite_scores() {
        let params = OpeningBookParams::default();
        let hand = UnorderedCards::new();
        let drafted = UnorderedCards::new();
        let display: FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY> = FixedVec::new();
        let card_lookup = [Card::BasicRed; 256];

        // All zeros should give score 0 for any card
        let score = score_draft_card(Card::Potash, &drafted, &display, &card_lookup, &params);
        assert!(score.is_finite());
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_base_priority_affects_score() {
        let mut params = OpeningBookParams::default();
        let potash_idx = draft_card_index(Card::Potash).unwrap();
        params.weights[BASE_PRIORITY_START + potash_idx] = 5.0;

        let drafted = UnorderedCards::new();
        let display: FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY> = FixedVec::new();
        let card_lookup = [Card::BasicRed; 256];

        let score = score_draft_card(Card::Potash, &drafted, &display, &card_lookup, &params);
        assert_eq!(score, 5.0);

        let score2 = score_draft_card(Card::Alum, &drafted, &display, &card_lookup, &params);
        assert_eq!(score2, 0.0);
    }

    #[test]
    fn test_color_synergy() {
        let mut params = OpeningBookParams::default();
        // Set Red-Blue synergy to positive
        let red_idx = Color::Red.index();
        let blue_idx = Color::Blue.index();
        params.weights[COLOR_SYNERGY_START + symmetric_index(red_idx, blue_idx, NUM_COLORS)] = 2.0;

        let mut card_lookup = [Card::BasicRed; 256];
        // Card 0 is Brazilwood (R, R, B)
        card_lookup[0] = Card::Brazilwood;

        let mut drafted = UnorderedCards::new();
        drafted.insert(0); // Brazilwood drafted

        let display: FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY> = FixedVec::new();

        // Lac has colors R, R, Y — Red-Red synergy with Brazilwood's R, R, B
        let score_lac = score_draft_card(Card::Lac, &drafted, &display, &card_lookup, &params);
        // Lac colors: [R, R, Y], Brazilwood colors: [R, R, B]
        // R-R: symmetric_index(0,0) = 0 → 0.0
        // R-R: same → 0.0
        // R-B: symmetric_index(0,8) → 2.0
        // R-B: same → 2.0
        // Y-R: symmetric_index(4,0) → 0.0
        // Y-R: same → 0.0
        // Y-B: symmetric_index(4,8) → 0.0
        // Total from R-B pairs: 4 pairs × 2.0 = 8.0 (R×R cross, each R with each B)
        // Lac has 2 Reds, Brazilwood has 1 Blue → 2 × 1 × 2.0 = 4.0
        assert!(score_lac > 0.0, "Lac should benefit from Red-Blue synergy with Brazilwood, got {}", score_lac);
    }

    #[test]
    fn test_symmetric_index() {
        assert_eq!(symmetric_index(0, 0, 12), 0);
        assert_eq!(symmetric_index(0, 1, 12), 1);
        assert_eq!(symmetric_index(1, 0, 12), 1); // symmetric
        assert_eq!(symmetric_index(1, 1, 12), 2);
        assert_eq!(symmetric_index(0, 2, 12), 3);
    }

    #[test]
    fn test_param_count() {
        assert_eq!(NUM_OPENING_BOOK_PARAMS, 127);
    }

    #[test]
    fn test_all_draft_cards_have_indices() {
        for &card in &DRAFT_CARDS {
            assert!(draft_card_index(card).is_some(), "Card {:?} should have a draft index", card);
        }
    }

    #[test]
    fn test_action_cards_have_indices() {
        for &card in &ACTION_CARDS {
            assert!(action_card_index(card).is_some());
        }
        assert!(action_card_index(Card::BasicRed).is_none());
    }
}
