mod destroy;
mod mix_sequences;
mod multiset;

use crate::action_phase::{
    can_afford_sell_card, can_buy_with_ducat, can_sell_to_any_buyer, for_each_unique_card_type,
};
use crate::apply_choice::resolve_card_types_to_ids;
use crate::colors::{can_mix, can_supply_mix, perform_mix_unchecked, SECONDARIES};
use crate::types::*;
use smallvec::SmallVec;

use destroy::enumerate_destroy_choices;
use mix_sequences::enumerate_mix_sequences;
use multiset::{count_card_types, enumerate_multiset_subsets, enumerate_multiset_subsets_exact};

/// Whether sub-maximum workshop subsets can be skipped during enumeration.
///
/// Nothing enters the workshop mid-turn, so holding picks back only pays off
/// when a drafted card could still move one of the cards left unworkshopped
/// into the draft pool. When none can, workshopping fewer cards than allowed
/// is strictly worse and the smaller subsets are dead branches in the search
/// tree.
pub(crate) fn should_force_max_workshop(state: &GameState, player: &PlayerState) -> bool {
    state.force_max_workshop
        && !player.drafted_cards.iter().any(|id| {
            state.card_lookup[id as usize]
                .destroy_abilities()
                .contains(&Ability::MoveToDraftPool)
        })
}

// ── Choice enumeration ──

/// What `player_index` may choose now. The buyers phase and the draft seat one
/// player at a time and offer nothing to anyone else; in the action phase
/// everyone still taking their turn has choices.
pub fn enumerate_choices_into(state: &GameState, player_index: usize, choices: &mut Vec<Choice>) {
    choices.clear();
    match &state.phase {
        GamePhase::Buyers { buyers_state } => {
            if buyers_state.current_player_index == player_index {
                enumerate_buyer_choices(state, choices);
            }
        }
        GamePhase::Draft { draft_state } => {
            if draft_state.current_player_index == player_index {
                let hand = draft_state.hands[player_index];
                for_each_unique_card_type(&hand, &state.card_lookup, |card| {
                    choices.push(Choice::DraftPick { card });
                });
            }
        }
        GamePhase::Action { action_state } => {
            let turn = &action_state.turns[player_index];
            if turn.ended {
                return;
            }
            let player = &state.players[player_index];

            match turn.ability_stack.last() {
                None => {
                    for_each_unique_card_type(&player.drafted_cards, &state.card_lookup, |card| {
                        enumerate_destroy_choices(state, player, card, choices);
                    });
                    for purchase in ALL_DUCAT_PURCHASES {
                        if can_buy_with_ducat(player, purchase) {
                            choices.push(Choice::SpendDucat { purchase });
                        }
                    }
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
                    if player.workshop_cards.is_empty() {
                        choices.push(Choice::MoveToDraftPool { card: None });
                    } else {
                        for_each_unique_card_type(&player.workshop_cards, &state.card_lookup, |card| {
                            choices.push(Choice::MoveToDraftPool { card: Some(card) });
                        });
                    }
                }
                Some(Ability::MixColors { count }) => {
                    enumerate_mix_sequences(
                        &player.color_wheel,
                        player.wild_primaries,
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
                // Instant abilities (GainDucats, GainWildPrimaries, GainBoats)
                // should never be on top when waiting for a choice — they get
                // processed immediately.
                Some(_) => {}
            }
        }
        _ => {}
    }
}

pub fn enumerate_choices(state: &GameState, player_index: usize) -> Vec<Choice> {
    let mut choices = Vec::new();
    enumerate_choices_into(state, player_index, &mut choices);
    choices
}

// ── Choice availability ──

/// Whether `mixes` can be made one after another, within the `granted` number
/// of mixes.
fn mix_sequence_is_valid(player: &PlayerState, mixes: &[(Color, Color)], granted: u32) -> bool {
    if mixes.len() as u32 > granted {
        return false;
    }
    let mut wheel = player.color_wheel.clone();
    let mut wild_primaries = player.wild_primaries;
    for &(a, b) in mixes {
        if !can_mix(a, b) || !can_supply_mix(&wheel, wild_primaries, a, b) {
            return false;
        }
        perform_mix_unchecked(&mut wheel, &mut wild_primaries, a, b);
    }
    true
}

/// The player's pending abilities, while they are still taking their action
/// turn.
fn open_turn_stack(state: &GameState, player_index: usize) -> Option<&AbilityStack> {
    match &state.phase {
        GamePhase::Action { action_state } => {
            let turn = &action_state.turns[player_index];
            (!turn.ended).then_some(&turn.ability_stack)
        }
        _ => None,
    }
}

/// The ability the player must answer next, if they are mid-turn and one is
/// waiting.
fn pending_ability(state: &GameState, player_index: usize) -> Option<Ability> {
    open_turn_stack(state, player_index).and_then(|stack| stack.last().copied())
}

/// Whether `target` is a legal answer to Move to Draft Pool. The move is
/// mandatory when the workshop has a card to move, so `None` stands only for
/// the fizzle.
fn move_target_is_valid(state: &GameState, player: &PlayerState, target: &Option<Card>) -> bool {
    match target {
        None => player.workshop_cards.is_empty(),
        Some(card) => player
            .workshop_cards
            .iter()
            .any(|id| state.card_lookup[id as usize] == *card),
    }
}

/// Check common preconditions for all destroy-drafted-card variants: the
/// player is between abilities and holds the card in their draft pool.
fn check_destroy_preconditions(state: &GameState, player_index: usize, card: &Card) -> bool {
    open_turn_stack(state, player_index).is_some_and(|stack| stack.is_empty())
        && state.players[player_index]
            .drafted_cards
            .iter()
            .any(|id| state.card_lookup[id as usize] == *card)
}

pub fn check_choice_available(state: &GameState, player_index: usize, choice: &Choice) -> bool {
    let player = &state.players[player_index];
    match choice {
        Choice::DraftPick { card } => {
            if let GamePhase::Draft { ref draft_state } = state.phase {
                draft_state.current_player_index == player_index
                    && draft_state.hands[player_index]
                        .iter()
                        .any(|id| state.card_lookup[id as usize] == *card)
            } else {
                false
            }
        }
        Choice::DestroyDraftedCard { card } => {
            if !check_destroy_preconditions(state, player_index, card) {
                return false;
            }
            match card.destroy_abilities()[0] {
                Ability::Sell => !can_sell_to_any_buyer(player),
                Ability::MixColors { .. } => false,
                Ability::Workshop { .. } => false,
                Ability::MoveToDraftPool => false,
                _ => true,
            }
        }
        Choice::EndTurn => open_turn_stack(state, player_index).is_some_and(|stack| stack.is_empty()),
        Choice::Workshop { card_types } => {
            matches!(pending_ability(state, player_index), Some(Ability::Workshop { .. }))
                && !card_types.is_empty()
                && resolve_card_types_to_ids(card_types, &player.workshop_cards, &state.card_lookup).is_some()
        }
        Choice::SkipWorkshop => {
            matches!(pending_ability(state, player_index), Some(Ability::Workshop { .. }))
        }
        Choice::MoveToDraftPool { card } => {
            if !matches!(pending_ability(state, player_index), Some(Ability::MoveToDraftPool)) {
                return false;
            }
            move_target_is_valid(state, player, card)
        }
        Choice::SelectSellCard { sell_card } => {
            matches!(pending_ability(state, player_index), Some(Ability::Sell))
                && player.buyers.iter().any(|b| b.sell_card == *sell_card && can_afford_sell_card(player, &b.sell_card))
        }
        Choice::GainSecondary { color } => {
            matches!(pending_ability(state, player_index), Some(Ability::GainSecondary))
                && SECONDARIES.contains(color)
        }
        Choice::MixAll { mixes } => match pending_ability(state, player_index) {
            Some(Ability::MixColors { count }) => mix_sequence_is_valid(player, mixes, count),
            _ => false,
        },
        Choice::DestroyAndMix { card, mixes } => {
            if !check_destroy_preconditions(state, player_index, card) {
                return false;
            }
            let Ability::MixColors { count } = card.destroy_abilities()[0] else {
                return false;
            };
            mix_sequence_is_valid(player, mixes, count)
        }
        Choice::DestroyAndSell { card, sell_card } => {
            check_destroy_preconditions(state, player_index, card)
                && player.buyers.iter().any(|b| b.sell_card == *sell_card && can_afford_sell_card(player, &b.sell_card))
        }
        Choice::DestroyAndWorkshop { card, workshop_cards } => {
            check_destroy_preconditions(state, player_index, card)
                && (workshop_cards.is_empty()
                    || resolve_card_types_to_ids(workshop_cards, &player.workshop_cards, &state.card_lookup).is_some())
        }
        Choice::DestroyAndMoveToDraftPool { card, target } => {
            if !check_destroy_preconditions(state, player_index, card) {
                return false;
            }
            move_target_is_valid(state, player, target)
        }
        Choice::SpendDucat { purchase } => {
            open_turn_stack(state, player_index).is_some_and(|stack| stack.is_empty())
                && can_buy_with_ducat(player, *purchase)
        }
        Choice::TakeBuyer { sell_card } => {
            matches!(&state.phase, GamePhase::Buyers { buyers_state } if buyers_state.current_player_index == player_index)
                && state
                    .sell_card_display
                    .iter()
                    .any(|c| c.sell_card == *sell_card)
        }
        Choice::DrawBuyer => {
            matches!(&state.phase, GamePhase::Buyers { buyers_state } if buyers_state.current_player_index == player_index)
                && !state.sell_card_deck.is_empty()
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
