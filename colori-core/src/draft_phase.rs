use crate::action_phase::initialize_action_phase;
use crate::types::{DraftState, GamePhase, GameState, MAX_PLAYERS};
use crate::unordered_cards::UnorderedCards;
use rand::Rng;

pub fn initialize_draft<R: Rng>(state: &mut GameState, rng: &mut R) {
    let num_players = state.players.len();
    let mut hands = [UnorderedCards::new(); MAX_PLAYERS];

    for i in 0..num_players {
        let deck_len = state.draft_deck.len();
        if deck_len >= 5 {
            hands[i] = state.draft_deck.draw_multiple(5, rng);
        } else {
            // Take everything from deck directly
            hands[i] = state.draft_deck;
            state.draft_deck = UnorderedCards::new();
            let remaining = 5 - deck_len;
            if remaining > 0 {
                if state.draft_deck.is_empty() && !state.destroyed_pile.is_empty() {
                    state.draft_deck = state.destroyed_pile;
                    state.destroyed_pile = UnorderedCards::new();
                }
                let available = state.draft_deck.len().min(remaining);
                if available > 0 {
                    let drawn = state.draft_deck.draw_multiple(available, rng);
                    hands[i] = hands[i].union(drawn);
                }
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

    let (pi, next_player) = match &mut state.phase {
        GamePhase::Draft { draft_state } => {
            let pi = draft_state.current_player_index;
            assert!(
                draft_state.hands[pi].contains(id),
                "Card not found in player's draft hand"
            );
            draft_state.hands[pi].remove(id);
            let next = (pi + 1) % num_players;
            draft_state.current_player_index = next;
            (pi, next)
        }
        _ => panic!("Expected draft phase"),
    };
    state.players[pi].drafted_cards.insert(id);

    if next_player == starting_player {
        advance_draft(state);
    } else if let GamePhase::Draft { ref mut draft_state } = state.phase {
        draft_state.waiting_for_pass = true;
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
    match &mut state.phase {
        GamePhase::Draft { draft_state } => {
            assert!(
                draft_state.hands[player_index].contains(id),
                "Card not found in player's draft hand"
            );
            draft_state.hands[player_index].remove(id);
        }
        _ => panic!("Expected draft phase"),
    }
    state.players[player_index].drafted_cards.insert(id);
}

pub fn confirm_pass(state: &mut GameState) {
    if let GamePhase::Draft { ref mut draft_state } = state.phase {
        draft_state.waiting_for_pass = false;
    } else {
        panic!("Expected draft phase");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apply_choice::apply_choice;
    use crate::colori_game::enumerate_choices;
    use crate::draw_phase::execute_draw_phase;
    use crate::scoring::calculate_score;
    use crate::setup::create_initial_game_state;
    use crate::types::*;
    use crate::unordered_cards::{
        get_buyer_registry, get_card_registry, set_buyer_registry, set_card_registry,
    };
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    fn test_serialize(state: &GameState) -> String {
        set_card_registry(&state.card_lookup);
        set_buyer_registry(&state.buyer_lookup);
        serde_json::to_string(state).unwrap()
    }

    fn test_deserialize(json: &str) -> GameState {
        let mut state: GameState = serde_json::from_str(json).unwrap();
        state.card_lookup = get_card_registry();
        state.buyer_lookup = get_buyer_registry();
        for p in state.players.iter_mut() {
            p.cached_score = calculate_score(p);
        }
        state
    }

    fn round_trip(state: &GameState) -> GameState {
        let json = test_serialize(state);
        test_deserialize(&json)
    }

    #[test]
    fn test_round2_draft_hands_after_serde_round_trips() {
        let mut rng = SmallRng::seed_from_u64(42);
        let ai_players = vec![false, true, true];
        let mut state = create_initial_game_state(3, &ai_players, &mut rng);

        // Play through round 1 fully using enumerate_choices + apply_choice
        assert_eq!(state.round, 1);

        // Execute draw phase to start round 1
        execute_draw_phase(&mut state, &mut rng);
        assert!(matches!(state.phase, GamePhase::Draft { .. }));

        // Play through all phases until round 2
        loop {
            if state.round == 2 && matches!(state.phase, GamePhase::Draw) {
                break;
            }

            let choices = enumerate_choices(&state);
            if choices.is_empty() {
                // waiting_for_pass - call confirm_pass
                confirm_pass(&mut state);
                continue;
            }
            apply_choice(&mut state, &choices[0], &mut rng);
        }

        // Now at round 2 draw phase - execute draw to initialize draft
        execute_draw_phase(&mut state, &mut rng);
        assert!(matches!(state.phase, GamePhase::Draft { .. }));
        assert_eq!(state.round, 2);

        // Verify all 3 hands have 5 cards
        if let GamePhase::Draft { ref draft_state } = state.phase {
            for i in 0..3 {
                assert_eq!(
                    draft_state.hands[i].len(),
                    5,
                    "Hand {} should have 5 cards before any picks",
                    i
                );
            }
            // In round 2, starting player is (2-1) % 3 = player 1 (AI)
            assert_eq!(draft_state.current_player_index, 1);
        }

        // Simulate the frontend's WASM round-trip pattern for AI picks.
        // The frontend does: serialize -> deserialize -> applyChoice -> serialize -> deserialize -> confirmPass
        // for each AI pick.

        // AI player 1 picks
        let choices = enumerate_choices(&state);
        assert!(!choices.is_empty(), "AI player 1 should have draft choices");
        // Round trip before applying choice (simulating frontend state passing)
        state = round_trip(&state);
        apply_choice(&mut state, &choices[0], &mut rng);
        // Round trip after applying choice (the applyChoice WASM call returns serialized state)
        state = round_trip(&state);

        // Check hand[0] after first AI pick + round trips
        if let GamePhase::Draft { ref draft_state } = state.phase {
            assert_eq!(
                draft_state.hands[0].len(),
                5,
                "Hand 0 should still have 5 cards after player 1's pick (before confirm_pass)"
            );
        }

        // confirmPass round trip
        confirm_pass(&mut state);
        state = round_trip(&state);

        if let GamePhase::Draft { ref draft_state } = state.phase {
            assert_eq!(
                draft_state.hands[0].len(),
                5,
                "Hand 0 should still have 5 cards after player 1's confirm_pass"
            );
        }

        // AI player 2 picks
        let choices = enumerate_choices(&state);
        assert!(!choices.is_empty(), "AI player 2 should have draft choices");
        state = round_trip(&state);
        apply_choice(&mut state, &choices[0], &mut rng);
        state = round_trip(&state);

        if let GamePhase::Draft { ref draft_state } = state.phase {
            assert_eq!(
                draft_state.hands[0].len(),
                5,
                "Hand 0 should still have 5 cards after player 2's pick (before confirm_pass)"
            );
        }

        // After player 2 picks, all players have picked once, so advance_draft rotates hands.
        // Now it should be player 1's turn again (starting player for pick 2).
        // But first check that hand 0 still has the right number of cards.
        // After rotation, each hand should have 4 cards (since one was picked from each).

        // The human (player 0) should now pick. They should see their hand.
        let choices = enumerate_choices(&state);
        if choices.is_empty() {
            // May need to confirm_pass first
            confirm_pass(&mut state);
            state = round_trip(&state);
        }
        let choices = enumerate_choices(&state);

        // The human should have choices matching their hand size
        let hand_size = if let GamePhase::Draft { ref draft_state } = state.phase {
            draft_state.hands[draft_state.current_player_index].len()
        } else {
            panic!("Expected draft phase");
        };

        assert_eq!(
            choices.len(),
            hand_size as usize,
            "Number of draft choices should match hand size for human player"
        );

        // Player 0 hasn't picked yet, so their hand should still have 5 cards.
        // (Rotation only happens after ALL players in a round have picked.)
        if let GamePhase::Draft { ref draft_state } = state.phase {
            assert_eq!(
                draft_state.hands[0].len(),
                5,
                "Hand 0 should have 5 cards - player 0 hasn't picked yet"
            );
        }
    }
}
