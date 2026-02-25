use crate::action_phase::initialize_action_phase;
use crate::types::{DraftState, GamePhase, GameState, MAX_PLAYERS};
use crate::unordered_cards::UnorderedCards;
use rand::Rng;

pub fn initialize_draft<R: Rng>(state: &mut GameState, rng: &mut R) {
    let num_players = state.players.len();
    let mut hands = [UnorderedCards::new(); MAX_PLAYERS];

    for i in 0..num_players {
        for _ in 0..5 {
            if state.draft_deck.is_empty() {
                if !state.destroyed_pile.is_empty() {
                    state.draft_deck = state.destroyed_pile;
                    state.destroyed_pile = UnorderedCards::new();
                } else {
                    break;
                }
            }
            if let Some(id) = state.draft_deck.draw(rng) {
                hands[i].insert(id);
            }
        }
    }

    if (0..num_players).any(|i| hands[i].is_empty()) {
        for i in 0..num_players {
            state.destroyed_pile = state.destroyed_pile.union(hands[i]);
        }
        initialize_action_phase(state);
        return;
    }

    let draft_state = DraftState {
        pick_number: 0,
        current_player_index: ((state.round - 1) as usize) % num_players,
        hands,
        num_hands: num_players,
        direction: if state.round % 2 == 1 { 1 } else { -1 },
        waiting_for_pass: false,
    };

    state.phase = GamePhase::Draft { draft_state };
}

pub fn player_pick(state: &mut GameState, card_instance_id: u32) {
    let num_players = state.players.len();
    let starting_player = ((state.round - 1) as usize) % num_players;
    let id = card_instance_id as u8;

    let mut phase = std::mem::replace(&mut state.phase, GamePhase::Draw);
    let _player_index = match &mut phase {
        GamePhase::Draft { draft_state } => {
            let pi = draft_state.current_player_index;
            assert!(
                draft_state.hands[pi].contains(id),
                "Card not found in player's draft hand"
            );
            draft_state.hands[pi].remove(id);
            state.players[pi].drafted_cards.insert(id);
            draft_state.current_player_index = (pi + 1) % num_players;
            pi
        }
        _ => panic!("Expected draft phase"),
    };
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

        let n = ds.num_hands;
        if ds.direction == 1 {
            let last = ds.hands[n - 1];
            for i in (1..n).rev() {
                ds.hands[i] = ds.hands[i - 1];
            }
            ds.hands[0] = last;
        } else {
            let first = ds.hands[0];
            for i in 0..n - 1 {
                ds.hands[i] = ds.hands[i + 1];
            }
            ds.hands[n - 1] = first;
        }

        ds.pick_number += 1;
        (
            ds.pick_number,
            (0..n).any(|i| ds.hands[i].is_empty()),
        )
    };

    if pick_number >= 4 || any_empty {
        if let GamePhase::Draft { ref draft_state } = state.phase {
            for i in 0..draft_state.num_hands {
                state.destroyed_pile = state.destroyed_pile.union(draft_state.hands[i]);
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
    let id = card_instance_id as u8;
    let mut phase = std::mem::replace(&mut state.phase, GamePhase::Draw);
    match &mut phase {
        GamePhase::Draft { draft_state } => {
            assert!(
                draft_state.hands[player_index].contains(id),
                "Card not found in player's draft hand"
            );
            draft_state.hands[player_index].remove(id);
            state.players[player_index].drafted_cards.insert(id);
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
