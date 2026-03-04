use crate::action_phase::{can_afford_buyer, can_sell_to_any_buyer};
use crate::apply_choice::resolve_card_types_to_ids;
use crate::colors::{can_mix, perform_mix_unchecked, PRIMARIES, SECONDARIES, TERTIARIES, VALID_MIX_PAIRS};
use crate::types::*;
use crate::unordered_cards::UnorderedCards;
use smallvec::SmallVec;

// ── Multiset subset enumeration ──

/// Count occurrences of each card type in an UnorderedCards bitset,
/// returning a sorted array of (Card, count) pairs and the number of distinct types.
fn count_card_types(
    mask: UnorderedCards,
    card_lookup: &[Card; 256],
) -> ([Card; 49], [u8; 49], usize) {
    let mut card_types = [Card::BasicRed; 49];
    let mut type_counts = [0u8; 49];
    let mut len = 0usize;
    let mut seen: u64 = 0;
    for id in mask.iter() {
        let card = card_lookup[id as usize];
        let bit = 1u64 << (card as u64);
        if seen & bit == 0 {
            seen |= bit;
            card_types[len] = card;
            type_counts[len] = 1;
            len += 1;
        } else {
            for i in 0..len {
                if card_types[i] == card {
                    type_counts[i] += 1;
                    break;
                }
            }
        }
    }
    // Sort by card discriminant for deterministic output
    for i in 1..len {
        let mut j = i;
        while j > 0 && (card_types[j] as usize) < (card_types[j - 1] as usize) {
            card_types.swap(j, j - 1);
            type_counts.swap(j, j - 1);
            j -= 1;
        }
    }
    (card_types, type_counts, len)
}

/// Enumerate all non-empty subsets of a card-type multiset up to max_size.
/// Produces unique sorted SmallVec<[Card; 4]> entries without needing deduplication.
fn enumerate_multiset_subsets(
    types: &[Card],
    counts: &[u8],
    max_remaining: usize,
    current_subset: &mut SmallVec<[Card; 4]>,
    choices: &mut Vec<Choice>,
    make_choice: &impl Fn(SmallVec<[Card; 4]>) -> Choice,
) {
    if types.is_empty() || max_remaining == 0 {
        if !current_subset.is_empty() {
            choices.push(make_choice(current_subset.clone()));
        }
        return;
    }
    let card = types[0];
    let count = counts[0] as usize;
    let max_take = max_remaining.min(count);
    let base_len = current_subset.len();
    for take in 0..=max_take {
        enumerate_multiset_subsets(
            &types[1..],
            &counts[1..],
            max_remaining - take,
            current_subset,
            choices,
            make_choice,
        );
        current_subset.push(card);
    }
    current_subset.truncate(base_len);
}

// ── Mix sequence enumeration ──

fn enumerate_mix_sequences<F>(
    wheel: &ColorWheel,
    remaining_mixes: u32,
    choices: &mut Vec<Choice>,
    make_choice: F,
) where
    F: Fn(SmallVec<[(Color, Color); 2]>) -> Choice,
{
    // Always include skip-all (empty mixes)
    choices.push(make_choice(SmallVec::new()));

    for &(a, b) in &VALID_MIX_PAIRS {
        if wheel.get(a) > 0 && wheel.get(b) > 0 {
            let mut mixes1 = SmallVec::new();
            mixes1.push((a, b));
            choices.push(make_choice(mixes1));

            if remaining_mixes > 1 {
                let mut wheel2 = wheel.clone();
                perform_mix_unchecked(&mut wheel2, a, b);
                for &(c, d) in &VALID_MIX_PAIRS {
                    if wheel2.get(c) > 0 && wheel2.get(d) > 0 {
                        let mut mixes2 = SmallVec::new();
                        mixes2.push((a, b));
                        mixes2.push((c, d));
                        choices.push(make_choice(mixes2));
                    }
                }
            }
        }
    }
}

// ── Destroy choice dispatch ──

fn enumerate_destroy_choices(
    state: &GameState,
    player: &PlayerState,
    card: Card,
    choices: &mut Vec<Choice>,
) {
    match card.ability() {
        Ability::MixColors { count } => {
            enumerate_mix_sequences(
                &player.color_wheel,
                count,
                choices,
                |mixes| Choice::DestroyAndMix { card, mixes },
            );
        }
        Ability::Sell => {
            let mut has_buyer = false;
            for buyer in state.buyer_display.iter() {
                if can_afford_buyer(player, &buyer.buyer) {
                    has_buyer = true;
                    choices.push(Choice::DestroyAndSell {
                        card,
                        buyer: buyer.buyer,
                    });
                }
            }
            if !has_buyer {
                choices.push(Choice::DestroyDraftedCard { card });
            }
        }
        Ability::Workshop { count } => {
            // Skip option (empty workshop_cards)
            choices.push(Choice::DestroyAndWorkshop {
                card,
                workshop_cards: SmallVec::new(),
            });
            let (card_types, type_counts, len) =
                count_card_types(player.workshop_cards, &state.card_lookup);
            if len > 0 {
                enumerate_multiset_subsets(
                    &card_types[..len],
                    &type_counts[..len],
                    count as usize,
                    &mut SmallVec::new(),
                    choices,
                    &|workshop_cards| Choice::DestroyAndWorkshop {
                        card,
                        workshop_cards,
                    },
                );
            }
        }
        Ability::DestroyCards => {
            if player.workshop_cards.is_empty() {
                choices.push(Choice::DestroyAndDestroyCards {
                    card,
                    target: None,
                });
            } else {
                let mut seen: u64 = 0;
                for id in player.workshop_cards.iter() {
                    let target_card = state.card_lookup[id as usize];
                    let bit = 1u64 << (target_card as u64);
                    if seen & bit != 0 {
                        continue;
                    }
                    seen |= bit;
                    choices.push(Choice::DestroyAndDestroyCards {
                        card,
                        target: Some(target_card),
                    });
                }
            }
        }
        _ => {
            choices.push(Choice::DestroyDraftedCard { card });
        }
    }
}

// ── Choice enumeration ──

pub fn enumerate_choices_into(state: &GameState, choices: &mut Vec<Choice>) {
    choices.clear();
    match &state.phase {
        GamePhase::Draft { draft_state } => {
            let hand = draft_state.hands[draft_state.current_player_index];
            let mut seen: u64 = 0;
            for id in hand.iter() {
                let card = state.card_lookup[id as usize];
                let bit = 1u64 << (card as u64);
                if seen & bit != 0 { continue; }
                seen |= bit;
                choices.push(Choice::DraftPick { card });
            }
        }
        GamePhase::Action { action_state } => {
            let player = &state.players[action_state.current_player_index];

            match action_state.ability_stack.last() {
                None => {
                    let mut seen: u64 = 0;
                    for id in player.drafted_cards.iter() {
                        let card = state.card_lookup[id as usize];
                        let bit = 1u64 << (card as u64);
                        if seen & bit != 0 { continue; }
                        seen |= bit;
                        enumerate_destroy_choices(state, player, card, choices);
                    }
                    choices.push(Choice::EndTurn);
                }
                Some(Ability::Workshop { count }) => {
                    choices.push(Choice::SkipWorkshop);
                    let (card_types, type_counts, len) = count_card_types(player.workshop_cards, &state.card_lookup);
                    enumerate_multiset_subsets(
                        &card_types[..len],
                        &type_counts[..len],
                        *count as usize,
                        &mut SmallVec::new(),
                        choices,
                        &|card_types| Choice::Workshop { card_types },
                    );
                }
                Some(Ability::DestroyCards) => {
                    if player.workshop_cards.is_empty() {
                        choices.push(Choice::DestroyDrawnCards { card: None });
                    } else {
                        let mut seen: u64 = 0;
                        for id in player.workshop_cards.iter() {
                            let card = state.card_lookup[id as usize];
                            let bit = 1u64 << (card as u64);
                            if seen & bit != 0 { continue; }
                            seen |= bit;
                            choices.push(Choice::DestroyDrawnCards { card: Some(card) });
                        }
                    }
                }
                Some(Ability::MixColors { count }) => {
                    enumerate_mix_sequences(
                        &player.color_wheel,
                        *count,
                        choices,
                        |mixes| Choice::MixAll { mixes },
                    );
                }
                Some(Ability::Sell) => {
                    for buyer in state.buyer_display.iter() {
                        if can_afford_buyer(player, &buyer.buyer) {
                            choices.push(Choice::SelectBuyer {
                                buyer: buyer.buyer,
                            });
                        }
                    }
                }
                Some(Ability::GainSecondary) => {
                    for &c in SECONDARIES.iter() {
                        choices.push(Choice::GainSecondary { color: c });
                    }
                }
                Some(Ability::GainPrimary) => {
                    for &c in PRIMARIES.iter() {
                        choices.push(Choice::GainPrimary { color: c });
                    }
                }
                Some(Ability::ChangeTertiary) => {
                    for &lose in TERTIARIES.iter() {
                        if player.color_wheel.get(lose) > 0 {
                            for &gain in TERTIARIES.iter() {
                                if gain != lose {
                                    choices.push(Choice::SwapTertiary { lose, gain });
                                }
                            }
                        }
                    }
                }
                // Instant abilities (DrawCards, GainDucats) should never be on top
                // when waiting for a choice — they get processed immediately.
                Some(_) => {}
            }
        }
        _ => {}
    }
}

pub fn enumerate_choices(state: &GameState) -> Vec<Choice> {
    let mut choices = Vec::new();
    enumerate_choices_into(state, &mut choices);
    choices
}

// ── Choice availability ──

/// Check common preconditions for all destroy-drafted-card variants:
/// ability stack is empty and the card exists in the current player's drafted_cards.
fn check_destroy_preconditions(state: &GameState, card: &Card) -> bool {
    if let GamePhase::Action { ref action_state } = state.phase {
        action_state.ability_stack.is_empty()
            && state.players[action_state.current_player_index]
                .drafted_cards
                .iter()
                .any(|id| state.card_lookup[id as usize] == *card)
    } else {
        false
    }
}

pub fn check_choice_available(state: &GameState, choice: &Choice) -> bool {
    match choice {
        Choice::DraftPick { card } => {
            if let GamePhase::Draft { ref draft_state } = state.phase {
                let hand = draft_state.hands[draft_state.current_player_index];
                hand.iter().any(|id| state.card_lookup[id as usize] == *card)
            } else {
                false
            }
        }
        Choice::DestroyDraftedCard { card } => {
            if !check_destroy_preconditions(state, card) {
                return false;
            }
            match card.ability() {
                Ability::Sell => !can_sell_to_any_buyer(state),
                Ability::MixColors { .. } => false,
                Ability::Workshop { .. } => false,
                Ability::DestroyCards => false,
                _ => true,
            }
        }
        Choice::EndTurn => {
            if let GamePhase::Action { ref action_state } = state.phase {
                action_state.ability_stack.is_empty()
            } else {
                false
            }
        }
        Choice::Workshop { card_types } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match action_state.ability_stack.last() {
                    Some(Ability::Workshop { .. }) => {
                        if card_types.is_empty() {
                            return false;
                        }
                        let player = &state.players[action_state.current_player_index];
                        resolve_card_types_to_ids(card_types, &player.workshop_cards, &state.card_lookup).is_some()
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
        Choice::SkipWorkshop => {
            if let GamePhase::Action { ref action_state } = state.phase {
                matches!(
                    action_state.ability_stack.last(),
                    Some(Ability::Workshop { .. })
                )
            } else {
                false
            }
        }
        Choice::DestroyDrawnCards { card } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match action_state.ability_stack.last() {
                    Some(Ability::DestroyCards) => match card {
                        None => true,
                        Some(card) => {
                            let player = &state.players[action_state.current_player_index];
                            player.workshop_cards.iter().any(|id| state.card_lookup[id as usize] == *card)
                        }
                    },
                    _ => false,
                }
            } else {
                false
            }
        }
        Choice::SelectBuyer { buyer } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match action_state.ability_stack.last() {
                    Some(Ability::Sell) => {
                        let player = &state.players[action_state.current_player_index];
                        state.buyer_display.iter().any(|b| b.buyer == *buyer && can_afford_buyer(player, &b.buyer))
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
        Choice::GainSecondary { color } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                matches!(
                    action_state.ability_stack.last(),
                    Some(Ability::GainSecondary)
                ) && SECONDARIES.contains(color)
            } else {
                false
            }
        }
        Choice::GainPrimary { color } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                matches!(
                    action_state.ability_stack.last(),
                    Some(Ability::GainPrimary)
                ) && PRIMARIES.contains(color)
            } else {
                false
            }
        }
        Choice::MixAll { mixes } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match action_state.ability_stack.last() {
                    Some(Ability::MixColors { .. }) => {
                        if mixes.is_empty() {
                            return true;
                        }
                        let player = &state.players[action_state.current_player_index];
                        let (a, b) = mixes[0];
                        player.color_wheel.get(a) > 0
                            && player.color_wheel.get(b) > 0
                            && can_mix(a, b)
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
        Choice::SwapTertiary { lose, gain } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match action_state.ability_stack.last() {
                    Some(Ability::ChangeTertiary) => {
                        let player = &state.players[action_state.current_player_index];
                        TERTIARIES.contains(lose)
                            && player.color_wheel.get(*lose) > 0
                            && TERTIARIES.contains(gain)
                            && *lose != *gain
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
        Choice::DestroyAndMix { card, mixes } => {
            if !check_destroy_preconditions(state, card) {
                return false;
            }
            if mixes.is_empty() {
                return true;
            }
            let player = &state.players[match &state.phase {
                GamePhase::Action { action_state } => action_state.current_player_index,
                _ => return false,
            }];
            let (a, b) = mixes[0];
            if player.color_wheel.get(a) == 0 || player.color_wheel.get(b) == 0 || !can_mix(a, b) {
                return false;
            }
            if mixes.len() > 1 {
                let mut wheel = player.color_wheel.clone();
                perform_mix_unchecked(&mut wheel, a, b);
                let (c, d) = mixes[1];
                if wheel.get(c) == 0 || wheel.get(d) == 0 || !can_mix(c, d) {
                    return false;
                }
            }
            true
        }
        Choice::DestroyAndSell { card, buyer } => {
            if !check_destroy_preconditions(state, card) {
                return false;
            }
            let player = &state.players[match &state.phase {
                GamePhase::Action { action_state } => action_state.current_player_index,
                _ => return false,
            }];
            state.buyer_display.iter().any(|b| b.buyer == *buyer && can_afford_buyer(player, &b.buyer))
        }
        Choice::DestroyAndWorkshop { card, workshop_cards } => {
            if !check_destroy_preconditions(state, card) {
                return false;
            }
            if workshop_cards.is_empty() {
                return true;
            }
            let player = &state.players[match &state.phase {
                GamePhase::Action { action_state } => action_state.current_player_index,
                _ => return false,
            }];
            resolve_card_types_to_ids(workshop_cards, &player.workshop_cards, &state.card_lookup).is_some()
        }
        Choice::DestroyAndDestroyCards { card, target } => {
            if !check_destroy_preconditions(state, card) {
                return false;
            }
            match target {
                None => true,
                Some(target_card) => {
                    let player = &state.players[match &state.phase {
                        GamePhase::Action { action_state } => action_state.current_player_index,
                        _ => return false,
                    }];
                    player.workshop_cards.iter().any(|id| state.card_lookup[id as usize] == *target_card)
                }
            }
        }
    }
}

