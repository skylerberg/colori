use crate::action_phase::*;
use crate::draft_phase::player_pick;
use crate::types::{Ability, Choice, GamePhase, GameState};
use crate::unordered_cards::UnorderedCards;
use rand::Rng;

/// Find the first card instance ID matching a card type in a card set.
fn find_card_instance(state: &GameState, card: &crate::types::Card, cards: &UnorderedCards) -> u32 {
    for id in cards.iter() {
        if state.card_lookup[id as usize] == *card {
            return id as u32;
        }
    }
    panic!("Card type {:?} not found in card set", card);
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

/// Find instance IDs for a list of card types, tracking used IDs to avoid duplicates.
fn find_card_instances(
    state: &GameState,
    card_types: &[crate::types::Card],
    cards: &UnorderedCards,
) -> UnorderedCards {
    let mut ids = UnorderedCards::new();
    let mut used = UnorderedCards::new();
    for ct in card_types.iter() {
        for id in cards.iter() {
            if !used.contains(id) && state.card_lookup[id as usize] == *ct {
                ids.insert(id);
                used.insert(id);
                break;
            }
        }
    }
    ids
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
            let drafted = match &state.phase {
                GamePhase::Action { action_state } => {
                    state.players[action_state.current_player_index].drafted_cards
                }
                _ => panic!("Expected action phase"),
            };
            let card_instance_id = find_card_instance(state, card, &drafted);
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
            let card_instance_ids = find_card_instances(state, card_types, &workshop);
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
        Choice::DestroyAndMixAll { card, mixes } => {
            let drafted = match &state.phase {
                GamePhase::Action { action_state } => {
                    state.players[action_state.current_player_index].drafted_cards
                }
                _ => panic!("Expected action phase"),
            };
            let card_instance_id = find_card_instance(state, card, &drafted);
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
            let drafted = match &state.phase {
                GamePhase::Action { action_state } => {
                    state.players[action_state.current_player_index].drafted_cards
                }
                _ => panic!("Expected action phase"),
            };
            let card_instance_id = find_card_instance(state, card, &drafted);
            let buyer_instance_id = find_buyer_instance(state, buyer);
            destroy_drafted_card(state, card_instance_id, rng);
            resolve_select_buyer(state, buyer_instance_id, rng);
        }
        Choice::KeepWorkshopCards { card_types } => {
            let workshop = match &state.phase {
                GamePhase::Cleanup { cleanup_state } => {
                    state.players[cleanup_state.current_player_index].workshop_cards
                }
                _ => panic!("Expected cleanup phase"),
            };
            let card_instance_ids = find_card_instances(state, card_types, &workshop);
            resolve_keep_workshop_cards(state, card_instance_ids, rng);
        }
    }
}
