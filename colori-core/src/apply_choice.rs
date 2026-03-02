use crate::action_phase::*;
use crate::cleanup_phase::resolve_keep_workshop_cards;
use crate::draft_phase::player_pick;
use crate::types::{Ability, Card, Choice, GamePhase, GameState};
use crate::unordered_cards::UnorderedCards;
use rand::Rng;

/// Find the first card instance ID matching a card type in a card set.
fn find_card_instance(state: &GameState, card: &Card, cards: &UnorderedCards) -> u32 {
    for id in cards.iter() {
        if state.card_lookup[id as usize] == *card {
            return id as u32;
        }
    }
    panic!("Card type {:?} not found in card set", card);
}

/// Get the instance ID of a drafted card belonging to the current action-phase player.
fn get_drafted_card_instance(state: &GameState, card: &Card) -> u32 {
    let drafted = match &state.phase {
        GamePhase::Action { action_state } => {
            state.players[action_state.current_player_index].drafted_cards
        }
        _ => panic!("Expected action phase"),
    };
    find_card_instance(state, card, &drafted)
}

/// Find the first buyer instance ID matching a buyer type in the buyer display.
fn find_buyer_instance(state: &GameState, buyer: &crate::types::BuyerCard) -> u32 {
    for buyer_instance in state.buyer_display.iter() {
        if buyer_instance.buyer == *buyer {
            return buyer_instance.instance_id;
        }
    }
    panic!("Buyer type {:?} not found in buyer display", buyer);
}

/// Resolve a list of card types to instance IDs from a card set, tracking used IDs
/// to avoid duplicates. Returns None if any card type can't be found.
pub(crate) fn resolve_card_types_to_ids(
    card_types: &[Card],
    available: &UnorderedCards,
    card_lookup: &[Card; 256],
) -> Option<UnorderedCards> {
    let mut ids = UnorderedCards::new();
    let mut used = UnorderedCards::new();
    for &ct in card_types.iter() {
        let mut found = false;
        for id in available.iter() {
            if !used.contains(id) && card_lookup[id as usize] == ct {
                ids.insert(id);
                used.insert(id);
                found = true;
                break;
            }
        }
        if !found {
            return None;
        }
    }
    Some(ids)
}

pub fn apply_choice<R: Rng>(state: &mut GameState, choice: &Choice, rng: &mut R) {
    match choice {
        Choice::DraftPick { card } => {
            let hand = match &state.phase {
                GamePhase::Draft { draft_state } => {
                    draft_state.hands[draft_state.current_player_index]
                }
                _ => panic!("Expected draft phase"),
            };
            let card_instance_id = find_card_instance(state, card, &hand);
            player_pick(state, card_instance_id);
        }
        Choice::DestroyDraftedCard { card } => {
            let card_instance_id = get_drafted_card_instance(state, card);
            destroy_drafted_card(state, card_instance_id, rng);
        }
        Choice::EndTurn => {
            end_player_turn(state, rng);
        }
        Choice::Workshop { card_types } => {
            let workshop = match &state.phase {
                GamePhase::Action { action_state } => {
                    state.players[action_state.current_player_index].workshop_cards
                }
                _ => panic!("Expected action phase"),
            };
            let card_instance_ids = resolve_card_types_to_ids(card_types, &workshop, &state.card_lookup)
                .expect("Card types not found in workshop");
            resolve_workshop_choice(state, card_instance_ids, rng);
        }
        Choice::SkipWorkshop => {
            skip_workshop(state, rng);
        }
        Choice::DestroyDrawnCards { card } => {
            let mut selected = UnorderedCards::new();
            if let Some(card) = card {
                let workshop = match &state.phase {
                    GamePhase::Action { action_state } => {
                        state.players[action_state.current_player_index].workshop_cards
                    }
                    _ => panic!("Expected action phase"),
                };
                for id in workshop.iter() {
                    if state.card_lookup[id as usize] == *card {
                        selected.insert(id);
                        break;
                    }
                }
            }
            resolve_destroy_cards(state, selected, rng);
        }
        Choice::SelectBuyer { buyer } => {
            let buyer_instance_id = find_buyer_instance(state, buyer);
            resolve_select_buyer(state, buyer_instance_id, rng);
        }
        Choice::GainSecondary { color } => {
            resolve_gain_secondary(state, *color, rng);
        }
        Choice::GainPrimary { color } => {
            resolve_gain_primary(state, *color, rng);
        }
        Choice::MixAll { mixes } => {
            for &(a, b) in mixes.iter() {
                resolve_mix_colors(state, a, b, rng);
            }
            // Skip any remaining mixes not used
            if let GamePhase::Action { ref action_state } = state.phase {
                if matches!(action_state.ability_stack.last(), Some(Ability::MixColors { .. })) {
                    skip_mix(state, rng);
                }
            }
        }
        Choice::SwapTertiary { lose, gain } => {
            resolve_choose_tertiary_to_lose(state, *lose);
            resolve_choose_tertiary_to_gain(state, *gain, rng);
        }
        Choice::DestroyAndMix { card, mixes } => {
            let card_instance_id = get_drafted_card_instance(state, card);
            destroy_drafted_card(state, card_instance_id, rng);
            for &(a, b) in mixes.iter() {
                resolve_mix_colors(state, a, b, rng);
            }
            if let GamePhase::Action { ref action_state } = state.phase {
                if matches!(action_state.ability_stack.last(), Some(Ability::MixColors { .. })) {
                    skip_mix(state, rng);
                }
            }
        }
        Choice::DestroyAndSell { card, buyer } => {
            let card_instance_id = get_drafted_card_instance(state, card);
            let buyer_instance_id = find_buyer_instance(state, buyer);
            destroy_drafted_card(state, card_instance_id, rng);
            resolve_select_buyer(state, buyer_instance_id, rng);
        }
        Choice::DestroyAndWorkshop { card, workshop_cards } => {
            let card_instance_id = get_drafted_card_instance(state, card);
            destroy_drafted_card(state, card_instance_id, rng);
            if workshop_cards.is_empty() {
                skip_workshop(state, rng);
            } else {
                let workshop = match &state.phase {
                    GamePhase::Action { action_state } => {
                        state.players[action_state.current_player_index].workshop_cards
                    }
                    _ => panic!("Expected action phase"),
                };
                let card_instance_ids = resolve_card_types_to_ids(workshop_cards, &workshop, &state.card_lookup)
                    .expect("Card types not found in workshop");
                resolve_workshop_choice(state, card_instance_ids, rng);
            }
        }
        Choice::DestroyAndDestroyCards { card, target } => {
            let card_instance_id = get_drafted_card_instance(state, card);
            destroy_drafted_card(state, card_instance_id, rng);
            let mut selected = UnorderedCards::new();
            if let Some(target_card) = target {
                let workshop = match &state.phase {
                    GamePhase::Action { action_state } => {
                        state.players[action_state.current_player_index].workshop_cards
                    }
                    _ => panic!("Expected action phase"),
                };
                for id in workshop.iter() {
                    if state.card_lookup[id as usize] == *target_card {
                        selected.insert(id);
                        break;
                    }
                }
            }
            resolve_destroy_cards(state, selected, rng);
        }
        Choice::KeepWorkshopCards { card_types } => {
            let workshop = match &state.phase {
                GamePhase::Cleanup { cleanup_state } => {
                    state.players[cleanup_state.current_player_index].workshop_cards
                }
                _ => panic!("Expected cleanup phase"),
            };
            let card_instance_ids = resolve_card_types_to_ids(card_types, &workshop, &state.card_lookup)
                .expect("Card types not found in workshop");
            resolve_keep_workshop_cards(state, card_instance_ids, rng);
        }
    }
}
