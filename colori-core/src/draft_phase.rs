use crate::action_phase::initialize_action_phase;
use crate::deck_utils::shuffle;
use crate::types::{CardInstance, DraftState, GamePhase, GameState};
use rand::Rng;

pub fn initialize_draft<R: Rng>(state: &mut GameState, rng: &mut R) {
    let num_players = state.players.len();
    let mut hands: Vec<Vec<CardInstance>> = Vec::with_capacity(num_players);

    for _ in 0..num_players {
        let mut hand = Vec::with_capacity(5);
        for _ in 0..5 {
            if state.draft_deck.is_empty() {
                if !state.destroyed_pile.is_empty() {
                    state.draft_deck = shuffle(&state.destroyed_pile, rng);
                    state.destroyed_pile.clear();
                } else {
                    break;
                }
            }
            if let Some(card) = state.draft_deck.pop() {
                hand.push(card);
            }
        }
        hands.push(hand);
    }

    if hands.iter().any(|h| h.is_empty()) {
        for hand in hands {
            for card in hand {
                state.destroyed_pile.push(card);
            }
        }
        initialize_action_phase(state);
        return;
    }

    let draft_state = DraftState {
        pick_number: 0,
        current_player_index: ((state.round - 1) as usize) % num_players,
        hands,
        direction: if state.round % 2 == 1 { 1 } else { -1 },
        waiting_for_pass: false,
    };

    state.phase = GamePhase::Draft { draft_state };
}

pub fn player_pick(state: &mut GameState, card_instance_id: u32) {
    let num_players = state.players.len();
    let starting_player = ((state.round - 1) as usize) % num_players;

    // Step 1: Extract the card from the draft hand and get player index.
    // We temporarily take the phase out of state to split the borrows.
    let mut phase = std::mem::replace(&mut state.phase, GamePhase::Draw);
    let _player_index = match &mut phase {
        GamePhase::Draft { draft_state } => {
            let pi = draft_state.current_player_index;
            let hand = &mut draft_state.hands[pi];
            let card_index = hand
                .iter()
                .position(|c| c.instance_id == card_instance_id)
                .expect("Card not found in player's draft hand");
            let card = hand.swap_remove(card_index);
            state.players[pi].drafted_cards.push(card);
            draft_state.current_player_index = (pi + 1) % num_players;
            pi
        }
        _ => panic!("Expected draft phase"),
    };
    // Put the phase back
    state.phase = phase;

    let should_advance = match &state.phase {
        GamePhase::Draft { draft_state } => draft_state.current_player_index == starting_player,
        _ => false,
    };

    if should_advance {
        advance_draft(state);
    } else {
        if let GamePhase::Draft { ref mut draft_state } = state.phase {
            draft_state.waiting_for_pass = true;
        }
    }
}

pub fn advance_draft(state: &mut GameState) {
    let num_players = state.players.len();
    let round = state.round;

    let (pick_number, any_empty) = {
        let ds = match &mut state.phase {
            GamePhase::Draft { draft_state } => draft_state,
            _ => panic!("Expected draft phase"),
        };

        if ds.direction == 1 {
            ds.hands.rotate_right(1);
        } else {
            ds.hands.rotate_left(1);
        }

        ds.pick_number += 1;
        (ds.pick_number, ds.hands.iter().any(|h| h.is_empty()))
    };

    if pick_number >= 4 || any_empty {
        let remaining_hands: Vec<Vec<CardInstance>> = match &mut state.phase {
            GamePhase::Draft { draft_state } => draft_state.hands.drain(..).collect(),
            _ => vec![],
        };
        for hand in remaining_hands {
            for card in hand {
                state.destroyed_pile.push(card);
            }
        }
        initialize_action_phase(state);
    } else {
        if let GamePhase::Draft { ref mut draft_state } = state.phase {
            draft_state.current_player_index = ((round - 1) as usize) % num_players;
            draft_state.waiting_for_pass = true;
        }
    }
}

pub fn simultaneous_pick(state: &mut GameState, player_index: usize, card_instance_id: u32) {
    let mut phase = std::mem::replace(&mut state.phase, GamePhase::Draw);
    match &mut phase {
        GamePhase::Draft { draft_state } => {
            let hand = &mut draft_state.hands[player_index];
            let card_index = hand
                .iter()
                .position(|c| c.instance_id == card_instance_id)
                .expect("Card not found in player's draft hand");
            let card = hand.swap_remove(card_index);
            state.players[player_index].drafted_cards.push(card);
        }
        _ => panic!("Expected draft phase"),
    }
    state.phase = phase;
}

pub fn confirm_pass(state: &mut GameState) {
    if let GamePhase::Draft { ref mut draft_state } = state.phase {
        draft_state.waiting_for_pass = false;
    } else {
        panic!("Expected draft phase");
    }
}
