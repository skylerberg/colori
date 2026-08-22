use colori_core::colori_game::{apply_choice_to_state, check_choice_available, enumerate_choices};
use colori_core::apply_choice::apply_choice;
use colori_core::deck::Deck;
use colori_core::draw_phase::execute_draw_phase;
use colori_core::scoring::calculate_score;
use colori_core::setup::create_initial_game_state;
use colori_core::types::{Ability, SellCard, Card, Choice, Color, GamePhase, GameState, MaterialType, ALL_MATERIAL_TYPES};
use colori_core::unordered_cards::UnorderedCards;
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
            invalid.push(Choice::MoveToDraftPool { card: None });
            // DraftPick with card types not in the current hand
            let hand = draft_state.hands[draft_state.current_player_index];
            let all_cards = [
                Card::BasicRed, Card::BasicYellow, Card::BasicBlue,
                Card::Lac,
                Card::Madder, Card::Turmeric, Card::DyersGreenweed,
                Card::VermilionDye, Card::Saffron, Card::PersianBerries,
                Card::StarterCeramics, Card::StarterPaintings, Card::StarterTextiles,
                Card::Alum, Card::CreamOfTartar, Card::GumArabic,
                Card::Potash, Card::Chalk,
                Card::LinseedOil, Card::Warehouse,
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
                invalid.push(Choice::MoveToDraftPool { card: None });
                invalid.push(Choice::GainMaterial {
                    material: MaterialType::Textiles,
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
                    Ability::GainMaterial => {
                        // Color gains are invalid while GainMaterial is on the stack
                        invalid.push(Choice::GainPrimary {
                            color: Color::Red,
                        });
                        invalid.push(Choice::GainSecondary {
                            color: Color::Orange,
                        });
                        invalid.push(Choice::EndTurn);
                        invalid.push(Choice::SkipWorkshop);
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
                    Ability::MoveToDraftPool => {
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
            assert_eq!(
                state.round,
                state.max_rounds + 1,
                "Game did not end after exactly {} rounds (seed={}, players={}, round={})",
                state.max_rounds,
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
            assert_eq!(
                state.round,
                state.max_rounds + 1,
                "Game did not end after exactly {} rounds (seed={}, players={}, round={})",
                state.max_rounds,
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
    // 4-player games deal 120 draft cards over 6 rounds from a 90-card deck,
    // so the destroyed pile must be recycled.
    for seed in 700..800 {
        let state = run_random_game_with_invariants(seed, 4);
        assert_eq!(
            state.round,
            state.max_rounds + 1,
            "Game did not complete all rounds (seed={}, round={})",
            seed,
            state.round
        );
    }
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

// ── Workshopped cards as destroy/move targets ──

/// Find a card id whose lookup matches `target` in any of the sources
/// (player decks, draft_deck) and return it along with a state ready for
/// action phase mutation.
fn find_card_id(state: &GameState, target: Card) -> u8 {
    for (idx, &c) in state.card_lookup.iter().enumerate() {
        if c == target {
            for player in state.players.iter() {
                if player.deck.contains(idx as u8) { return idx as u8; }
                if player.workshop_cards.contains(idx as u8) { return idx as u8; }
                if player.workshopped_cards.contains(idx as u8) { return idx as u8; }
                if player.drafted_cards.contains(idx as u8) { return idx as u8; }
            }
            if state.draft_deck.contains(idx as u8) { return idx as u8; }
        }
    }
    panic!("Could not find a {:?} in the game state", target);
}

fn remove_card_anywhere(state: &mut GameState, id: u8) {
    for player in state.players.iter_mut() {
        if player.deck.contains(id) { player.deck.remove(id); return; }
        if player.workshop_cards.contains(id) { player.workshop_cards.remove(id); return; }
        if player.workshopped_cards.contains(id) { player.workshopped_cards.remove(id); return; }
        if player.drafted_cards.contains(id) { player.drafted_cards.remove(id); return; }
    }
    if state.draft_deck.contains(id) { state.draft_deck.remove(id); return; }
    panic!("Could not find card id {} anywhere", id);
}

fn setup_action_state_with(
    alum_in_drafted: bool,
    workshopped_target: Option<Card>,
    workshop_target: Option<Card>,
    top_ability: Option<Ability>,
) -> (GameState, u8, Option<u8>, Option<u8>) {
    let mut rng = WyRand::seed_from_u64(12345);
    let mut state = create_initial_game_state(2, &[true, true], &mut rng);

    let alum_id = find_card_id(&state, Card::Alum);
    remove_card_anywhere(&mut state, alum_id);
    if alum_in_drafted {
        state.players[0].drafted_cards.insert(alum_id);
    }

    let ws_id = workshopped_target.map(|c| {
        let id = find_card_id(&state, c);
        remove_card_anywhere(&mut state, id);
        state.players[0].workshopped_cards.insert(id);
        id
    });

    let wsc_id = workshop_target.map(|c| {
        let id = find_card_id(&state, c);
        remove_card_anywhere(&mut state, id);
        state.players[0].workshop_cards.insert(id);
        id
    });

    let mut ability_stack = colori_core::types::AbilityStack::new();
    if let Some(a) = top_ability {
        ability_stack.push(a);
    }
    state.phase = GamePhase::Action {
        action_state: colori_core::types::ActionState {
            current_player_index: 0,
            ability_stack,
        },
    };

    (state, alum_id, ws_id, wsc_id)
}

#[test]
fn test_destroy_cards_enumerates_workshopped_target() {
    let (state, _alum_id, _ws_id, _wsc_id) = setup_action_state_with(
        false,
        Some(Card::StarterCeramics),
        None,
        Some(Ability::MoveToDraftPool),
    );

    let choices = enumerate_choices(&state);
    let wanted = Choice::MoveToDraftPool { card: Some(Card::StarterCeramics) };
    assert!(
        choices.iter().any(|c| matches!(c, Choice::MoveToDraftPool { card: Some(Card::StarterCeramics) })),
        "enumerate_choices should include workshopped-card target; got {:?}",
        choices
    );
    assert!(check_choice_available(&state, &wanted));
}

#[test]
fn test_destroy_cards_skip_when_both_areas_empty() {
    let (state, _, _, _) = setup_action_state_with(false, None, None, Some(Ability::MoveToDraftPool));
    let choices = enumerate_choices(&state);
    assert_eq!(choices.len(), 1);
    assert!(matches!(choices[0], Choice::MoveToDraftPool { card: None }));
}

#[test]
fn test_gain_material_enumerates_every_material_type() {
    let (state, _, _, _) = setup_action_state_with(false, None, None, Some(Ability::GainMaterial));

    let choices = enumerate_choices(&state);
    assert_eq!(choices.len(), ALL_MATERIAL_TYPES.len());
    for &material in ALL_MATERIAL_TYPES.iter() {
        let wanted = Choice::GainMaterial { material };
        assert!(
            choices.contains(&wanted),
            "enumerate_choices should include GainMaterial for {:?}; got {:?}",
            material,
            choices
        );
        assert!(check_choice_available(&state, &wanted));
    }
}

#[test]
fn test_gain_material_stores_the_chosen_material() {
    let (mut state, _, _, _) =
        setup_action_state_with(false, None, None, Some(Ability::GainMaterial));
    let before = state.players[0].materials.get(MaterialType::Paintings);

    let mut rng = WyRand::seed_from_u64(7);
    apply_choice_to_state(
        &mut state,
        &Choice::GainMaterial { material: MaterialType::Paintings },
        &mut rng,
    );

    assert_eq!(
        state.players[0].materials.get(MaterialType::Paintings),
        before + 1
    );
    if let GamePhase::Action { ref action_state } = state.phase {
        assert!(action_state.ability_stack.is_empty());
    } else {
        panic!("Expected action phase");
    }
}

#[test]
fn test_destroy_and_destroy_cards_enumerates_workshopped_target() {
    // No ability on the stack — enumerating destroy-drafted choices for Alum.
    let (state, _, _, _) = setup_action_state_with(
        true,
        Some(Card::StarterCeramics),
        None,
        None,
    );

    let choices = enumerate_choices(&state);
    let wanted = Choice::DestroyAndMoveToDraftPool {
        card: Card::Alum,
        target: Some(Card::StarterCeramics),
    };
    assert!(
        choices.iter().any(|c| matches!(
            c,
            Choice::DestroyAndMoveToDraftPool { card: Card::Alum, target: Some(Card::StarterCeramics) }
        )),
        "enumerate_choices should include DestroyAndMoveToDraftPool targeting workshopped card; got {:?}",
        choices
    );
    assert!(check_choice_available(&state, &wanted));
}

/// A rotated workshop card can be moved, and the move does not destroy it:
/// it lands in the draft pool, where the ordinary destroy path can pick it up
/// later — or leave it there for end of turn to return to the workshop.
#[test]
fn test_apply_move_to_draft_pool_moves_workshopped_card_without_destroying_it() {
    let (mut state, _, ws_id, _) = setup_action_state_with(
        false,
        Some(Card::StarterCeramics),
        None,
        Some(Ability::MoveToDraftPool),
    );
    let ws_id = ws_id.unwrap();

    let mut rng = WyRand::seed_from_u64(99);
    apply_choice_to_state(
        &mut state,
        &Choice::MoveToDraftPool { card: Some(Card::StarterCeramics) },
        &mut rng,
    );

    assert!(!state.players[0].workshopped_cards.contains(ws_id),
        "moved card should have left workshopped_cards");
    assert!(state.players[0].drafted_cards.contains(ws_id),
        "moved card should be in the draft pool");
    assert!(!state.destroyed_pile.contains(ws_id),
        "the move must not destroy the card");
    if let GamePhase::Action { ref action_state } = state.phase {
        assert!(action_state.ability_stack.is_empty(),
            "the move resolves the ability and triggers nothing");
    } else {
        panic!("Expected action phase");
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

#[test]
fn test_workshopping_warehouse_pushes_gain_material() {
    let (mut state, _, _, wsc_id) = setup_action_state_with(
        false,
        None,
        Some(Card::Warehouse),
        Some(Ability::Workshop { count: 1 }),
    );
    let wsc_id = wsc_id.unwrap();

    let mut rng = WyRand::seed_from_u64(3);
    apply_choice_to_state(
        &mut state,
        &Choice::Workshop { card_types: SmallVec::from_slice(&[Card::Warehouse]) },
        &mut rng,
    );

    assert!(state.players[0].workshopped_cards.contains(wsc_id),
        "Warehouse should have rotated into workshopped_cards");
    if let GamePhase::Action { ref action_state } = state.phase {
        assert_eq!(
            action_state.ability_stack.last(),
            Some(&Ability::GainMaterial),
            "Workshopping Warehouse should leave GainMaterial awaiting input"
        );
    } else {
        panic!("Expected action phase");
    }

    let choices = enumerate_choices(&state);
    assert_eq!(choices.len(), ALL_MATERIAL_TYPES.len());

    let before = state.players[0].materials.get(MaterialType::Ceramics);
    apply_choice_to_state(
        &mut state,
        &Choice::GainMaterial { material: MaterialType::Ceramics },
        &mut rng,
    );
    assert_eq!(
        state.players[0].materials.get(MaterialType::Ceramics),
        before + 1
    );
}

#[test]
fn test_draft_deck_composition() {
    let mut rng = WyRand::seed_from_u64(1);
    let state = create_initial_game_state(2, &[true, true], &mut rng);
    assert_eq!(state.draft_deck.len(), 90, "draft deck should hold 90 cards");

    let mut action_count = 0usize;
    let mut warehouse_count = 0usize;
    for id in state.draft_deck.iter() {
        let card = state.card_lookup[id as usize];
        if card.is_action() {
            action_count += 1;
        }
        if card == Card::Warehouse {
            warehouse_count += 1;
        }
    }
    assert_eq!(action_count, 24, "24 action cards in the draft deck");
    assert_eq!(warehouse_count, 4, "4 copies of Warehouse");
}

// ── End-of-round deck handling ──

/// Play a two-player game to the end of player 0's first turn, choosing
/// `EndTurn` as soon as it is legal so the sweep is reached with cards still
/// in both areas.
fn state_at_first_end_of_turn(seed: u64) -> (GameState, UnorderedCards, UnorderedCards) {
    let mut rng = WyRand::seed_from_u64(seed);
    let mut state = create_initial_game_state(2, &[true, true], &mut rng);
    execute_draw_phase(&mut state, &mut rng);

    // Finish the draft by always taking the first option.
    while matches!(state.phase, GamePhase::Draft { .. }) {
        let choices = enumerate_choices(&state);
        apply_choice_to_state(&mut state, &choices[0], &mut rng);
    }

    let workshop_area = state.players[0]
        .workshop_cards
        .union(state.players[0].workshopped_cards);
    let drafted = state.players[0].drafted_cards;
    assert!(!workshop_area.is_empty() && !drafted.is_empty());

    apply_choice_to_state(&mut state, &Choice::EndTurn, &mut rng);
    (state, workshop_area, drafted)
}

#[test]
fn test_end_of_turn_sends_the_workshop_to_the_bottom_of_the_deck() {
    let (state, workshop_area, _) = state_at_first_end_of_turn(4);
    let player = &state.players[0];

    let bottom = *player.deck.segments().last().expect("deck has a segment");
    assert_eq!(
        bottom, workshop_area,
        "the whole workshop area should form the deck's bottom segment"
    );
    assert!(player.workshopped_cards.is_empty());
}

#[test]
fn test_end_of_turn_moves_the_draft_pool_into_the_workshop() {
    let (state, _, drafted) = state_at_first_end_of_turn(4);
    let player = &state.players[0];

    assert_eq!(player.workshop_cards, drafted);
    assert!(player.drafted_cards.is_empty());
    assert!(
        player.deck.cards().intersection(drafted).is_empty(),
        "kept draft-pool cards go to the workshop, not into the deck"
    );
}

/// The point of the segmented deck: cards placed on the bottom are not seen
/// again until everything above them has been drawn.
#[test]
fn test_cards_put_on_the_bottom_are_drawn_after_what_was_above_them() {
    let (mut state, workshop_area, _) = state_at_first_end_of_turn(4);
    let above = state.players[0].deck.segments()[0];
    assert!(!above.is_empty(), "seed should leave cards above the new segment");

    let mut rng = WyRand::seed_from_u64(1);
    let drawn_first = {
        let player = &mut state.players[0];
        let mut hand = UnorderedCards::new();
        player.deck.draw_into(&mut hand, above.len(), &mut rng);
        hand
    };

    assert_eq!(drawn_first, above);
    assert!(
        drawn_first.intersection(workshop_area).is_empty(),
        "the bottom segment was reached before the top ran out"
    );
}

#[test]
fn test_draw_phase_tops_the_workshop_up_to_five() {
    let (mut state, _, drafted) = state_at_first_end_of_turn(4);
    let mut rng = WyRand::seed_from_u64(2);

    // Finish player 1's turn so the round ends. `apply_choice` is the raw
    // engine call: `apply_choice_to_state` would run the next draw phase for
    // us, and the carried-over workshop is what this test wants to see first.
    while !matches!(state.phase, GamePhase::Draw) {
        let choices = enumerate_choices(&state);
        apply_choice(&mut state, &choices[0], &mut rng);
    }
    let carried = state.players[0].workshop_cards;
    assert_eq!(carried, drafted);

    execute_draw_phase(&mut state, &mut rng);
    assert_eq!(
        state.players[0].workshop_cards.len(),
        5,
        "the draw phase should top the workshop up to five, not add five to it"
    );
    assert!(
        carried.difference(state.players[0].workshop_cards).is_empty(),
        "carried-over cards should still be in the workshop"
    );
}

/// With no discard pile there is nothing to reshuffle, so a draw that outruns
/// the deck simply yields fewer cards.
#[test]
fn test_a_dry_deck_yields_fewer_cards_rather_than_reshuffling() {
    let mut rng = WyRand::seed_from_u64(7);
    let mut state = create_initial_game_state(2, &[true, true], &mut rng);
    let owned = state.players[0].deck.cards();
    state.players[0].deck = Deck::from_cards(owned);

    execute_draw_phase(&mut state, &mut rng);
    assert_eq!(state.players[0].workshop_cards.len(), 5);
    assert_eq!(state.players[0].deck.len(), 2);

    let player = &mut state.players[0];
    let mut hand = UnorderedCards::new();
    assert_eq!(player.deck.draw_into(&mut hand, 4, &mut rng), 2);
    assert!(player.deck.is_empty());
    assert_eq!(player.deck.draw_into(&mut hand, 1, &mut rng), 0);
}

/// A six-round game can create at most seven segments — the starting deck plus
/// one per round end — so `MAX_DECK_SEGMENTS` is never reached in real play.
#[test]
fn test_decks_stay_well_inside_the_segment_cap() {
    for seed in 0..30 {
        for num_players in 2..=4 {
            let state = run_random_game_with_invariants(seed, num_players);
            for (pi, player) in state.players.iter().enumerate() {
                assert!(
                    player.deck.segment_count() <= 7,
                    "player {pi} ended with {} deck segments (seed={seed}, players={num_players})",
                    player.deck.segment_count(),
                );
            }
        }
    }
}
