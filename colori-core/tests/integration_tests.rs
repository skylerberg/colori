use colori_core::colori_game::{apply_choice_to_state, check_choice_available, enumerate_choices};
use colori_core::draw_phase::execute_draw_phase;
use colori_core::scoring::calculate_score;
use colori_core::setup::create_initial_game_state;
use colori_core::types::{Color, GamePhase, GameState};
use rand::SeedableRng;
use wyrand::WyRand;

// ── Helpers ──

fn count_all_cards(state: &GameState) -> u32 {
    let mut total = 0u32;

    for player in state.players.iter() {
        total += player.deck.len();
        total += player.discard.len();
        total += player.workshopped_cards.len();
        total += player.workshop_cards.len();
        total += player.drafted_cards.len();
    }

    total += state.draft_deck.len();
    total += state.destroyed_pile.len();

    if let GamePhase::Draft { ref draft_state } = state.phase {
        for i in 0..draft_state.num_hands {
            total += draft_state.hands[i].len();
        }
    }

    total
}

fn count_all_buyers(state: &GameState) -> u32 {
    let mut total = 0u32;

    total += state.buyer_deck.len();
    total += state.buyer_display.len() as u32;

    for player in state.players.iter() {
        total += player.completed_buyers.len() as u32;
    }

    total
}

fn check_no_negative_resources(state: &GameState, step: u32, seed: u64, num_players: usize) {
    for (pi, player) in state.players.iter().enumerate() {
        for &count in player.color_wheel.counts.iter() {
            assert!(
                count < 1000,
                "Player {} has color wheel count {} (likely u32 underflow) at step {} (seed={}, players={})",
                pi, count, step, seed, num_players
            );
        }
        for &count in player.materials.counts.iter() {
            assert!(
                count < 1000,
                "Player {} has material count {} (likely u32 underflow) at step {} (seed={}, players={})",
                pi, count, step, seed, num_players
            );
        }
    }
}

fn run_full_game_with_invariants(seed: u64, num_players: usize) -> GameState {
    let mut rng = WyRand::seed_from_u64(seed);
    let ai_players = vec![true; num_players];
    let mut state = create_initial_game_state(num_players, &ai_players, &mut rng);

    let initial_cards = count_all_cards(&state);
    let initial_buyers = count_all_buyers(&state);

    execute_draw_phase(&mut state, &mut rng);

    let mut step = 0u32;
    loop {
        // Check invariants after each transition
        assert_eq!(
            count_all_cards(&state),
            initial_cards,
            "Card conservation violated at step {} (seed={}, players={})",
            step,
            seed,
            num_players
        );
        assert_eq!(
            count_all_buyers(&state),
            initial_buyers,
            "Buyer conservation violated at step {} (seed={}, players={})",
            step,
            seed,
            num_players
        );
        for (pi, player) in state.players.iter().enumerate() {
            assert_eq!(
                player.cached_score,
                calculate_score(player),
                "Cached score mismatch for player {} at step {} (seed={}, players={})",
                pi,
                step,
                seed,
                num_players
            );
        }
        check_no_negative_resources(&state, step, seed, num_players);

        match state.phase {
            GamePhase::GameOver => break,
            GamePhase::Draw => {
                execute_draw_phase(&mut state, &mut rng);
                step += 1;
                continue;
            }
            _ => {}
        }

        let choices = enumerate_choices(&state);
        assert!(
            !choices.is_empty(),
            "No choices available at step {} (seed={}, players={})",
            step,
            seed,
            num_players
        );

        for choice in &choices {
            assert!(
                check_choice_available(&state, choice),
                "Choice {:?} from enumerate_choices failed check_choice_available at step {} (seed={}, players={})",
                choice, step, seed, num_players
            );
        }

        apply_choice_to_state(&mut state, &choices[0], &mut rng);
        step += 1;

        assert!(
            step < 10_000,
            "Game did not terminate within 10000 steps (seed={}, players={})",
            seed,
            num_players
        );
    }

    state
}

// ── Full game simulation tests ──

#[test]
fn test_card_conservation() {
    for seed in 0..10 {
        for num_players in 2..=4 {
            run_full_game_with_invariants(seed, num_players);
        }
    }
}

#[test]
fn test_buyer_conservation() {
    for seed in 0..10 {
        for num_players in 2..=4 {
            run_full_game_with_invariants(seed, num_players);
        }
    }
}

#[test]
fn test_cached_score_consistency() {
    for seed in 10..20 {
        for num_players in 2..=4 {
            run_full_game_with_invariants(seed, num_players);
        }
    }
}

#[test]
fn test_all_enumerated_choices_are_valid() {
    for seed in 20..30 {
        for num_players in 2..=4 {
            run_full_game_with_invariants(seed, num_players);
        }
    }
}

#[test]
fn test_game_terminates_properly() {
    for seed in 30..40 {
        for num_players in 2..=4 {
            let state = run_full_game_with_invariants(seed, num_players);
            let any_reached_15 = state.players.iter().any(|p| p.cached_score >= 15);
            assert!(
                any_reached_15 || state.round > 20,
                "Game ended without meeting termination condition (seed={}, players={}, round={})",
                seed,
                num_players,
                state.round
            );
        }
    }
}

#[test]
fn test_no_negative_resources() {
    for seed in 40..50 {
        for num_players in 2..=4 {
            run_full_game_with_invariants(seed, num_players);
        }
    }
}

// ── Initial state tests ──

#[test]
fn test_draft_deck_card_count() {
    let mut rng = WyRand::seed_from_u64(0);
    let state = create_initial_game_state(2, &[true, true], &mut rng);
    assert_eq!(
        state.draft_deck.len(),
        94,
        "Draft deck should have 94 cards (63 dye + 15 material + 16 action)"
    );
}

#[test]
fn test_buyer_deck_card_count() {
    for num_players in 2..=4 {
        let mut rng = WyRand::seed_from_u64(0);
        let ai_players = vec![true; num_players];
        let state = create_initial_game_state(num_players, &ai_players, &mut rng);
        let total_buyers = state.buyer_deck.len() + state.buyer_display.len() as u32;
        assert_eq!(
            total_buyers, 54,
            "Total buyers should be 54 (players={})",
            num_players
        );
    }
}

#[test]
fn test_personal_deck_card_count() {
    for num_players in 2..=4 {
        let mut rng = WyRand::seed_from_u64(0);
        let ai_players = vec![true; num_players];
        let state = create_initial_game_state(num_players, &ai_players, &mut rng);
        for (i, player) in state.players.iter().enumerate() {
            assert_eq!(
                player.deck.len(),
                7,
                "Player {} should start with 7 cards (players={})",
                i,
                num_players
            );
        }
    }
}

#[test]
fn test_starting_color_wheel() {
    let mut rng = WyRand::seed_from_u64(0);
    let state = create_initial_game_state(3, &[true, true, true], &mut rng);

    for (i, player) in state.players.iter().enumerate() {
        assert_eq!(
            player.color_wheel.get(Color::Red),
            1,
            "Player {} should start with 1 Red",
            i
        );
        assert_eq!(
            player.color_wheel.get(Color::Yellow),
            1,
            "Player {} should start with 1 Yellow",
            i
        );
        assert_eq!(
            player.color_wheel.get(Color::Blue),
            1,
            "Player {} should start with 1 Blue",
            i
        );

        let non_primary = [
            Color::Vermilion,
            Color::Orange,
            Color::Amber,
            Color::Chartreuse,
            Color::Green,
            Color::Teal,
            Color::Indigo,
            Color::Purple,
            Color::Magenta,
        ];
        for &color in &non_primary {
            assert_eq!(
                player.color_wheel.get(color),
                0,
                "Player {} should start with 0 {:?}",
                i,
                color
            );
        }
    }
}

#[test]
fn test_score_is_stars_plus_ducats() {
    for seed in 50..60 {
        for num_players in 2..=4 {
            let state = run_full_game_with_invariants(seed, num_players);
            for (i, player) in state.players.iter().enumerate() {
                let stars: u32 = player
                    .completed_buyers
                    .iter()
                    .map(|bi| bi.buyer.stars())
                    .sum();
                let expected = stars + player.ducats;
                assert_eq!(
                    calculate_score(player),
                    expected,
                    "Score mismatch for player {} (seed={}, players={}): stars={}, ducats={}",
                    i,
                    seed,
                    num_players,
                    stars,
                    player.ducats
                );
            }
        }
    }
}
