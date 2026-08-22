mod destroy;
mod mix_sequences;
mod multiset;

use crate::action_phase::{
    can_afford_sell_card, can_sell_to_any_sell_card, for_each_unique_card_type,
    for_each_unique_card_type_in_workshop_area,
};
use crate::apply_choice::resolve_card_types_to_ids;
use crate::colors::{can_mix, perform_mix_unchecked, PRIMARIES, SECONDARIES};
use crate::types::*;
use smallvec::SmallVec;

use destroy::enumerate_destroy_choices;
use mix_sequences::enumerate_mix_sequences;
use multiset::{count_card_types, enumerate_multiset_subsets, enumerate_multiset_subsets_exact};

/// Whether sub-maximum workshop subsets can be skipped during enumeration.
///
/// Holding picks back only pays off when something later in the turn could
/// still add to the workshop or consume a card left sitting in it. When
/// nothing can, workshopping fewer cards than allowed is strictly worse and
/// the smaller subsets are dead branches in the search tree.
pub(crate) fn should_force_max_workshop(state: &GameState, player: &PlayerState) -> bool {
    if !state.force_max_workshop {
        return false;
    }
    for id in player.drafted_cards.iter() {
        match state.card_lookup[id as usize].ability() {
            Ability::MoveToDraftPool | Ability::DrawCards { .. } => return false,
            _ => {}
        }
    }
    for id in player.workshop_cards.iter() {
        let card = state.card_lookup[id as usize];
        for &wa in card.workshop_abilities() {
            if matches!(wa, Ability::DrawCards { .. }) {
                return false;
            }
        }
    }
    true
}

// ── Choice enumeration ──

pub fn enumerate_choices_into(state: &GameState, choices: &mut Vec<Choice>) {
    choices.clear();
    match &state.phase {
        GamePhase::Buyers { .. } => {
            enumerate_buyer_choices(state, choices);
        }
        GamePhase::Draft { draft_state } => {
            let hand = draft_state.hands[draft_state.current_player_index];
            for_each_unique_card_type(&hand, &state.card_lookup, |card| {
                choices.push(Choice::DraftPick { card });
            });
        }
        GamePhase::Action { action_state } => {
            let player = &state.players[action_state.current_player_index];

            match action_state.ability_stack.last() {
                None => {
                    for_each_unique_card_type(&player.drafted_cards, &state.card_lookup, |card| {
                        enumerate_destroy_choices(state, player, card, choices);
                    });
                    choices.push(Choice::EndTurn);
                }
                Some(Ability::Workshop { count }) => {
                    let force_max = should_force_max_workshop(state, player);
                    let (card_types, type_counts, len) = count_card_types(player.workshop_cards, &state.card_lookup);
                    let total_available: usize = type_counts[..len].iter().map(|&c| c as usize).sum();
                    if force_max {
                        if total_available <= *count as usize {
                            // Only one option: workshop everything
                            let mut all_cards = SmallVec::new();
                            for i in 0..len {
                                for _ in 0..type_counts[i] {
                                    all_cards.push(card_types[i]);
                                }
                            }
                            choices.push(Choice::Workshop { card_types: all_cards });
                        } else {
                            enumerate_multiset_subsets_exact(
                                &card_types[..len],
                                &type_counts[..len],
                                *count as usize,
                                &mut SmallVec::new(),
                                choices,
                                &|card_types| Choice::Workshop { card_types },
                            );
                        }
                    } else {
                        choices.push(Choice::SkipWorkshop);
                        enumerate_multiset_subsets(
                            &card_types[..len],
                            &type_counts[..len],
                            *count as usize,
                            &mut SmallVec::new(),
                            choices,
                            &|card_types| Choice::Workshop { card_types },
                        );
                    }
                }
                Some(Ability::MoveToDraftPool) => {
                    if player.workshop_cards.is_empty() && player.workshopped_cards.is_empty() {
                        choices.push(Choice::MoveToDraftPool { card: None });
                    } else {
                        for_each_unique_card_type_in_workshop_area(player, &state.card_lookup, |card| {
                            choices.push(Choice::MoveToDraftPool { card: Some(card) });
                        });
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
                    for sell_card in player.buyers.iter() {
                        if can_afford_sell_card(player, &sell_card.sell_card) {
                            choices.push(Choice::SelectSellCard {
                                sell_card: sell_card.sell_card,
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
                Some(Ability::GainMaterial) => {
                    for &m in ALL_MATERIAL_TYPES.iter() {
                        choices.push(Choice::GainMaterial { material: m });
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
                Ability::Sell => !can_sell_to_any_sell_card(state),
                Ability::MixColors { .. } => false,
                Ability::Workshop { .. } => false,
                Ability::MoveToDraftPool => false,
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
        Choice::MoveToDraftPool { card } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match action_state.ability_stack.last() {
                    Some(Ability::MoveToDraftPool) => {
                        let player = &state.players[action_state.current_player_index];
                        let area = player.workshop_cards.union(player.workshopped_cards);
                        match card {
                            // The move is mandatory when there is anything to
                            // move, so this stands only for the fizzle.
                            None => area.is_empty(),
                            Some(card) => {
                                area.iter().any(|id| state.card_lookup[id as usize] == *card)
                            }
                        }
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
        Choice::SelectSellCard { sell_card } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match action_state.ability_stack.last() {
                    Some(Ability::Sell) => {
                        let player = &state.players[action_state.current_player_index];
                        player.buyers.iter().any(|b| b.sell_card == *sell_card && can_afford_sell_card(player, &b.sell_card))
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
        Choice::GainMaterial { .. } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                matches!(action_state.ability_stack.last(), Some(Ability::GainMaterial))
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
        Choice::DestroyAndSell { card, sell_card } => {
            if !check_destroy_preconditions(state, card) {
                return false;
            }
            let player = &state.players[match &state.phase {
                GamePhase::Action { action_state } => action_state.current_player_index,
                _ => return false,
            }];
            player.buyers.iter().any(|b| b.sell_card == *sell_card && can_afford_sell_card(player, &b.sell_card))
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
        Choice::DestroyAndMoveToDraftPool { card, target } => {
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
                    let area = player.workshop_cards.union(player.workshopped_cards);
                    area.iter().any(|id| state.card_lookup[id as usize] == *target_card)
                }
            }
        }
        Choice::TakeBuyer { sell_card } => {
            matches!(state.phase, GamePhase::Buyers { .. })
                && state
                    .sell_card_display
                    .iter()
                    .any(|c| c.sell_card == *sell_card)
        }
        Choice::DrawBuyer => {
            matches!(state.phase, GamePhase::Buyers { .. }) && !state.sell_card_deck.is_empty()
        }
    }
}

/// What the acting player may claim: one entry per distinct face-up sell card,
/// plus the top of the deck when there is one.
///
/// Deduplicating by card type keeps the tree from branching on which of two
/// identical cards was taken, exactly as draft picks do. The phase only ever
/// seats a player who has an empty slot, so there is no "decline" — but both
/// piles can run dry, and then there is nothing to enumerate and the phase
/// ends without seating anyone.
fn enumerate_buyer_choices(state: &GameState, choices: &mut Vec<Choice>) {
    let display = &state.sell_card_display;
    for (i, instance) in display.iter().enumerate() {
        // At most five entries, so a scan beats any bookkeeping.
        if display[..i].iter().any(|c| c.sell_card == instance.sell_card) {
            continue;
        }
        choices.push(Choice::TakeBuyer {
            sell_card: instance.sell_card,
        });
    }
    if !state.sell_card_deck.is_empty() {
        choices.push(Choice::DrawBuyer);
    }
}
