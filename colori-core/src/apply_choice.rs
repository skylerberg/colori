use crate::action_phase::*;
use crate::buyers_phase::{draw_buyer, take_buyer};
use crate::draft_phase::player_pick;
use crate::types::{Ability, Card, Choice, Color, GamePhase, GameState, Buyer};
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
        GamePhase::Buyers { .. } => "Buyers",
        GamePhase::Draft { .. } => "Draft",
        GamePhase::Action { .. } => "Action",
        GamePhase::GameOver => "GameOver",
    }
}

fn get_top_card_instance(state: &GameState, player_index: usize, card: &Card) -> u32 {
    find_card_instance(state, card, &state.players[player_index].top_cards)
}

/// Find the first instance of a buyer card among the player's buyers.
fn find_buyer_instance(state: &GameState, player_index: usize, buyer: &Buyer) -> u32 {
    for buyer_instance in state.players[player_index].buyers.iter() {
        if buyer_instance.buyer == *buyer {
            return buyer_instance.instance_id;
        }
    }
    panic!("Buyer card type {:?} not found in the player's buyers", buyer);
}

/// Find the first instance of a buyer card in the shared display.
fn find_display_buyer_instance(state: &GameState, buyer: &Buyer) -> u32 {
    for buyer_instance in state.buyer_display.iter() {
        if buyer_instance.buyer == *buyer {
            return buyer_instance.instance_id;
        }
    }
    panic!("Buyer card type {:?} not found in buyer display", buyer);
}

/// The workshop card a Lift takes; `None` when it fizzles.
fn lift_target(state: &GameState, player_index: usize, card: &Option<Card>) -> Option<u8> {
    card.map(|card| {
        find_card_instance(state, &card, &state.players[player_index].workshop_cards) as u8
    })
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

/// Make `mixes` against the mix ability on top of the player's stack,
/// forfeiting any mixes it grants beyond them.
fn resolve_mix_sequence(state: &mut GameState, player_index: usize, mixes: &[(Color, Color)]) {
    let granted = match ability_stack(state, player_index).last() {
        Some(Ability::MixColors { count }) => *count,
        // The ability is gone because it could mix nothing, which is what an
        // empty sequence asks for too. A sequence that mixes something with no
        // ability to mix it is a caller's mistake.
        _ => {
            assert!(mixes.is_empty(), "Mixing with no pending mix ability");
            return;
        }
    };
    for &(a, b) in mixes {
        resolve_mix_colors(state, player_index, a, b);
    }
    // Checked against the count rather than the stack: once this ability is
    // used up, a mix left on top belongs to something else.
    if (mixes.len() as u32) < granted {
        skip_mix(state, player_index);
    }
}

fn resolve_workshop_card_types(state: &mut GameState, player_index: usize, card_types: &[Card]) {
    let card_instance_ids = resolve_card_types_to_ids(
        card_types,
        &state.players[player_index].workshop_cards,
        &state.card_lookup,
    )
    .expect("Card types not found in workshop");
    resolve_workshop_choice(state, player_index, card_instance_ids);
}

/// Apply `player_index`'s choice. The buyers phase and the draft seat one
/// player at a time, and it must be theirs; in the action phase anyone still
/// taking their turn may choose.
pub fn apply_choice<R: Rng>(state: &mut GameState, player_index: usize, choice: &Choice, rng: &mut R) {
    match &state.phase {
        GamePhase::Buyers { buyers_state } => assert_eq!(
            buyers_state.current_player_index, player_index,
            "Player {player_index} claimed a buyer out of turn"
        ),
        GamePhase::Draft { draft_state } => assert_eq!(
            draft_state.current_player_index, player_index,
            "Player {player_index} drafted out of turn"
        ),
        GamePhase::Action { action_state } => assert!(
            !action_state.turns[player_index].ended,
            "Player {player_index} acted after ending their turn"
        ),
        GamePhase::GameOver => {}
    }

    match choice {
        Choice::DraftPick { card } => {
            let hand = match &state.phase {
                GamePhase::Draft { draft_state } => draft_state.hands[player_index],
                _ => panic!("Expected draft phase"),
            };
            let card_instance_id = find_card_instance(state, card, &hand);
            player_pick(state, card_instance_id, rng);
        }
        Choice::TrashTopCard { card } => {
            let card_instance_id = get_top_card_instance(state, player_index, card);
            trash_top_card(state, player_index, card_instance_id);
        }
        Choice::KeepCard { card } => {
            let card_instance_id = get_top_card_instance(state, player_index, card);
            resolve_keep_card(state, player_index, card_instance_id);
        }
        Choice::EndTurn { keep_remaining } => {
            end_player_turn(state, player_index, *keep_remaining, rng);
        }
        Choice::Workshop { card_types } => {
            resolve_workshop_card_types(state, player_index, card_types);
        }
        Choice::SkipWorkshop => {
            skip_workshop(state, player_index);
        }
        Choice::LiftCard { card } => {
            let target = lift_target(state, player_index, card);
            resolve_lift_card(state, player_index, target);
        }
        Choice::SellToBuyer { buyer } => {
            let buyer_instance_id = find_buyer_instance(state, player_index, buyer);
            resolve_sell_to_buyer(state, player_index, buyer_instance_id);
        }
        Choice::GainSecondary { color } => {
            resolve_gain_color(state, player_index, *color);
        }
        Choice::MixAll { mixes } => {
            resolve_mix_sequence(state, player_index, mixes);
        }
        Choice::TrashAndMix { card, mixes } => {
            let card_instance_id = get_top_card_instance(state, player_index, card);
            trash_top_card(state, player_index, card_instance_id);
            resolve_mix_sequence(state, player_index, mixes);
        }
        Choice::TrashAndSell { card, buyer } => {
            let card_instance_id = get_top_card_instance(state, player_index, card);
            let buyer_instance_id = find_buyer_instance(state, player_index, buyer);
            trash_top_card(state, player_index, card_instance_id);
            resolve_sell_to_buyer(state, player_index, buyer_instance_id);
        }
        Choice::TrashAndWorkshop { card, workshop_cards } => {
            let card_instance_id = get_top_card_instance(state, player_index, card);
            trash_top_card(state, player_index, card_instance_id);
            // With nothing in the workshop the ability has already fizzled, and
            // whatever is on top now is not this choice's to skip.
            let pending = matches!(
                ability_stack(state, player_index).last(),
                Some(Ability::Workshop { .. })
            );
            if !pending {
                assert!(workshop_cards.is_empty(), "No pending workshop choice");
            } else if workshop_cards.is_empty() {
                skip_workshop(state, player_index);
            } else {
                resolve_workshop_card_types(state, player_index, workshop_cards);
            }
        }
        Choice::TrashAndLift { card, target } => {
            let card_instance_id = get_top_card_instance(state, player_index, card);
            trash_top_card(state, player_index, card_instance_id);
            let target = lift_target(state, player_index, target);
            resolve_lift_card(state, player_index, target);
        }
        Choice::SpendGoldCoin { purchase } => {
            resolve_spend_gold_coin(state, player_index, *purchase);
        }
        Choice::ClaimBuyer { buyer } => {
            let instance_id = find_display_buyer_instance(state, buyer);
            take_buyer(state, instance_id, rng);
        }
        Choice::DrawBuyer => {
            draw_buyer(state, rng);
        }
    }
}
