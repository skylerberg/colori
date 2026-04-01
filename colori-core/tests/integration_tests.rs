use colori_core::colori_game::{apply_choice_to_state, check_choice_available, enumerate_choices};
use colori_core::draw_phase::execute_draw_phase;
use colori_core::scoring::calculate_score;
use colori_core::setup::create_initial_game_state;
use colori_core::types::{Ability, SellCard, Card, Choice, Color, GamePhase, GameState};
use colori_core::unordered_cards::{
    get_sell_card_registry, get_card_registry, set_sell_card_registry, set_card_registry,
};
use rand::RngExt;
use rand::SeedableRng;
use smallvec::SmallVec;
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

fn count_all_sell_cards(state: &GameState) -> u32 {
    let mut total = 0u32;

    total += state.sell_card_deck.len();
    total += state.sell_card_display.len() as u32;

    for player in state.players.iter() {
        total += player.completed_sell_cards.len() as u32;
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
    let initial_sell_cards = count_all_sell_cards(&state);

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
            count_all_sell_cards(&state),
            initial_sell_cards,
            "Sell card conservation violated at step {} (seed={}, players={})",
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

fn run_random_game_with_invariants(seed: u64, num_players: usize) -> GameState {
    let mut rng = WyRand::seed_from_u64(seed);
    let ai_players = vec![true; num_players];
    let mut state = create_initial_game_state(num_players, &ai_players, &mut rng);

    let initial_cards = count_all_cards(&state);
    let initial_sell_cards = count_all_sell_cards(&state);

    execute_draw_phase(&mut state, &mut rng);

    let mut step = 0u32;
    loop {
        assert_eq!(
            count_all_cards(&state),
            initial_cards,
            "Card conservation violated at step {} (seed={}, players={})",
            step,
            seed,
            num_players
        );
        assert_eq!(
            count_all_sell_cards(&state),
            initial_sell_cards,
            "Sell card conservation violated at step {} (seed={}, players={})",
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

        let idx = rng.random_range(0..choices.len());
        apply_choice_to_state(&mut state, &choices[idx], &mut rng);
        step += 1;

        assert!(
            step < 50_000,
            "Game did not terminate within 50000 steps (seed={}, players={})",
            seed,
            num_players
        );
    }

    state
}

fn generate_invalid_choices(state: &GameState) -> Vec<Choice> {
    let mut invalid = Vec::new();

    match &state.phase {
        GamePhase::Draft { draft_state } => {
            // Wrong-phase choices
            invalid.push(Choice::EndTurn);
            invalid.push(Choice::SkipWorkshop);
            invalid.push(Choice::SelectSellCard {
                sell_card: SellCard::Textiles2Vermilion,
            });
            invalid.push(Choice::GainSecondary {
                color: Color::Orange,
            });
            invalid.push(Choice::GainPrimary {
                color: Color::Red,
            });
            invalid.push(Choice::MixAll {
                mixes: SmallVec::new(),
            });
            invalid.push(Choice::DestroyDrawnCards { card: None });
            // DraftPick with card types not in the current hand
            let hand = draft_state.hands[draft_state.current_player_index];
            let all_cards = [
                Card::BasicRed, Card::BasicYellow, Card::BasicBlue,
                Card::Lac,
                Card::Madder, Card::Turmeric, Card::DyersGreenweed,
                Card::VermilionDye, Card::Saffron, Card::PersianBerries,
                Card::StarterCeramics, Card::StarterPaintings, Card::StarterTextiles,
                Card::Alum, Card::CreamOfTartar, Card::GumArabic,
                Card::Potash, Card::Vinegar, Card::Chalk,
                Card::LinseedOil,
            ];
            for &card in &all_cards {
                let in_hand = hand.iter().any(|id| state.card_lookup[id as usize] == card);
                if !in_hand {
                    invalid.push(Choice::DraftPick { card });
                    break;
                }
            }
        }
        GamePhase::Action { action_state } => {
            // Always wrong during action phase
            invalid.push(Choice::DraftPick {
                card: Card::BasicRed,
            });

            if action_state.ability_stack.is_empty() {
                // Empty stack: these require specific abilities on stack
                invalid.push(Choice::SkipWorkshop);
                invalid.push(Choice::SelectSellCard {
                    sell_card: SellCard::Textiles2Vermilion,
                });
                invalid.push(Choice::GainSecondary {
                    color: Color::Orange,
                });
                invalid.push(Choice::GainPrimary {
                    color: Color::Red,
                });
                invalid.push(Choice::MixAll {
                    mixes: SmallVec::new(),
                });
                invalid.push(Choice::DestroyDrawnCards { card: None });
                invalid.push(Choice::SwapTertiary {
                    lose: Color::Vermilion,
                    gain: Color::Amber,
                });
            } else {
                let top = action_state.ability_stack.last().unwrap();
                match top {
                    Ability::Workshop { .. } => {
                        invalid.push(Choice::EndTurn);
                        invalid.push(Choice::SelectSellCard {
                            sell_card: SellCard::Textiles2Vermilion,
                        });
                        invalid.push(Choice::GainSecondary {
                            color: Color::Orange,
                        });
                        // Empty workshop is invalid
                        invalid.push(Choice::Workshop {
                            card_types: SmallVec::new(),
                        });
                    }
                    Ability::GainSecondary => {
                        // Primary colors are invalid for GainSecondary
                        invalid.push(Choice::GainSecondary {
                            color: Color::Red,
                        });
                        invalid.push(Choice::GainSecondary {
                            color: Color::Yellow,
                        });
                        invalid.push(Choice::GainSecondary {
                            color: Color::Blue,
                        });
                        // Tertiary colors are invalid for GainSecondary
                        invalid.push(Choice::GainSecondary {
                            color: Color::Vermilion,
                        });
                        invalid.push(Choice::EndTurn);
                    }
                    Ability::GainPrimary => {
                        // Secondary colors are invalid for GainPrimary
                        invalid.push(Choice::GainPrimary {
                            color: Color::Orange,
                        });
                        invalid.push(Choice::GainPrimary {
                            color: Color::Green,
                        });
                        invalid.push(Choice::GainPrimary {
                            color: Color::Purple,
                        });
                        // Tertiary colors are invalid for GainPrimary
                        invalid.push(Choice::GainPrimary {
                            color: Color::Vermilion,
                        });
                        invalid.push(Choice::EndTurn);
                    }
                    Ability::ChangeTertiary => {
                        // Same lose/gain is invalid
                        invalid.push(Choice::SwapTertiary {
                            lose: Color::Vermilion,
                            gain: Color::Vermilion,
                        });
                        // Primary colors are invalid for SwapTertiary
                        invalid.push(Choice::SwapTertiary {
                            lose: Color::Red,
                            gain: Color::Vermilion,
                        });
                        invalid.push(Choice::SwapTertiary {
                            lose: Color::Vermilion,
                            gain: Color::Red,
                        });
                        invalid.push(Choice::EndTurn);
                    }
                    Ability::MixColors { .. } => {
                        // Non-adjacent colors are invalid
                        invalid.push(Choice::MixAll {
                            mixes: SmallVec::from_slice(&[(Color::Orange, Color::Green)]),
                        });
                        invalid.push(Choice::EndTurn);
                    }
                    Ability::Sell => {
                        invalid.push(Choice::EndTurn);
                        invalid.push(Choice::SkipWorkshop);
                        invalid.push(Choice::GainSecondary {
                            color: Color::Orange,
                        });
                    }
                    Ability::DestroyCards => {
                        invalid.push(Choice::EndTurn);
                        invalid.push(Choice::SkipWorkshop);
                    }
                    Ability::DrawCards { .. } => {
                        invalid.push(Choice::EndTurn);
                        invalid.push(Choice::SkipWorkshop);
                    }
                    Ability::GainDucats { .. } => {
                        // GainDucats is auto-resolved, but just in case
                        invalid.push(Choice::EndTurn);
                    }
                    Ability::MoveToDrafted => {
                        invalid.push(Choice::EndTurn);
                        invalid.push(Choice::SkipWorkshop);
                    }
                }
            }
        }
        _ => {}
    }

    invalid
}

fn serialize_state(state: &GameState) -> String {
    set_card_registry(&state.card_lookup);
    set_sell_card_registry(&state.sell_card_lookup);
    serde_json::to_string(state).unwrap()
}

fn deserialize_state(json: &str) -> GameState {
    let mut state: GameState = serde_json::from_str(json).unwrap();
    state.card_lookup = get_card_registry();
    state.sell_card_lookup = get_sell_card_registry();
    for p in state.players.iter_mut() {
        p.cached_score = calculate_score(p);
    }
    state
}

fn round_trip(state: &GameState) -> GameState {
    let json = serialize_state(state);
    deserialize_state(&json)
}

fn assert_states_match(a: &GameState, b: &GameState, context: &str) {
    assert_eq!(a.round, b.round, "round mismatch: {}", context);
    assert_eq!(
        a.players.len(),
        b.players.len(),
        "player count mismatch: {}",
        context
    );

    // Compare per-player state
    for (pi, (pa, pb)) in a.players.iter().zip(b.players.iter()).enumerate() {
        assert_eq!(
            pa.deck, pb.deck,
            "player {} deck mismatch: {}",
            pi, context
        );
        assert_eq!(
            pa.discard, pb.discard,
            "player {} discard mismatch: {}",
            pi, context
        );
        assert_eq!(
            pa.workshopped_cards, pb.workshopped_cards,
            "player {} workshopped_cards mismatch: {}",
            pi, context
        );
        assert_eq!(
            pa.workshop_cards, pb.workshop_cards,
            "player {} workshop_cards mismatch: {}",
            pi, context
        );
        assert_eq!(
            pa.drafted_cards, pb.drafted_cards,
            "player {} drafted_cards mismatch: {}",
            pi, context
        );
        assert_eq!(
            pa.color_wheel.counts, pb.color_wheel.counts,
            "player {} color_wheel mismatch: {}",
            pi, context
        );
        assert_eq!(
            pa.materials.counts, pb.materials.counts,
            "player {} materials mismatch: {}",
            pi, context
        );
        assert_eq!(
            pa.ducats, pb.ducats,
            "player {} ducats mismatch: {}",
            pi, context
        );
        assert_eq!(
            pa.cached_score, pb.cached_score,
            "player {} cached_score mismatch: {}",
            pi, context
        );
        assert_eq!(
            pa.completed_sell_cards.len(),
            pb.completed_sell_cards.len(),
            "player {} completed_sell_cards length mismatch: {}",
            pi, context
        );
        for (bi, (ba, bb)) in pa
            .completed_sell_cards
            .iter()
            .zip(pb.completed_sell_cards.iter())
            .enumerate()
        {
            assert_eq!(
                ba.instance_id, bb.instance_id,
                "player {} sell_card {} instance_id mismatch: {}",
                pi, bi, context
            );
            assert_eq!(
                ba.sell_card, bb.sell_card,
                "player {} sell_card {} card mismatch: {}",
                pi, bi, context
            );
        }
    }

    // Compare global state
    assert_eq!(
        a.draft_deck, b.draft_deck,
        "draft_deck mismatch: {}",
        context
    );
    assert_eq!(
        a.destroyed_pile, b.destroyed_pile,
        "destroyed_pile mismatch: {}",
        context
    );
    assert_eq!(
        a.sell_card_deck, b.sell_card_deck,
        "sell_card_deck mismatch: {}",
        context
    );
    assert_eq!(
        a.sell_card_display.len(),
        b.sell_card_display.len(),
        "sell_card_display length mismatch: {}",
        context
    );
    for (i, (ba, bb)) in a
        .sell_card_display
        .iter()
        .zip(b.sell_card_display.iter())
        .enumerate()
    {
        assert_eq!(
            ba.instance_id, bb.instance_id,
            "sell_card_display {} instance_id mismatch: {}",
            i, context
        );
        assert_eq!(
            ba.sell_card, bb.sell_card,
            "sell_card_display {} card mismatch: {}",
            i, context
        );
    }

    // Compare phase
    match (&a.phase, &b.phase) {
        (
            GamePhase::Draft {
                draft_state: da,
            },
            GamePhase::Draft {
                draft_state: db,
            },
        ) => {
            assert_eq!(
                da.pick_number, db.pick_number,
                "draft pick_number mismatch: {}",
                context
            );
            assert_eq!(
                da.current_player_index, db.current_player_index,
                "draft current_player_index mismatch: {}",
                context
            );
            assert_eq!(
                da.num_hands, db.num_hands,
                "draft num_hands mismatch: {}",
                context
            );
            for i in 0..da.num_hands {
                assert_eq!(
                    da.hands[i], db.hands[i],
                    "draft hand {} mismatch: {}",
                    i, context
                );
            }
        }
        (
            GamePhase::Action {
                action_state: aa,
            },
            GamePhase::Action {
                action_state: ab,
            },
        ) => {
            assert_eq!(
                aa.current_player_index, ab.current_player_index,
                "action current_player_index mismatch: {}",
                context
            );
            assert_eq!(
                aa.ability_stack.len(),
                ab.ability_stack.len(),
                "action ability_stack length mismatch: {}",
                context
            );
            for (i, (sa, sb)) in aa
                .ability_stack
                .iter()
                .zip(ab.ability_stack.iter())
                .enumerate()
            {
                assert_eq!(
                    sa, sb,
                    "action ability_stack {} mismatch: {}",
                    i, context
                );
            }
        }
        (GamePhase::Draw, GamePhase::Draw) => {}
        (GamePhase::GameOver, GamePhase::GameOver) => {}
        _ => panic!(
            "Phase variant mismatch: {:?} vs {:?}: {}",
            std::mem::discriminant(&a.phase),
            std::mem::discriminant(&b.phase),
            context
        ),
    }
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
fn test_sell_card_conservation() {
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
            let any_reached_16 = state.players.iter().any(|p| p.cached_score >= 16);
            assert!(
                any_reached_16 || state.round > 20,
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

// ── Random choice tests ──

#[test]
fn test_random_choice_invariants() {
    for seed in 100..200 {
        for num_players in 2..=4 {
            run_random_game_with_invariants(seed, num_players);
        }
    }
}

#[test]
fn test_random_choice_game_terminates() {
    for seed in 200..300 {
        for num_players in 2..=4 {
            let state = run_random_game_with_invariants(seed, num_players);
            let any_reached_16 = state.players.iter().any(|p| p.cached_score >= 16);
            assert!(
                any_reached_16 || state.round > 20,
                "Game ended without meeting termination condition (seed={}, players={}, round={})",
                seed,
                num_players,
                state.round
            );
        }
    }
}

// ── Stress tests ──

#[test]
fn test_stress_high_player_count() {
    for seed in 300..500 {
        run_random_game_with_invariants(seed, 4);
    }
}

#[test]
fn test_stress_two_player_long_games() {
    for seed in 500..700 {
        run_random_game_with_invariants(seed, 2);
    }
}

// ── Late-game tests ──

#[test]
fn test_draft_deck_recycling() {
    let mut max_round = 0u32;
    for seed in 700..800 {
        for num_players in 2..=4 {
            let state = run_random_game_with_invariants(seed, num_players);
            if state.round > max_round {
                max_round = state.round;
            }
        }
    }
    assert!(
        max_round >= 10,
        "No game reached round 10+ (max was {}), draft deck recycling not exercised",
        max_round
    );
}

// ── Fuzzing tests ──

#[test]
fn test_invalid_choices_rejected() {
    for seed in 800..830 {
        for num_players in 2..=4 {
            let mut rng = WyRand::seed_from_u64(seed);
            let ai_players = vec![true; num_players];
            let mut state = create_initial_game_state(num_players, &ai_players, &mut rng);

            execute_draw_phase(&mut state, &mut rng);

            let mut step = 0u32;
            loop {
                match state.phase {
                    GamePhase::GameOver => break,
                    GamePhase::Draw => {
                        execute_draw_phase(&mut state, &mut rng);
                        step += 1;
                        continue;
                    }
                    _ => {}
                }

                // Generate and test invalid choices
                let invalid_choices = generate_invalid_choices(&state);
                for invalid_choice in &invalid_choices {
                    assert!(
                        !check_choice_available(&state, invalid_choice),
                        "Invalid choice {:?} was accepted at step {} (seed={}, players={})",
                        invalid_choice,
                        step,
                        seed,
                        num_players
                    );
                }

                let choices = enumerate_choices(&state);
                assert!(!choices.is_empty());

                let idx = rng.random_range(0..choices.len());
                apply_choice_to_state(&mut state, &choices[idx], &mut rng);
                step += 1;

                if step >= 50_000 {
                    break;
                }
            }
        }
    }
}

// ── Serialization tests ──

#[test]
fn test_serialization_round_trip_preserves_state() {
    for seed in 900..930 {
        for num_players in 2..=4 {
            let mut rng = WyRand::seed_from_u64(seed);
            let ai_players = vec![true; num_players];
            let mut state = create_initial_game_state(num_players, &ai_players, &mut rng);

            execute_draw_phase(&mut state, &mut rng);

            let mut step = 0u32;
            loop {
                if step % 10 == 0 {
                    let restored = round_trip(&state);
                    assert_states_match(
                        &state,
                        &restored,
                        &format!(
                            "round-trip at step {} (seed={}, players={})",
                            step, seed, num_players
                        ),
                    );
                }

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
                assert!(!choices.is_empty());

                let idx = rng.random_range(0..choices.len());
                apply_choice_to_state(&mut state, &choices[idx], &mut rng);
                step += 1;

                if step >= 50_000 {
                    break;
                }
            }
        }
    }
}

#[test]
fn test_serialization_round_trip_determinism() {
    for seed in 930..960 {
        for num_players in 2..=4 {
            let mut rng = WyRand::seed_from_u64(seed);
            let ai_players = vec![true; num_players];
            let mut state = create_initial_game_state(num_players, &ai_players, &mut rng);

            execute_draw_phase(&mut state, &mut rng);

            let mut step = 0u32;
            loop {
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
                assert!(!choices.is_empty());

                let idx = rng.random_range(0..choices.len());
                let choice = &choices[idx];

                if step % 15 == 0 {
                    // Clone and round-trip the state
                    let mut original = state.clone();
                    let mut restored = round_trip(&state);

                    // Apply same choice with identically-seeded RNGs
                    let rng_seed = seed * 1_000_003 + step as u64;
                    let mut rng_a = WyRand::seed_from_u64(rng_seed);
                    let mut rng_b = WyRand::seed_from_u64(rng_seed);

                    apply_choice_to_state(&mut original, choice, &mut rng_a);
                    apply_choice_to_state(&mut restored, choice, &mut rng_b);

                    assert_states_match(
                        &original,
                        &restored,
                        &format!(
                            "determinism at step {} (seed={}, players={})",
                            step, seed, num_players
                        ),
                    );
                }

                apply_choice_to_state(&mut state, choice, &mut rng);
                step += 1;

                if step >= 50_000 {
                    break;
                }
            }
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
        90,
        "Draft deck should have 90 cards (54 dye + 12 material + 24 action)"
    );
}

#[test]
fn test_sell_card_deck_card_count() {
    for num_players in 2..=4 {
        let mut rng = WyRand::seed_from_u64(0);
        let ai_players = vec![true; num_players];
        let state = create_initial_game_state(num_players, &ai_players, &mut rng);
        let total_sell_cards = state.sell_card_deck.len() + state.sell_card_display.len() as u32;
        assert_eq!(
            total_sell_cards, 54,
            "Total sell cards should be 54 (players={})",
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
fn test_score_is_ducats() {
    for seed in 50..60 {
        for num_players in 2..=4 {
            let state = run_full_game_with_invariants(seed, num_players);
            for (i, player) in state.players.iter().enumerate() {
                let sell_card_ducats: u32 = player
                    .completed_sell_cards
                    .iter()
                    .map(|bi| bi.sell_card.ducats())
                    .sum();
                let expected = sell_card_ducats + player.ducats;
                assert_eq!(
                    calculate_score(player),
                    expected,
                    "Score mismatch for player {} (seed={}, players={}): sell_card_ducats={}, ability_ducats={}",
                    i,
                    seed,
                    num_players,
                    sell_card_ducats,
                    player.ducats
                );
            }
        }
    }
}
