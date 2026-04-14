use crate::action_phase::*;
use crate::draft_phase::player_pick;
use crate::types::{Ability, Card, Choice, GamePhase, GameState, SellCard};
use crate::unordered_cards::UnorderedCards;
use rand::Rng;

/// Find the first card instance ID matching a card type in a card set.
fn find_card_instance(state: &GameState, card: &Card, cards: &UnorderedCards) -> u32 {
    for id in cards.iter() {
        if state.card_lookup[id as usize] == *card {
            return id as u32;
        }
    }
    let contents: Vec<(u8, Card)> = cards.iter().map(|id| (id, state.card_lookup[id as usize])).collect();
    panic!(
        "Card type {:?} not found in card set (round {}, phase {:?}, set contents: {:?})",
        card, state.round, phase_name(&state.phase), contents
    );
}

fn phase_name(phase: &GamePhase) -> &'static str {
    match phase {
        GamePhase::Draw => "Draw",
        GamePhase::Draft { .. } => "Draft",
        GamePhase::Action { .. } => "Action",
        GamePhase::GameOver => "GameOver",
    }
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

/// Find the first sell card instance ID matching a sell card type in the sell card display.
fn find_sell_card_instance(state: &GameState, sell_card: &SellCard) -> u32 {
    for sell_card_instance in state.sell_card_display.iter() {
        if sell_card_instance.sell_card == *sell_card {
            return sell_card_instance.instance_id;
        }
    }
    panic!("Sell card type {:?} not found in sell card display", sell_card);
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
            player_pick(state, card_instance_id, rng);
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
        Choice::SelectSellCard { sell_card } => {
            let sell_card_instance_id = find_sell_card_instance(state, sell_card);
            resolve_select_sell_card(state, sell_card_instance_id, rng);
        }
        Choice::GainSecondary { color } => {
            resolve_gain_color(state, *color, rng);
        }
        Choice::GainPrimary { color } => {
            resolve_gain_color(state, *color, rng);
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
        Choice::DestroyAndSell { card, sell_card } => {
            let card_instance_id = get_drafted_card_instance(state, card);
            let sell_card_instance_id = find_sell_card_instance(state, sell_card);
            destroy_drafted_card(state, card_instance_id, rng);
            resolve_select_sell_card(state, sell_card_instance_id, rng);
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
        Choice::SelectMoveToDrafted { card } => {
            get_action_state_mut(state).ability_stack.pop();
            let player_index = get_action_state(state).current_player_index;
            let id = find_card_instance(state, card, &state.players[player_index].workshop_cards);
            state.players[player_index].workshop_cards.remove(id as u8);
            state.players[player_index].drafted_cards.insert(id as u8);
            process_ability_stack(state, rng);
        }
        Choice::SkipMoveToDrafted => {
            get_action_state_mut(state).ability_stack.pop();
            process_ability_stack(state, rng);
        }
        Choice::SelectMoveToWorkshop { card } => {
            get_action_state_mut(state).ability_stack.pop();
            let player_index = get_action_state(state).current_player_index;
            let id = find_card_instance(state, card, &state.players[player_index].drafted_cards);
            state.players[player_index].drafted_cards.remove(id as u8);
            state.players[player_index].workshop_cards.insert(id as u8);
            process_ability_stack(state, rng);
        }
        Choice::SkipMoveToWorkshop => {
            get_action_state_mut(state).ability_stack.pop();
            process_ability_stack(state, rng);
        }
        Choice::DeferredMoveToDraft { .. } => {
            // Behaviorally identical to DestroyDrawnCards { card: None }: the
            // engine simply pops the DestroyCards ability and leaves the card
            // in the workshop. The `card` field is carried only so the UI log
            // can name which card was visually moved to the draft pool.
            resolve_destroy_cards(state, UnorderedCards::new(), rng);
        }
        Choice::DestroyWorkshopCardDeferred { card } => {
            let player_index = get_action_state(state).current_player_index;
            let workshop = state.players[player_index].workshop_cards;
            let card_instance_id = find_card_instance(state, card, &workshop);
            destroy_workshop_card_and_trigger(state, card_instance_id, rng);
        }
    }
}
