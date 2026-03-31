use crate::action_phase::initialize_action_phase;
use crate::types::{DraftState, GamePhase, GameState, GlassCard, MAX_PLAYERS};
use crate::unordered_cards::UnorderedCards;
use rand::Rng;

pub fn initialize_draft<R: Rng>(state: &mut GameState, rng: &mut R) {
    let num_players = state.players.len();
    // Solo mode: deal 2 hands so the player gets hand rotation
    let num_hands = if num_players == 1 { 2 } else { num_players };
    let mut hands = [UnorderedCards::new(); MAX_PLAYERS];

    for i in 0..num_hands {
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

    if (0..num_hands).any(|i| hands[i].is_empty()) {
        for i in 0..num_hands {
            state.destroyed_pile = state.destroyed_pile.union(hands[i]);
        }
        initialize_action_phase(state);
        return;
    }

    let draft_state = DraftState {
        pick_number: 0,
        current_player_index: ((state.round - 1) as usize) % num_players,
        hands,
        num_hands,
    };

    state.phase = GamePhase::Draft { draft_state };
}

pub fn player_pick<R: Rng>(state: &mut GameState, card_instance_id: u32, rng: &mut R) {
    let num_players = state.players.len();
    let starting_player = ((state.round - 1) as usize) % num_players;
    let id = card_instance_id as u8;

    let (pi, should_advance) = match &mut state.phase {
        GamePhase::Draft { draft_state } => {
            let pi = draft_state.current_player_index;
            assert!(
                draft_state.hands[pi].contains(id),
                "Card not found in player's draft hand"
            );
            draft_state.hands[pi].remove(id);
            // Find next player, skipping players with empty hands
            let mut next = (pi + 1) % num_players;
            let mut crossed_start = false;
            loop {
                if next == starting_player {
                    crossed_start = true;
                }
                if !draft_state.hands[next].is_empty() || crossed_start {
                    break;
                }
                next = (next + 1) % num_players;
            }
            draft_state.current_player_index = next;
            (pi, crossed_start)
        }
        _ => panic!("Expected draft phase"),
    };
    state.players[pi].drafted_cards.insert(id);

    // Solo mode: remove a random card from each phantom hand
    if num_players == 1 {
        let num_hands = match &state.phase {
            GamePhase::Draft { draft_state } => draft_state.num_hands,
            _ => 0,
        };
        for hand_idx in 1..num_hands {
            let is_empty = match &state.phase {
                GamePhase::Draft { draft_state } => draft_state.hands[hand_idx].is_empty(),
                _ => true,
            };
            if is_empty {
                continue;
            }
            let removed_id = phantom_draft_removal(state, hand_idx, rng);
            if let GamePhase::Draft { ref mut draft_state } = state.phase {
                draft_state.hands[hand_idx].remove(removed_id);
            }
            state.destroyed_pile.insert(removed_id);
        }
    }

    if should_advance {
        advance_draft(state);
    }
}

/// Remove a card from a phantom hand, recording/replaying via draw log.
fn phantom_draft_removal<R: Rng>(state: &mut GameState, hand_idx: usize, rng: &mut R) -> u8 {
    use crate::game_log::{DrawEvent, DrawLog};
    use crate::types::CardInstance;

    // Check if replaying
    if let Some(DrawLog::Replaying(events)) = &mut state.draw_log {
        if let Some(pos) = events.iter().position(|e| matches!(e, DrawEvent::PhantomDraftRemoval { hand_index, .. } if *hand_index == hand_idx)) {
            if let DrawEvent::PhantomDraftRemoval { card, .. } = events.remove(pos).unwrap() {
                return card.instance_id as u8;
            }
        }
    }

    // Pick randomly
    let removed_id = match &state.phase {
        GamePhase::Draft { draft_state } => {
            draft_state.hands[hand_idx].pick_random(rng).expect("Phantom hand should not be empty")
        }
        _ => panic!("Expected draft phase"),
    };

    // Record if recording
    if let Some(DrawLog::Recording(events)) = &mut state.draw_log {
        events.push(DrawEvent::PhantomDraftRemoval {
            hand_index: hand_idx,
            card: CardInstance {
                instance_id: removed_id as u32,
                card: state.card_lookup[removed_id as usize],
            },
        });
    }

    removed_id
}

pub fn advance_draft(state: &mut GameState) {
    let num_players = state.players.len();
    let round = state.round;

    let (pick_number, n) = {
        let draft_state = match &mut state.phase {
            GamePhase::Draft { draft_state } => draft_state,
            _ => panic!("Expected draft phase"),
        };

        let n = draft_state.num_hands;
        let last = draft_state.hands[n - 1];
        for i in (1..n).rev() {
            draft_state.hands[i] = draft_state.hands[i - 1];
        }
        draft_state.hands[0] = last;

        draft_state.pick_number += 1;
        (draft_state.pick_number, n)
    };

    // GlassKeepBoth: after pick 3, players with this glass keep both remaining cards
    if pick_number == 3 {
        let mut hands_to_clear = [false; MAX_PLAYERS];
        if let GamePhase::Draft { ref draft_state } = state.phase {
            for i in 0..num_players {
                let has_keep_both = state.players[i]
                    .completed_glass
                    .iter()
                    .any(|g| g.card == GlassCard::GlassKeepBoth);
                if has_keep_both {
                    for id in draft_state.hands[i].iter() {
                        state.players[i].drafted_cards.insert(id);
                    }
                    hands_to_clear[i] = true;
                }
            }
        }
        if let GamePhase::Draft { ref mut draft_state } = state.phase {
            for i in 0..num_players {
                if hands_to_clear[i] {
                    draft_state.hands[i] = UnorderedCards::new();
                }
            }
        }
    }

    let all_hands_empty = if let GamePhase::Draft { ref draft_state } = state.phase {
        (0..n).all(|i| draft_state.hands[i].is_empty())
    } else {
        false
    };

    if pick_number >= 4 || all_hands_empty {
        // GlassKeepBoth was already handled at pick 3, so remaining cards go to destroyed pile
        if let GamePhase::Draft { ref draft_state } = state.phase {
            for i in 0..draft_state.num_hands {
                state.destroyed_pile = state.destroyed_pile.union(draft_state.hands[i]);
            }
        }
        initialize_action_phase(state);
    } else {
        if let GamePhase::Draft { ref mut draft_state } = state.phase {
            let mut start = ((round - 1) as usize) % num_players;
            // Skip players with empty hands (e.g., GlassKeepBoth already took their cards)
            let original_start = start;
            loop {
                if !draft_state.hands[start].is_empty() {
                    break;
                }
                start = (start + 1) % num_players;
                if start == original_start {
                    break;
                }
            }
            draft_state.current_player_index = start;
        }
    }
}

pub fn simultaneous_pick(state: &mut GameState, player_index: usize, card: crate::types::Card) {
    let id = match &state.phase {
        GamePhase::Draft { draft_state } => {
            let hand = draft_state.hands[player_index];
            let mut found = None;
            for inst_id in hand.iter() {
                if state.card_lookup[inst_id as usize] == card {
                    found = Some(inst_id);
                    break;
                }
            }
            found.expect("Card type not found in player's draft hand")
        }
        _ => panic!("Expected draft phase"),
    };
    match &mut state.phase {
        GamePhase::Draft { draft_state } => {
            draft_state.hands[player_index].remove(id);
        }
        _ => unreachable!(),
    }
    state.players[player_index].drafted_cards.insert(id);
}

#[cfg(test)]
mod tests {
    use crate::apply_choice::apply_choice;
    use crate::colori_game::enumerate_choices;
    use crate::draw_phase::execute_draw_phase;
    use crate::scoring::calculate_score;
    use crate::setup::create_initial_game_state;
    use crate::types::*;
    use crate::unordered_cards::{
        get_sell_card_registry, get_card_registry, set_sell_card_registry, set_card_registry,
    };
    use rand::SeedableRng;
    use wyrand::WyRand;

    fn test_serialize(state: &GameState) -> String {
        set_card_registry(&state.card_lookup);
        set_sell_card_registry(&state.sell_card_lookup);
        serde_json::to_string(state).unwrap()
    }

    fn test_deserialize(json: &str) -> GameState {
        let mut state: GameState = serde_json::from_str(json).unwrap();
        state.card_lookup = get_card_registry();
        state.sell_card_lookup = get_sell_card_registry();
        for p in state.players.iter_mut() {
            p.cached_score = calculate_score(p);
        }
        state
    }

    fn round_trip(state: &GameState) -> GameState {
        let json = test_serialize(state);
        test_deserialize(&json)
    }

    use smallvec::smallvec;

    /// Helper: set up a 3-player game at draft phase with specified players having GlassKeepBoth.
    fn setup_draft_with_glass_keep_both(
        glass_players: &[usize],
    ) -> (GameState, WyRand) {
        let mut rng = WyRand::seed_from_u64(123);
        let ai_players = vec![false, false, false];
        let mut state = create_initial_game_state(3, &ai_players, &mut rng);

        // Give specified players GlassKeepBoth
        for &idx in glass_players {
            state.players[idx].completed_glass = smallvec![GlassInstance {
                instance_id: 1000 + idx as u32,
                card: GlassCard::GlassKeepBoth,
            }];
        }

        // Execute draw phase to enter draft
        execute_draw_phase(&mut state, &mut rng);
        assert!(matches!(state.phase, GamePhase::Draft { .. }));

        (state, rng)
    }

    #[test]
    fn test_glass_keep_both_one_player() {
        let (mut state, mut rng) = setup_draft_with_glass_keep_both(&[0]);

        // Verify hands start with 5 cards each
        if let GamePhase::Draft { ref draft_state } = state.phase {
            for i in 0..3 {
                assert_eq!(draft_state.hands[i].len(), 5);
            }
        }

        // Do 3 rounds of picks (all players pick once per round)
        for _pick_round in 0..3 {
            assert!(matches!(state.phase, GamePhase::Draft { .. }));
            // Each player picks one card
            for _player in 0..3 {
                let choices = enumerate_choices(&state);
                assert!(!choices.is_empty());
                apply_choice(&mut state, &choices[0], &mut rng);
            }
        }

        // After 3 picks, player 0 (GlassKeepBoth) should have gotten both remaining cards
        // and their hand should be empty. Others should still have 2-card hands.
        assert!(matches!(state.phase, GamePhase::Draft { .. }));

        // Player 0 should have 3 picked + 2 kept = 5 drafted cards
        assert_eq!(
            state.players[0].drafted_cards.len(),
            5,
            "GlassKeepBoth player should have 5 drafted cards (3 picked + 2 kept)"
        );

        if let GamePhase::Draft { ref draft_state } = state.phase {
            // Player 0's hand should be empty
            assert_eq!(draft_state.hands[0].len(), 0, "GlassKeepBoth player's hand should be empty");
            // Other players should have 2 cards remaining
            assert_eq!(draft_state.hands[1].len(), 2);
            assert_eq!(draft_state.hands[2].len(), 2);
            // pick_number should be 3
            assert_eq!(draft_state.pick_number, 3);
        }

        // Do pick 4 for the remaining players (player 0 is skipped)
        for _player in 0..2 {
            assert!(matches!(state.phase, GamePhase::Draft { .. }));
            let choices = enumerate_choices(&state);
            assert!(!choices.is_empty());
            apply_choice(&mut state, &choices[0], &mut rng);
        }

        // Draft should now be over, action phase started
        assert!(
            matches!(state.phase, GamePhase::Action { .. }),
            "Draft should end after pick 4 for remaining players"
        );

        // Other players should have 4 drafted cards each (4 picks)
        assert_eq!(state.players[1].drafted_cards.len(), 4);
        assert_eq!(state.players[2].drafted_cards.len(), 4);
    }

    #[test]
    fn test_glass_keep_both_all_players() {
        let (mut state, mut rng) = setup_draft_with_glass_keep_both(&[0, 1, 2]);

        // Do 3 rounds of picks
        for _pick_round in 0..3 {
            assert!(matches!(state.phase, GamePhase::Draft { .. }));
            for _player in 0..3 {
                let choices = enumerate_choices(&state);
                assert!(!choices.is_empty());
                apply_choice(&mut state, &choices[0], &mut rng);
            }
        }

        // After 3 picks with all players having GlassKeepBoth,
        // draft should end immediately (all hands empty)
        assert!(
            matches!(state.phase, GamePhase::Action { .. }),
            "Draft should end after pick 3 when all players have GlassKeepBoth"
        );

        // All players should have 5 drafted cards (3 picked + 2 kept)
        for i in 0..3 {
            assert_eq!(
                state.players[i].drafted_cards.len(),
                5,
                "Player {} should have 5 drafted cards",
                i
            );
        }
    }

    #[test]
    fn test_round2_draft_hands_after_serde_round_trips() {
        let mut rng = WyRand::seed_from_u64(42);
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
        // The frontend does: serialize -> deserialize -> applyChoice -> serialize -> deserialize
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
                "Hand 0 should still have 5 cards after player 1's pick (pick 1 done)"
            );
        }

        if let GamePhase::Draft { ref draft_state } = state.phase {
            assert_eq!(
                draft_state.hands[0].len(),
                5,
                "Hand 0 should still have 5 cards after player 1's pick"
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
                "Hand 0 should still have 5 cards after player 2's pick"
            );
        }

        // After player 2 picks, all players have picked once, so advance_draft rotates hands.
        // Now it should be player 1's turn again (starting player for pick 2).
        // But first check that hand 0 still has the right number of cards.
        // After rotation, each hand should have 4 cards (since one was picked from each).

        // The human (player 0) should now pick. They should see their hand.
        let choices = enumerate_choices(&state);

        // The human should have choices (may be fewer than hand size due to dedup of same card types)
        let hand_size = if let GamePhase::Draft { ref draft_state } = state.phase {
            draft_state.hands[draft_state.current_player_index].len()
        } else {
            panic!("Expected draft phase");
        };

        assert!(
            !choices.is_empty() && choices.len() <= hand_size as usize,
            "Number of draft choices ({}) should be between 1 and hand size ({}) for human player",
            choices.len(),
            hand_size,
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

    #[test]
    fn test_glass_keep_both_keeps_remaining_draft_cards() {
        use crate::setup::create_initial_game_state_with_expansions;
        let mut rng = WyRand::seed_from_u64(99);
        let ai_players = vec![false, true];
        let expansions = Expansions { glass: true };
        let mut state = create_initial_game_state_with_expansions(2, &ai_players, expansions, &mut rng);

        // Give player 0 the GlassKeepBoth card
        state.players[0].completed_glass.push(GlassInstance {
            instance_id: 255,
            card: GlassCard::GlassKeepBoth,
        });

        // Play through round 1 to reach the draft
        execute_draw_phase(&mut state, &mut rng);
        assert!(matches!(state.phase, GamePhase::Draft { .. }));

        // Count initial drafted cards
        let initial_drafted = state.players[0].drafted_cards.len();
        let p1_initial_drafted = state.players[1].drafted_cards.len();

        // Play through all draft picks using enumerate_choices + apply_choice
        while matches!(state.phase, GamePhase::Draft { .. }) {
            let choices = enumerate_choices(&state);
            apply_choice(&mut state, &choices[0], &mut rng);
        }

        // After draft, should be in action phase
        assert!(matches!(state.phase, GamePhase::Action { .. }),
            "Expected action phase after draft, got {:?}", state.phase);

        // Player 0 should have 5 drafted cards (3 picked + 2 kept from GlassKeepBoth)
        let final_drafted = state.players[0].drafted_cards.len();
        assert_eq!(
            final_drafted - initial_drafted, 5,
            "Player 0 with GlassKeepBoth should have 5 new drafted cards (3 picked + 2 kept), but got {}",
            final_drafted - initial_drafted
        );

        // Player 1 should have only 4 drafted cards (no GlassKeepBoth)
        let p1_drafted = state.players[1].drafted_cards.len();
        assert_eq!(
            p1_drafted - p1_initial_drafted, 4,
            "Player 1 without GlassKeepBoth should have 4 drafted cards, but got {}",
            p1_drafted - p1_initial_drafted
        );
    }

    #[test]
    fn test_glass_keep_both_survives_json_round_trip() {
        use crate::setup::create_initial_game_state_with_expansions;
        let mut rng = WyRand::seed_from_u64(99);
        let ai_players = vec![false, true];
        let expansions = Expansions { glass: true };
        let mut state = create_initial_game_state_with_expansions(2, &ai_players, expansions, &mut rng);

        // Give player 0 the GlassKeepBoth card
        state.players[0].completed_glass.push(GlassInstance {
            instance_id: 255,
            card: GlassCard::GlassKeepBoth,
        });

        execute_draw_phase(&mut state, &mut rng);
        let initial_drafted = state.players[0].drafted_cards.len();

        // Simulate frontend WASM round-trips: serialize -> deserialize -> pick -> serialize -> deserialize
        while matches!(state.phase, GamePhase::Draft { .. }) {
            // Round-trip before choice (simulates frontend passing state to WASM)
            state = round_trip(&state);

            let choices = enumerate_choices(&state);
            apply_choice(&mut state, &choices[0], &mut rng);

            // Round-trip after choice (simulates WASM returning state to frontend)
            state = round_trip(&state);
        }

        assert!(matches!(state.phase, GamePhase::Action { .. }));
        let final_drafted = state.players[0].drafted_cards.len();
        assert_eq!(
            final_drafted - initial_drafted, 5,
            "GlassKeepBoth should survive JSON round-trips: expected 5 new drafted cards, got {}",
            final_drafted - initial_drafted
        );
    }

    #[test]
    fn test_glass_keep_both_with_simultaneous_pick_and_advance() {
        use crate::setup::create_initial_game_state_with_expansions;
        use crate::draft_phase::{simultaneous_pick, advance_draft};
        let mut rng = WyRand::seed_from_u64(99);
        let ai_players = vec![false, true];
        let expansions = Expansions { glass: true };
        let mut state = create_initial_game_state_with_expansions(2, &ai_players, expansions, &mut rng);

        // Give player 0 the GlassKeepBoth card
        state.players[0].completed_glass.push(GlassInstance {
            instance_id: 255,
            card: GlassCard::GlassKeepBoth,
        });

        execute_draw_phase(&mut state, &mut rng);
        let initial_drafted_p0 = state.players[0].drafted_cards.len();
        let initial_drafted_p1 = state.players[1].drafted_cards.len();

        // Simulate the frontend pattern: simultaneousPick for each player, then advanceDraft
        // with JSON round-trips between each call
        while matches!(state.phase, GamePhase::Draft { .. }) {
            // Round-trip (simulates frontend serialize -> WASM deserialize)
            state = round_trip(&state);

            // Each player with a non-empty hand picks the first card
            for player_idx in 0..2 {
                let card = if let GamePhase::Draft { ref draft_state } = state.phase {
                    let hand = &draft_state.hands[player_idx];
                    if hand.is_empty() {
                        continue; // GlassKeepBoth player's hand was cleared
                    }
                    let first_id = hand.iter().next().unwrap();
                    state.card_lookup[first_id as usize]
                } else {
                    break;
                };
                simultaneous_pick(&mut state, player_idx, card);
                // Round-trip after each pick
                state = round_trip(&state);
            }

            // Advance draft (rotate hands, or end draft)
            if matches!(state.phase, GamePhase::Draft { .. }) {
                advance_draft(&mut state);
                state = round_trip(&state);
            }
        }

        // Should be in action phase
        assert!(matches!(state.phase, GamePhase::Action { .. }),
            "Expected action phase, got {:?}", state.phase);

        let final_drafted_p0 = state.players[0].drafted_cards.len();
        let final_drafted_p1 = state.players[1].drafted_cards.len();

        assert_eq!(
            final_drafted_p0 - initial_drafted_p0, 5,
            "Player 0 with GlassKeepBoth should have 5 new drafted cards via simultaneous_pick, got {}",
            final_drafted_p0 - initial_drafted_p0
        );
        assert_eq!(
            final_drafted_p1 - initial_drafted_p1, 4,
            "Player 1 without GlassKeepBoth should have 4 new drafted cards, got {}",
            final_drafted_p1 - initial_drafted_p1
        );
    }
}
