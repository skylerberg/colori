use crate::action_phase::initialize_action_phase;
use crate::draw_log_helpers::is_replaying;
use crate::game_log::{DrawEvent, DrawLog};
use crate::types::{CardInstance, DraftState, GamePhase, GameState, MAX_PLAYERS};
use crate::unordered_cards::UnorderedCards;
use rand::Rng;

/// Cards dealt into each draft hand. Hands pass until each is down to its last
/// card, which goes to whoever it was passed to, so a player drafts exactly
/// this many and nothing is left over to trash.
pub const DRAFT_HAND_SIZE: u32 = 4;

pub fn initialize_draft<R: Rng>(state: &mut GameState, rng: &mut R) {
    let num_players = state.players.len();
    // Solo mode: deal 2 hands so the player gets hand rotation
    let num_hands = if num_players == 1 { 2 } else { num_players };
    let mut hands = [UnorderedCards::new(); MAX_PLAYERS];

    if is_replaying(state) {
        replay_draft_deals(state, &mut hands);
    } else {
        for hand in &mut hands[..num_hands] {
            *hand = deal_draft_hand(state, rng);
        }
        record_draft_deals(state, &hands[..num_hands]);
    }

    if hands[..num_hands].iter().any(|hand| hand.is_empty()) {
        for hand in &hands[..num_hands] {
            state.trash = state.trash.union(*hand);
        }
        initialize_action_phase(state);
        return;
    }

    state.phase = GamePhase::Draft {
        draft_state: DraftState {
            pick_number: 0,
            current_player_index: ((state.round - 1) as usize) % num_players,
            hands,
            num_hands,
        },
    };
}

/// Deal one draft hand. When the draft deck runs short, the hand takes what is
/// left and the trash becomes the new draft deck.
pub(crate) fn deal_draft_hand<R: Rng>(state: &mut GameState, rng: &mut R) -> UnorderedCards {
    if state.draft_deck.len() >= DRAFT_HAND_SIZE {
        return state.draft_deck.draw_multiple(DRAFT_HAND_SIZE, rng);
    }
    let hand = std::mem::take(&mut state.draft_deck);
    state.draft_deck = std::mem::take(&mut state.trash);
    hand.union(state.draft_deck.draw_multiple(DRAFT_HAND_SIZE - hand.len(), rng))
}

/// Record the hands dealt, phantom solo hands included.
fn record_draft_deals(state: &mut GameState, hands: &[UnorderedCards]) {
    let Some(DrawLog::Recording(log)) = &mut state.draw_log else {
        return;
    };
    for (player_index, hand) in hands.iter().enumerate() {
        if hand.is_empty() {
            continue;
        }
        let cards = hand
            .iter()
            .map(|id| CardInstance {
                instance_id: id as u32,
                card: state.card_lookup[id as usize],
            })
            .collect();
        log.push(DrawEvent::DraftDeal { player_index, cards });
    }
}

/// In replay mode, consume DraftDeal events to set up draft hands.
fn replay_draft_deals(state: &mut GameState, hands: &mut [UnorderedCards; MAX_PLAYERS]) {
    loop {
        let event = match &mut state.draw_log {
            Some(DrawLog::Replaying(queue))
                if matches!(queue.front(), Some(DrawEvent::DraftDeal { .. })) =>
            {
                queue.pop_front()
            }
            _ => break,
        };
        if let Some(DrawEvent::DraftDeal { player_index, cards }) = event {
            for card in &cards {
                let id = card.instance_id as u8;
                // A card the deck no longer holds was dealt after the deck ran
                // short and the trash was shuffled back in.
                if !state.draft_deck.contains(id) {
                    state.draft_deck = state.draft_deck.union(state.trash);
                    state.trash = UnorderedCards::new();
                }
                state.draft_deck.remove(id);
                hands[player_index].insert(id);
            }
        }
    }
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
    state.players[pi].top_cards.insert(id);

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
            state.trash.insert(removed_id);
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
    let starting = ((state.round - 1) as usize) % num_players;

    let GamePhase::Draft { draft_state } = &mut state.phase else {
        panic!("Expected draft phase");
    };
    let n = draft_state.num_hands;
    draft_state.hands[..n].rotate_right(1);
    draft_state.pick_number += 1;

    // Skip past players with empty hands
    let mut current = starting;
    while draft_state.hands[current].is_empty() {
        current = (current + 1) % num_players;
        if current == starting {
            break;
        }
    }
    draft_state.current_player_index = current;

    end_draft_if_no_choice_left(state);
}

/// Once no hand holds more than one card there is nothing left to choose, so
/// each player takes the card passed to them and the draft is over. A solo
/// game's phantom hand has nobody to take its card, so that card is trashed.
fn end_draft_if_no_choice_left(state: &mut GameState) {
    let (hands, num_hands) = match &state.phase {
        GamePhase::Draft { draft_state } => (draft_state.hands, draft_state.num_hands),
        _ => panic!("Expected draft phase"),
    };
    if hands[..num_hands].iter().any(|hand| hand.len() > 1) {
        return;
    }
    for (i, hand) in hands[..num_hands].iter().enumerate() {
        if i < state.players.len() {
            let player = &mut state.players[i];
            player.top_cards = player.top_cards.union(*hand);
        } else {
            state.trash = state.trash.union(*hand);
        }
    }
    initialize_action_phase(state);
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
    state.players[player_index].top_cards.insert(id);
}

#[cfg(test)]
mod tests {
    use super::DRAFT_HAND_SIZE;
    use crate::apply_choice::apply_choice;
    use crate::colori_game::{enumerate_choices, get_game_status, GameStatus};
    use crate::scoring::calculate_whole_points;
    use crate::setup::create_initial_game_state;
    use crate::types::*;
    use crate::unordered_cards::{
        get_buyer_registry, get_card_registry, set_buyer_registry, set_card_registry,
    };
    use rand::SeedableRng;
    use wyrand::WyRand;

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
            p.cached_whole_points = calculate_whole_points(p);
        }
        state
    }

    fn round_trip(state: &GameState) -> GameState {
        let json = test_serialize(state);
        test_deserialize(&json)
    }

    /// Run the buyers phase to its end by always claiming the first option,
    /// so a test that cares about the draft can get to it.
    fn resolve_buyers<R: rand::Rng>(state: &mut GameState, rng: &mut R) {
        while matches!(state.phase, GamePhase::Buyers { .. }) {
            let player = actor(state);
            let choices = enumerate_choices(state, player);
            apply_choice(state, player, &choices[0], rng);
        }
    }

    fn actor(state: &GameState) -> usize {
        match get_game_status(state, None) {
            GameStatus::AwaitingAction { player_index } => player_index,
            GameStatus::Terminated { .. } => panic!("No one acts in a finished game"),
        }
    }

    #[test]
    fn test_round2_draft_hands_after_serde_round_trips() {
        let mut rng = WyRand::seed_from_u64(42);
        let bot_players = vec![false, true, true];
        let mut state = create_initial_game_state(3, &bot_players, &mut rng);

        // Play through round 1 fully using enumerate_choices + apply_choice
        assert_eq!(state.round, 1);

        // Setup dealt the workshops and opened round 1's buyers phase.
        assert!(matches!(state.phase, GamePhase::Buyers { .. }));
        resolve_buyers(&mut state, &mut rng);
        assert!(matches!(state.phase, GamePhase::Draft { .. }));

        // Play through all phases until round 2 opens
        loop {
            if state.round == 2 && matches!(state.phase, GamePhase::Buyers { .. }) {
                break;
            }

            let player = actor(&state);
            let choices = enumerate_choices(&state, player);
            apply_choice(&mut state, player, &choices[0], &mut rng);
        }

        // Round 2 opened with its buyers phase; claim through it
        resolve_buyers(&mut state, &mut rng);
        assert!(matches!(state.phase, GamePhase::Draft { .. }));
        assert_eq!(state.round, 2);

        // Verify all 3 hands are full
        if let GamePhase::Draft { ref draft_state } = state.phase {
            for i in 0..3 {
                assert_eq!(
                    draft_state.hands[i].len(),
                    DRAFT_HAND_SIZE,
                    "Hand {} should be full before any picks",
                    i
                );
            }
            // In round 2, starting player is (2-1) % 3 = player 1 (a bot)
            assert_eq!(draft_state.current_player_index, 1);
        }

        // Simulate the frontend's WASM round-trip pattern for bot picks.
        // The frontend does: serialize -> deserialize -> applyChoice -> serialize -> deserialize
        // for each bot pick.

        // bot player 1 picks
        let choices = enumerate_choices(&state, 1);
        assert!(!choices.is_empty(), "bot player 1 should have draft choices");
        // Round trip before applying choice (simulating frontend state passing)
        state = round_trip(&state);
        apply_choice(&mut state, 1, &choices[0], &mut rng);
        // Round trip after applying choice (the applyChoice WASM call returns serialized state)
        state = round_trip(&state);

        if let GamePhase::Draft { ref draft_state } = state.phase {
            assert_eq!(
                draft_state.hands[0].len(),
                DRAFT_HAND_SIZE,
                "Hand 0 should still be full after player 1's pick"
            );
        }

        // bot player 2 picks
        let choices = enumerate_choices(&state, 2);
        assert!(!choices.is_empty(), "bot player 2 should have draft choices");
        state = round_trip(&state);
        apply_choice(&mut state, 2, &choices[0], &mut rng);
        state = round_trip(&state);

        if let GamePhase::Draft { ref draft_state } = state.phase {
            assert_eq!(
                draft_state.hands[0].len(),
                DRAFT_HAND_SIZE,
                "Hand 0 should still be full after player 2's pick"
            );
        }

        // After player 2 picks, all players have picked once, so advance_draft rotates hands.
        // Now it should be player 1's turn again (starting player for pick 2).
        // But first check that hand 0 still has the right number of cards.
        // After rotation, each hand should hold one card fewer.

        // The human (player 0) should now pick. They should see their hand.
        let choices = enumerate_choices(&state, 0);

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

        // Player 0 hasn't picked yet, so their hand should still be full.
        // (Rotation only happens after ALL players in a round have picked.)
        if let GamePhase::Draft { ref draft_state } = state.phase {
            assert_eq!(
                draft_state.hands[0].len(),
                DRAFT_HAND_SIZE,
                "Hand 0 should be full - player 0 hasn't picked yet"
            );
        }
    }

}
