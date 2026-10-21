use colori_core::colori_game::{
    apply_choice_to_state, check_choice_available, enumerate_choices, get_game_status,
    ActionPhaseViews, GameStatus,
};
use colori_core::apply_choice::apply_choice;
use colori_core::buyers_phase::initialize_buyers_phase;
use colori_core::deck::{Deck, MAX_DECK_SEGMENTS};
use colori_core::draft_phase::DRAFT_HAND_SIZE;
use colori_core::game_log::{DrawEvent, DrawLog};
use colori_core::draw_phase::{execute_draw_phase, round_income, RoundIncome};
use colori_core::cards::{draft_copies, draft_deck_cards, sell_card_copies, sell_deck_cards, starter_deck};
use colori_core::scoring::{calculate_whole_points, score_in_half_points};
use colori_core::setup::create_initial_game_state;
use colori_core::types::{Ability, AbilityStack, ActionState, Buyer, SellCard, Card, Choice, Color, ColorWheel, DucatPurchase, GamePhase, GameState, MaterialType, PlayerState, PlayerTurn, ALL_SELL_CARDS, MAX_SELL_CARD_DISPLAY};
use colori_core::unordered_cards::{UnorderedCards, UnorderedSellCards};
use colori_core::unordered_cards::{
    get_sell_card_registry, get_card_registry, set_sell_card_registry, set_card_registry,
};
use rand::RngExt;
use rand::SeedableRng;
use colori_core::fixed_vec::CopyVec;
use wyrand::WyRand;

// ── Helpers ──

/// The lowest seat with a decision to make.
fn actor(state: &GameState) -> usize {
    match get_game_status(state, None) {
        GameStatus::AwaitingAction { player_index } => player_index,
        GameStatus::Terminated { .. } => panic!("No one acts in a finished game"),
    }
}

/// A seat with a decision to make, picked at random among everyone still
/// taking their action turn so that turns interleave.
fn random_actor(state: &GameState, rng: &mut WyRand) -> usize {
    match &state.phase {
        GamePhase::Action { action_state } => {
            let open: Vec<usize> = (0..action_state.turns.len())
                .filter(|&p| !action_state.turns[p].ended)
                .collect();
            open[rng.random_range(0..open.len())]
        }
        _ => actor(state),
    }
}

fn count_all_cards(state: &GameState) -> u32 {
    let mut total = 0u32;

    for player in state.players.iter() {
        total += player.deck.len();
        total += player.workshopped_cards.len();
        total += player.workshop_cards.len();
        total += player.drafted_cards.len();
    }

    total += state.draft_deck.len();
    total += state.trash.len();

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
        total += player.buyers.len() as u32;
        total += player.completed_sell_cards.len() as u32;
    }

    total
}

fn check_no_negative_resources(state: &GameState, step: u32, seed: u64, num_players: usize) {
    for (pi, player) in state.players.iter().enumerate() {
        for &count in player.color_wheel.counts.iter() {
            assert!(
                count < 200,
                "Player {} has color wheel count {} (likely u8 underflow) at step {} (seed={}, players={})",
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
                player.cached_whole_points,
                calculate_whole_points(player),
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

        let player = actor(&state);
        let choices = enumerate_choices(&state, player);
        assert!(
            !choices.is_empty(),
            "No choices available at step {} (seed={}, players={})",
            step,
            seed,
            num_players
        );

        for choice in &choices {
            assert!(
                check_choice_available(&state, player, choice),
                "Choice {:?} from enumerate_choices failed check_choice_available at step {} (seed={}, players={})",
                choice, step, seed, num_players
            );
        }

        apply_choice_to_state(&mut state, player, &choices[0], &mut rng);
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
                player.cached_whole_points,
                calculate_whole_points(player),
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

        let player = random_actor(&state, &mut rng);
        let choices = enumerate_choices(&state, player);
        assert!(
            !choices.is_empty(),
            "No choices available at step {} (seed={}, players={})",
            step,
            seed,
            num_players
        );

        for choice in &choices {
            assert!(
                check_choice_available(&state, player, choice),
                "Choice {:?} from enumerate_choices failed check_choice_available at step {} (seed={}, players={})",
                choice, step, seed, num_players
            );
        }

        let idx = rng.random_range(0..choices.len());
        apply_choice_to_state(&mut state, player, &choices[idx], &mut rng);
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

fn generate_invalid_choices(state: &GameState, player: usize) -> Vec<Choice> {
    let mut invalid = Vec::new();

    match &state.phase {
        GamePhase::Draft { draft_state } => {
            // Wrong-phase choices
            invalid.push(Choice::EndTurn);
            invalid.push(Choice::SkipWorkshop);
            invalid.push(Choice::SelectSellCard {
                sell_card: SellCard::IlluminatorVermilion,
            });
            invalid.push(Choice::GainSecondary {
                color: Color::Orange,
            });
            invalid.push(Choice::MixAll {
                mixes: CopyVec::new(),
            });
            invalid.push(Choice::MoveToDraftPool { card: None });
            // DraftPick with card types not in the current hand
            let hand = draft_state.hands[player];
            let all_cards = [
                Card::BasicRed, Card::BasicYellow, Card::BasicBlue,
                Card::Realgar,
                Card::DragonsBlood, Card::Orpiment, Card::GreenEarth,
                Card::Cinnabar, Card::Saffron, Card::CopperResinate,
                Card::StarterCeramics, Card::StarterCanvas, Card::StarterFabric,
                Card::CeramicsRed, Card::FabricOrange, Card::CanvasCeramics,
                Card::Alum, Card::GumArabic, Card::Gondola, Card::Chalk,
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

            let stack = &action_state.turns[player].ability_stack;
            if stack.is_empty() {
                // Empty stack: these require specific abilities on stack
                invalid.push(Choice::SkipWorkshop);
                invalid.push(Choice::SelectSellCard {
                    sell_card: SellCard::IlluminatorVermilion,
                });
                invalid.push(Choice::GainSecondary {
                    color: Color::Orange,
                });
                invalid.push(Choice::MixAll {
                    mixes: CopyVec::new(),
                });
                invalid.push(Choice::MoveToDraftPool { card: None });
            } else {
                let top = stack.last().unwrap();
                match top {
                    Ability::Workshop { .. } => {
                        invalid.push(Choice::EndTurn);
                        invalid.push(Choice::SelectSellCard {
                            sell_card: SellCard::IlluminatorVermilion,
                        });
                        invalid.push(Choice::GainSecondary {
                            color: Color::Orange,
                        });
                        // Empty workshop is invalid
                        invalid.push(Choice::Workshop {
                            card_types: CopyVec::new(),
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
                    Ability::MixColors { .. } => {
                        // Non-adjacent colors are invalid
                        invalid.push(Choice::MixAll {
                            mixes: CopyVec::from_slice(&[(Color::Orange, Color::Green)]),
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
                    Ability::GainDucats { .. }
                    | Ability::GainWildPrimaries { .. }
                    | Ability::GainBoats { .. } => {
                        // Auto-resolved, but just in case
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
        p.cached_whole_points = calculate_whole_points(p);
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
            pa.cached_whole_points, pb.cached_whole_points,
            "player {} cached_whole_points mismatch: {}",
            pi, context
        );
        assert_eq!(
            pa.buyers.len(),
            pb.buyers.len(),
            "player {} buyers length mismatch: {}",
            pi, context
        );
        for (bi, (ba, bb)) in pa.buyers.iter().zip(pb.buyers.iter()).enumerate() {
            assert_eq!(
                ba.instance_id, bb.instance_id,
                "player {} buyer {} instance_id mismatch: {}",
                pi, bi, context
            );
            assert_eq!(
                ba.sell_card, bb.sell_card,
                "player {} buyer {} card mismatch: {}",
                pi, bi, context
            );
        }
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
        a.trash, b.trash,
        "trash mismatch: {}",
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
            GamePhase::Buyers { buyers_state: ba },
            GamePhase::Buyers { buyers_state: bb },
        ) => {
            assert_eq!(
                ba.current_player_index, bb.current_player_index,
                "buyers current_player_index mismatch: {}",
                context
            );
        }
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
                aa.turns.len(),
                ab.turns.len(),
                "action turn count mismatch: {}",
                context
            );
            for (p, (ta, tb)) in aa.turns.iter().zip(ab.turns.iter()).enumerate() {
                assert_eq!(ta.ended, tb.ended, "player {} turn ended mismatch: {}", p, context);
                assert_eq!(
                    ta.ability_stack, tb.ability_stack,
                    "player {} ability stack mismatch: {}",
                    p, context
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
fn test_cached_whole_points_consistency() {
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
    // 4-player games deal 112 draft cards over 7 rounds from a 100-card deck,
    // so the trash must be recycled.
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
                let player = random_actor(&state, &mut rng);
                let invalid_choices = generate_invalid_choices(&state, player);
                for invalid_choice in &invalid_choices {
                    assert!(
                        !check_choice_available(&state, player, invalid_choice),
                        "Invalid choice {:?} was accepted at step {} (seed={}, players={})",
                        invalid_choice,
                        step,
                        seed,
                        num_players
                    );
                }

                let choices = enumerate_choices(&state, player);
                assert!(!choices.is_empty());

                let idx = rng.random_range(0..choices.len());
                apply_choice_to_state(&mut state, player, &choices[idx], &mut rng);
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

                let player = random_actor(&state, &mut rng);
                let choices = enumerate_choices(&state, player);
                assert!(!choices.is_empty());

                let idx = rng.random_range(0..choices.len());
                apply_choice_to_state(&mut state, player, &choices[idx], &mut rng);
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

                let player = random_actor(&state, &mut rng);
                let choices = enumerate_choices(&state, player);
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

                    apply_choice_to_state(&mut original, player, choice, &mut rng_a);
                    apply_choice_to_state(&mut restored, player, choice, &mut rng_b);

                    assert_states_match(
                        &original,
                        &restored,
                        &format!(
                            "determinism at step {} (seed={}, players={})",
                            step, seed, num_players
                        ),
                    );
                }

                apply_choice_to_state(&mut state, player, choice, &mut rng);
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
        100,
        "Draft deck should have 100 cards (54 dye + 21 material + 25 action)"
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
            total_sell_cards, 75,
            "Total sell cards should be 75 (players={})",
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
            Color::Ultramarine,
            Color::Purple,
            Color::Fuchsia,
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

// ── Workshopped cards as trash/move targets ──

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

/// An action phase with every seat's turn open and `stack` pending for
/// player 0.
fn open_action_phase(state: &GameState, stack: AbilityStack) -> GamePhase {
    let mut turns: colori_core::fixed_vec::FixedVec<PlayerTurn, { colori_core::types::MAX_PLAYERS }> =
        state.players.iter().map(|_| PlayerTurn::default()).collect();
    turns[0].ability_stack = stack;
    GamePhase::Action { action_state: ActionState { turns } }
}

fn setup_action_state_with(
    alum_in_drafted: bool,
    workshopped_target: Option<Card>,
    workshop_target: Option<Card>,
    top_ability: Option<Ability>,
) -> (GameState, u8, Option<u8>, Option<u8>) {
    let mut rng = WyRand::seed_from_u64(12345);
    let mut state = create_initial_game_state(2, &[true, true], &mut rng);
    // Count what a card or sale gives from an empty purse.
    let player = &mut state.players[0];
    player.ducats = 0;
    player.boats = 0;
    player.cached_whole_points = 0;

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

    let mut ability_stack = AbilityStack::new();
    if let Some(a) = top_ability {
        ability_stack.push(a);
    }
    state.phase = open_action_phase(&state, ability_stack);

    (state, alum_id, ws_id, wsc_id)
}

#[test]
fn test_move_to_draft_pool_targets_only_unworkshopped_cards() {
    let (state, _, _, _) = setup_action_state_with(
        false,
        Some(Card::StarterCeramics),
        Some(Card::StarterCanvas),
        Some(Ability::MoveToDraftPool),
    );

    let canvas = Choice::MoveToDraftPool { card: Some(Card::StarterCanvas) };
    let ceramics = Choice::MoveToDraftPool { card: Some(Card::StarterCeramics) };
    assert_eq!(enumerate_choices(&state, 0), vec![canvas.clone()]);
    assert!(check_choice_available(&state, 0, &canvas));
    assert!(!check_choice_available(&state, 0, &ceramics), "a workshopped card cannot be moved");
    assert!(
        !check_choice_available(&state, 0, &Choice::MoveToDraftPool { card: None }),
        "the move is mandatory while there is a card to move"
    );
}

#[test]
fn test_move_to_draft_pool_fizzles_when_only_workshopped_cards_remain() {
    let (state, _, _, _) = setup_action_state_with(
        false,
        Some(Card::StarterCeramics),
        None,
        Some(Ability::MoveToDraftPool),
    );
    assert_eq!(
        enumerate_choices(&state, 0),
        vec![Choice::MoveToDraftPool { card: None }]
    );
}

#[test]
fn test_trashing_alum_targets_only_unworkshopped_cards() {
    // No ability on the stack — enumerating trash-drafted choices for Alum.
    let (state, _, _, _) = setup_action_state_with(
        true,
        Some(Card::StarterCeramics),
        Some(Card::StarterCanvas),
        None,
    );

    let choices = enumerate_choices(&state, 0);
    let moves: Vec<&Choice> = choices
        .iter()
        .filter(|c| matches!(c, Choice::TrashAndMoveToDraftPool { .. }))
        .collect();
    let canvas = Choice::TrashAndMoveToDraftPool {
        card: Card::Alum,
        target: Some(Card::StarterCanvas),
    };
    assert_eq!(moves, vec![&canvas]);
    assert!(check_choice_available(&state, 0, &canvas));
    assert!(!check_choice_available(
        &state,
        0,
        &Choice::TrashAndMoveToDraftPool { card: Card::Alum, target: Some(Card::StarterCeramics) }
    ));
    assert!(!check_choice_available(
        &state,
        0,
        &Choice::TrashAndMoveToDraftPool { card: Card::Alum, target: None }
    ));
}

/// The move does not trash the card: it lands in the draft pool, where the
/// ordinary trash path can pick it up later — or leave it there for end of
/// turn to return to the workshop.
#[test]
fn test_apply_move_to_draft_pool_moves_the_card_without_trashing_it() {
    let (mut state, _, _, wsc_id) = setup_action_state_with(
        false,
        None,
        Some(Card::StarterCanvas),
        Some(Ability::MoveToDraftPool),
    );
    let wsc_id = wsc_id.unwrap();

    let mut rng = WyRand::seed_from_u64(99);
    apply_choice_to_state(
        &mut state,
        0,
        &Choice::MoveToDraftPool { card: Some(Card::StarterCanvas) },
        &mut rng,
    );

    assert!(!state.players[0].workshop_cards.contains(wsc_id),
        "moved card should have left the workshop");
    assert!(state.players[0].drafted_cards.contains(wsc_id),
        "moved card should be in the draft pool");
    assert!(!state.trash.contains(wsc_id),
        "the move must not trash the card");
    if let GamePhase::Action { ref action_state } = state.phase {
        assert!(action_state.turns[0].ability_stack.is_empty(),
            "the move resolves the ability and triggers nothing");
    } else {
        panic!("Expected action phase");
    }
}

#[test]
fn test_score_is_points_plus_half_points_for_wild_primaries_and_boats() {
    for seed in 50..60 {
        for num_players in 2..=4 {
            let state = run_full_game_with_invariants(seed, num_players);
            for (i, player) in state.players.iter().enumerate() {
                let sell_card_points: u32 = player
                    .completed_sell_cards
                    .iter()
                    .map(|bi| bi.sell_card.points())
                    .sum();
                assert_eq!(
                    calculate_whole_points(player),
                    sell_card_points + player.ducats,
                    "Whole points mismatch for player {} (seed={}, players={})",
                    i, seed, num_players,
                );
                assert_eq!(
                    score_in_half_points(player),
                    2 * (sell_card_points + player.ducats) + player.wild_primaries + player.boats,
                    "Score mismatch for player {} (seed={}, players={})",
                    i, seed, num_players,
                );
            }
        }
    }
}

/// Workshop `card` from player 0's workshop with a one-pick Workshop pending.
fn state_after_workshopping(card: Card) -> GameState {
    let (mut state, _, _, _) = setup_action_state_with(
        false,
        None,
        Some(card),
        Some(Ability::Workshop { count: 1 }),
    );
    let mut rng = WyRand::seed_from_u64(3);
    apply_choice_to_state(
        &mut state,
        0,
        &Choice::Workshop { card_types: CopyVec::from_slice(&[card]) },
        &mut rng,
    );
    state
}

fn pending_ability(state: &GameState) -> Option<Ability> {
    match &state.phase {
        GamePhase::Action { action_state } => action_state.turns[0].ability_stack.last().copied(),
        _ => panic!("Expected action phase"),
    }
}

#[test]
fn test_workshopping_warehouse_gains_two_wild_primaries() {
    let state = state_after_workshopping(Card::Warehouse);
    assert_eq!(state.players[0].wild_primaries, 2);
    assert_eq!(pending_ability(&state), None, "wild primaries arrive without a choice");
    assert_eq!(score_in_half_points(&state.players[0]), 2);
}

#[test]
fn test_workshopping_chalk_gains_one_wild_primary() {
    let state = state_after_workshopping(Card::Chalk);
    assert_eq!(state.players[0].wild_primaries, 1);
    assert_eq!(pending_ability(&state), None);
}

#[test]
fn test_workshopping_gondola_gains_a_boat() {
    let state = state_after_workshopping(Card::Gondola);
    assert_eq!(state.players[0].boats, 1);
    assert_eq!(pending_ability(&state), None);
    assert_eq!(score_in_half_points(&state.players[0]), 1);
}

#[test]
fn test_linseed_oil_grants_three_mixes_in_one_choice() {
    let mut state = state_after_workshopping(Card::LinseedOil);
    assert_eq!(pending_ability(&state), Some(Ability::MixColors { count: 3 }));

    let wheel = &mut state.players[0].color_wheel;
    *wheel = ColorWheel::new();
    wheel.set(Color::Red, 2);
    wheel.set(Color::Yellow, 2);
    wheel.set(Color::Blue, 1);

    // Red+Yellow -> Orange, Red+Orange -> Vermilion, Yellow+Blue -> Green.
    let mixes = CopyVec::from_slice(&[
        (Color::Red, Color::Yellow),
        (Color::Red, Color::Orange),
        (Color::Yellow, Color::Blue),
    ]);
    let three_mix_outcomes = enumerate_choices(&state, 0)
        .iter()
        .filter(|c| matches!(c, Choice::MixAll { mixes } if mixes.len() == 3))
        .count();
    assert!(three_mix_outcomes > 0, "a three-mix sequence should be offered");

    let choice = Choice::MixAll { mixes };
    assert!(check_choice_available(&state, 0, &choice));
    let mut rng = WyRand::seed_from_u64(3);
    apply_choice_to_state(&mut state, 0, &choice, &mut rng);

    let wheel = &state.players[0].color_wheel;
    assert_eq!(wheel.get(Color::Vermilion), 1);
    assert_eq!(wheel.get(Color::Green), 1);
    assert_eq!(wheel.counts.iter().map(|&c| c as u32).sum::<u32>(), 2);
    assert_eq!(pending_ability(&state), None);
}

#[test]
fn test_mix_sequences_offer_each_outcome_once() {
    let mut state = state_after_workshopping(Card::LinseedOil);
    let wheel = &mut state.players[0].color_wheel;
    *wheel = ColorWheel::new();
    wheel.set(Color::Red, 1);
    wheel.set(Color::Yellow, 1);
    wheel.set(Color::Blue, 1);
    wheel.set(Color::Orange, 1);

    let choices = enumerate_choices(&state, 0);
    let mut outcomes = Vec::new();
    for choice in &choices {
        let mut after = state.clone();
        let mut rng = WyRand::seed_from_u64(3);
        apply_choice_to_state(&mut after, 0, choice, &mut rng);
        let outcome = (after.players[0].color_wheel.counts, after.players[0].wild_primaries);
        assert!(!outcomes.contains(&outcome), "{:?} repeats an earlier outcome", choice);
        outcomes.push(outcome);
    }
    // Yellow+Blue then Red+Orange lands where Red+Orange then Yellow+Blue does.
    assert!(choices.len() > 1);
}

#[test]
fn test_using_every_mix_leaves_a_mix_beneath_it_alone() {
    let (mut state, _, _, _) = setup_action_state_with(false, None, None, None);
    let wheel = &mut state.players[0].color_wheel;
    *wheel = ColorWheel::new();
    // Red pairs with Yellow or with Orange, so the mix below is one the wheel
    // can still make once the first ability has been used.
    wheel.set(Color::Red, 2);
    wheel.set(Color::Yellow, 1);
    wheel.set(Color::Blue, 1);
    if let GamePhase::Action { ref mut action_state } = state.phase {
        action_state.turns[0].ability_stack.push(Ability::MixColors { count: 1 });
        action_state.turns[0].ability_stack.push(Ability::MixColors { count: 1 });
    }

    let mut rng = WyRand::seed_from_u64(3);
    apply_choice_to_state(
        &mut state,
        0,
        &Choice::MixAll { mixes: CopyVec::from_slice(&[(Color::Red, Color::Yellow)]) },
        &mut rng,
    );
    assert_eq!(
        pending_ability(&state),
        Some(Ability::MixColors { count: 1 }),
        "the second mix ability is still owed"
    );
}

/// A mix the wheel cannot supply has one answer — mix nothing — so it is taken
/// with the trash that granted it rather than asked about.
///
/// The ability arrives by the route that made it an offer: a trash whose first
/// ability is a Workshop leaves the mix it granted beneath the picks.
#[test]
fn a_mix_the_wheel_cannot_supply_never_becomes_a_decision() {
    let mut state = state_with_drafted(Card::CeramicsRed);
    state.players[0].color_wheel = ColorWheel::new();
    state.players[0].wild_primaries = 0;
    let trash = Choice::TrashAndWorkshop {
        card: Card::CeramicsRed,
        workshop_cards: CopyVec::new(),
    };
    assert!(enumerate_choices(&state, 0).contains(&trash));

    let mut rng = WyRand::seed_from_u64(0);
    apply_choice_to_state(&mut state, 0, &trash, &mut rng);
    assert_eq!(
        pending_ability(&state),
        None,
        "the mix ability went with the trash that could not use it"
    );
    assert!(
        enumerate_choices(&state, 0).contains(&Choice::EndTurn),
        "what is left is the turn's own menu"
    );
}

/// The ability a mix sequence answers may already be gone, because it could mix
/// nothing and was taken for the player. A game logged before that was true
/// still replays: its empty sequence asks for exactly what already happened.
#[test]
fn an_empty_mix_sequence_with_no_ability_left_does_nothing() {
    let mut state = state_with_drafted(Card::CeramicsRed);
    state.players[0].color_wheel = ColorWheel::new();
    state.players[0].wild_primaries = 0;
    let mut rng = WyRand::seed_from_u64(0);
    apply_choice_to_state(
        &mut state,
        0,
        &Choice::TrashAndWorkshop {
            card: Card::CeramicsRed,
            workshop_cards: CopyVec::new(),
        },
        &mut rng,
    );
    let before = state.players[0].clone();

    apply_choice_to_state(&mut state, 0, &Choice::MixAll { mixes: CopyVec::new() }, &mut rng);
    assert_eq!(state.players[0].color_wheel, before.color_wheel);
    assert_eq!(state.players[0].wild_primaries, before.wild_primaries);
    assert_eq!(pending_ability(&state), None);
}

/// Put `card` in player 0's draft pool with the action phase open and nothing
/// pending.
fn state_with_drafted(card: Card) -> GameState {
    let (mut state, _, _, _) = setup_action_state_with(false, None, None, None);
    let id = find_card_id(&state, card);
    remove_card_anywhere(&mut state, id);
    state.players[0].drafted_cards.insert(id);
    state
}

#[test]
fn test_trashing_an_imports_card_workshops_then_mixes() {
    let mut state = state_with_drafted(Card::CanvasCeramics);
    let fabric = find_card_id(&state, Card::StarterFabric);
    remove_card_anywhere(&mut state, fabric);
    state.players[0].workshop_cards.insert(fabric);

    let mut rng = WyRand::seed_from_u64(3);
    let trash = Choice::TrashAndWorkshop {
        card: Card::CanvasCeramics,
        workshop_cards: CopyVec::from_slice(&[Card::StarterFabric]),
    };
    assert!(enumerate_choices(&state, 0).contains(&trash));
    apply_choice_to_state(&mut state, 0, &trash, &mut rng);

    assert_eq!(state.players[0].materials.get(MaterialType::Textiles), 1);
    assert_eq!(
        pending_ability(&state),
        Some(Ability::MixColors { count: 1 }),
        "the mix follows the workshop"
    );

    apply_choice_to_state(
        &mut state,
        0,
        &Choice::MixAll { mixes: CopyVec::from_slice(&[(Color::Red, Color::Blue)]) },
        &mut rng,
    );
    assert_eq!(state.players[0].color_wheel.get(Color::Purple), 1);
    assert_eq!(pending_ability(&state), None);
}

#[test]
fn test_an_imports_card_still_mixes_when_the_workshop_is_empty() {
    let mut state = state_with_drafted(Card::CeramicsRed);
    assert!(state.players[0].workshop_cards.is_empty());

    let trash = Choice::TrashAndWorkshop {
        card: Card::CeramicsRed,
        workshop_cards: CopyVec::new(),
    };
    assert!(enumerate_choices(&state, 0).contains(&trash));
    let mut rng = WyRand::seed_from_u64(3);
    apply_choice_to_state(&mut state, 0, &trash, &mut rng);

    assert_eq!(
        pending_ability(&state),
        Some(Ability::MixColors { count: 1 }),
        "the workshop fizzling must not take the mix with it"
    );
}

#[test]
fn test_a_wild_primary_stands_in_for_a_missing_primary_in_a_mix() {
    let mut state = state_with_drafted(Card::Realgar);
    let player = &mut state.players[0];
    player.color_wheel = ColorWheel::new();
    player.color_wheel.set(Color::Orange, 1);

    let mix = Choice::TrashAndMix {
        card: Card::Realgar,
        mixes: CopyVec::from_slice(&[(Color::Red, Color::Orange)]),
    };
    assert!(!enumerate_choices(&state, 0).contains(&mix), "no Red and no wild primary");
    assert!(!check_choice_available(&state, 0, &mix));

    state.players[0].wild_primaries = 1;
    assert!(enumerate_choices(&state, 0).contains(&mix));
    assert!(check_choice_available(&state, 0, &mix));

    let mut rng = WyRand::seed_from_u64(3);
    apply_choice_to_state(&mut state, 0, &mix, &mut rng);
    assert_eq!(state.players[0].wild_primaries, 0);
    assert_eq!(state.players[0].color_wheel.get(Color::Vermilion), 1);
    assert_eq!(state.players[0].color_wheel.get(Color::Orange), 0);
}

/// Give `player` exactly what `sell_card` asks for.
fn stock_for(player: &mut PlayerState, sell_card: SellCard) {
    for &material in sell_card.required_materials() {
        player.materials.increment(material);
    }
    player.boats += sell_card.required_boats();
    for &color in sell_card.color_cost() {
        player.color_wheel.increment(color);
    }
}

/// Give player 0 a buyer for `sell_card` and a pending Sell.
fn state_selling(sell_card: SellCard) -> GameState {
    let (mut state, _, _, _) = setup_action_state_with(false, None, None, Some(Ability::Sell));
    state.players[0].color_wheel = ColorWheel::new();
    state.players[0].buyers.push(colori_core::types::SellCardInstance {
        instance_id: 200,
        sell_card,
    });
    state
}

#[test]
fn test_a_boat_and_a_wild_primary_stand_in_for_what_a_sale_is_missing() {
    let sell_card = SellCard::DyerRedYellow;
    let mut state = state_selling(sell_card);
    state.players[0].color_wheel.set(Color::Yellow, 1);
    let select = Choice::SelectSellCard { sell_card };

    assert!(enumerate_choices(&state, 0).is_empty(), "no Textiles and no Red");
    state.players[0].boats = 1;
    assert!(!check_choice_available(&state, 0, &select), "still no Red");
    state.players[0].wild_primaries = 1;
    assert!(enumerate_choices(&state, 0).contains(&select));

    let mut rng = WyRand::seed_from_u64(3);
    apply_choice_to_state(&mut state, 0, &select, &mut rng);
    let player = &state.players[0];
    assert_eq!((player.boats, player.wild_primaries), (0, 0));
    assert_eq!(player.color_wheel.get(Color::Yellow), 0);
    assert_eq!(player.completed_sell_cards.len(), 1);
    assert_eq!(player.cached_whole_points, calculate_whole_points(player));
    assert_eq!(score_in_half_points(player), 2 * sell_card.points());
}

#[test]
fn test_a_sale_spends_what_it_asks_for_before_any_boat_or_wild_primary() {
    let sell_card = SellCard::DyerRedYellow;
    let mut state = state_selling(sell_card);
    let player = &mut state.players[0];
    stock_for(player, sell_card);
    player.boats = 1;
    player.wild_primaries = 1;

    let mut rng = WyRand::seed_from_u64(3);
    apply_choice_to_state(&mut state, 0, &Choice::SelectSellCard { sell_card }, &mut rng);
    let player = &state.players[0];
    assert_eq!((player.boats, player.wild_primaries), (1, 1));
    assert_eq!(player.materials.get(MaterialType::Textiles), 0);
    assert_eq!(player.color_wheel.get(Color::Red), 0);
}

#[test]
fn test_a_shipwright_takes_a_boat_as_well_as_its_textiles() {
    let sell_card = SellCard::ShipwrightPurpleGreen;
    let select = Choice::SelectSellCard { sell_card };
    let mut state = state_selling(sell_card);
    let player = &mut state.players[0];
    player.color_wheel.set(Color::Purple, 1);
    player.color_wheel.set(Color::Green, 1);
    player.materials.increment(MaterialType::Textiles);
    assert!(!check_choice_available(&state, 0, &select), "no boat");

    state.players[0].boats = 1;
    assert!(check_choice_available(&state, 0, &select));
    let mut rng = WyRand::seed_from_u64(3);
    apply_choice_to_state(&mut state, 0, &select, &mut rng);
    let player = &state.players[0];
    assert_eq!(player.boats, 0);
    assert_eq!(player.materials.get(MaterialType::Textiles), 0);
    assert_eq!(player.cached_whole_points, 4);
}

#[test]
fn test_a_shipwright_without_textiles_takes_a_second_boat() {
    let sell_card = SellCard::ShipwrightOrangeGreen;
    let select = Choice::SelectSellCard { sell_card };
    let mut state = state_selling(sell_card);
    let player = &mut state.players[0];
    player.color_wheel.set(Color::Orange, 1);
    player.color_wheel.set(Color::Green, 1);
    player.boats = 1;
    assert!(!check_choice_available(&state, 0, &select), "one boat cannot be both");

    state.players[0].boats = 2;
    let mut rng = WyRand::seed_from_u64(3);
    apply_choice_to_state(&mut state, 0, &select, &mut rng);
    assert_eq!(state.players[0].boats, 0);
}

#[test]
fn test_a_tilemaker_takes_two_ceramics() {
    let sell_card = SellCard::TilemakerTeal;
    let select = Choice::SelectSellCard { sell_card };
    let mut state = state_selling(sell_card);
    let player = &mut state.players[0];
    player.color_wheel.set(Color::Teal, 1);
    player.materials.increment(MaterialType::Ceramics);
    assert!(!check_choice_available(&state, 0, &select), "one Ceramics short");

    // A boat covers the second.
    state.players[0].boats = 1;
    let mut rng = WyRand::seed_from_u64(3);
    apply_choice_to_state(&mut state, 0, &select, &mut rng);
    let player = &state.players[0];
    assert_eq!(player.materials.get(MaterialType::Ceramics), 0);
    assert_eq!(player.boats, 0);
    assert_eq!(player.cached_whole_points, 4);
}

#[test]
fn test_glassmakers_and_illuminators_take_no_material() {
    for sell_card in [SellCard::GlassmakerAmberBlue, SellCard::IlluminatorFuchsia] {
        let mut state = state_selling(sell_card);
        for &color in sell_card.color_cost() {
            state.players[0].color_wheel.increment(color);
        }
        let mut rng = WyRand::seed_from_u64(3);
        apply_choice_to_state(&mut state, 0, &Choice::SelectSellCard { sell_card }, &mut rng);
        let player = &state.players[0];
        assert_eq!(player.completed_sell_cards.len(), 1, "{:?}", sell_card);
        assert_eq!(player.color_wheel.counts.iter().map(|&c| c as u32).sum::<u32>(), 0);
        assert_eq!(player.cached_whole_points, sell_card.points());
    }
}

#[test]
fn test_sell_deck_composition() {
    let mut rng = WyRand::seed_from_u64(1);
    let state = create_initial_game_state(2, &[true, true], &mut rng);
    let in_play = |sell_card: SellCard| {
        let in_deck = state
            .sell_card_deck
            .iter()
            .filter(|&id| state.sell_card_lookup[id as usize] == sell_card)
            .count();
        let on_display = state.sell_card_display.iter().filter(|c| c.sell_card == sell_card).count();
        in_deck + on_display
    };
    for (sell_card, copies) in sell_deck_cards() {
        assert_eq!(in_play(sell_card), copies, "copies of {:?}", sell_card);
    }

    let cards_of = |buyer: Buyer| ALL_SELL_CARDS.iter().filter(|c| c.buyer() == buyer).count();
    let expected = [
        (Buyer::Shipwright, 3, 1, 4),
        (Buyer::Glassmaker, 6, 1, 3),
        (Buyer::Dyer, 9, 2, 2),
        (Buyer::Illuminator, 6, 1, 2),
        (Buyer::Potter, 6, 2, 3),
        (Buyer::Tilemaker, 6, 1, 4),
        (Buyer::Painter, 18, 1, 4),
        (Buyer::MasterPainter, 6, 1, 5),
    ];
    for (buyer, cards, copies, points) in expected {
        assert_eq!(cards_of(buyer), cards, "{:?} cards", buyer);
        assert_eq!(buyer.points(), points, "{:?} points", buyer);
        for sell_card in ALL_SELL_CARDS.iter().filter(|c| c.buyer() == buyer) {
            assert_eq!(sell_card_copies(*sell_card), copies, "{:?} copies", sell_card);
        }
    }
    assert_eq!(sell_deck_cards().map(|(_, copies)| copies).sum::<usize>(), 75);
}

#[test]
fn test_starter_deck_composition() {
    let mut rng = WyRand::seed_from_u64(1);
    let state = create_initial_game_state(4, &[true; 4], &mut rng);
    for player in state.players.iter() {
        let mut cards: Vec<Card> = player.deck.cards().iter().map(|id| state.card_lookup[id as usize]).collect();
        let mut expected = starter_deck().to_vec();
        cards.sort_by_key(|&c| c as usize);
        expected.sort_by_key(|&c| c as usize);
        assert_eq!(cards, expected);
        assert_eq!((player.wild_primaries, player.boats, player.ducats), (0, 1, 1));
        assert_eq!(player.cached_whole_points, calculate_whole_points(player));
    }
}

/// The frontend picks a starter card's character portrait from its id.
#[test]
fn test_starter_cards_take_the_first_ids_a_deck_per_seat() {
    let mut rng = WyRand::seed_from_u64(1);
    let state = create_initial_game_state(4, &[true; 4], &mut rng);
    let deck_size = starter_deck().len() as u8;
    for (seat, player) in state.players.iter().enumerate() {
        let first = seat as u8 * deck_size;
        let ids: Vec<u8> = player.deck.cards().iter().collect();
        assert_eq!(ids, (first..first + deck_size).collect::<Vec<u8>>());
    }
}

#[test]
fn test_draft_deck_composition() {
    let mut rng = WyRand::seed_from_u64(1);
    let state = create_initial_game_state(2, &[true, true], &mut rng);
    assert_eq!(state.draft_deck.len(), 100, "draft deck should hold 100 cards");

    for (card, copies) in draft_deck_cards() {
        let in_deck = state
            .draft_deck
            .iter()
            .filter(|&id| state.card_lookup[id as usize] == card)
            .count();
        assert_eq!(in_deck, copies, "copies of {:?}", card);
        assert_eq!(draft_copies(card), copies);
    }
    assert_eq!(draft_copies(Card::Warehouse), 5);
    assert_eq!(draft_copies(Card::CeramicsRed), 2);
    assert_eq!(draft_copies(Card::CanvasCeramics), 1);
    assert_eq!(draft_copies(Card::Realgar), 3);
    for card in starter_deck() {
        assert_eq!(draft_copies(card), 0, "{:?} is a starter card", card);
    }
}

// ── End-of-round deck handling ──

/// Play a two-player game to the end of player 0's first turn, choosing
/// `EndTurn` as soon as it is legal so the sweep is reached with cards still
/// in both areas.
fn state_at_first_end_of_turn(seed: u64) -> (GameState, UnorderedCards, UnorderedCards) {
    let mut rng = WyRand::seed_from_u64(seed);
    let mut state = create_initial_game_state(2, &[true, true], &mut rng);
    execute_draw_phase(&mut state, &mut rng);

    // Claim buyers, then finish the draft, always taking the first option.
    while matches!(state.phase, GamePhase::Buyers { .. } | GamePhase::Draft { .. }) {
        let player = actor(&state);
        let choices = enumerate_choices(&state, player);
        apply_choice_to_state(&mut state, player, &choices[0], &mut rng);
    }

    let workshop_area = state.players[0]
        .workshop_cards
        .union(state.players[0].workshopped_cards);
    let drafted = state.players[0].drafted_cards;
    assert!(!workshop_area.is_empty() && !drafted.is_empty());

    apply_choice_to_state(&mut state, 0, &Choice::EndTurn, &mut rng);
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
        let player = actor(&state);
        let choices = enumerate_choices(&state, player);
        apply_choice(&mut state, player, &choices[0], &mut rng);
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

/// A game can create at most one segment per round on top of the starting
/// deck, which is what `MAX_DECK_SEGMENTS` allows for.
#[test]
fn test_decks_stay_well_inside_the_segment_cap() {
    for seed in 0..30 {
        for num_players in 2..=4 {
            let state = run_random_game_with_invariants(seed, num_players);
            for (pi, player) in state.players.iter().enumerate() {
                assert!(
                    player.deck.segment_count() <= MAX_DECK_SEGMENTS,
                    "player {pi} ended with {} deck segments (seed={seed}, players={num_players})",
                    player.deck.segment_count(),
                );
            }
        }
    }
}

// ── Draft ──

/// Start round 1 and claim buyers, leaving the game at the start of the draft.
fn state_at_first_draft(seed: u64, num_players: usize) -> (GameState, WyRand) {
    let mut rng = WyRand::seed_from_u64(seed);
    let mut state = create_initial_game_state(num_players, &vec![true; num_players], &mut rng);
    execute_draw_phase(&mut state, &mut rng);
    while matches!(state.phase, GamePhase::Buyers { .. }) {
        let player = actor(&state);
        let choices = enumerate_choices(&state, player);
        apply_choice_to_state(&mut state, player, &choices[0], &mut rng);
    }
    (state, rng)
}

/// Every hand passes until it is down to one card, which goes to whoever it was
/// passed to: one pick fewer than the hand size, a full hand drafted each, and
/// nothing left over to trash.
#[test]
fn test_each_player_drafts_a_full_hand_and_nothing_is_trashed() {
    for num_players in 2..=4 {
        for seed in 0..5 {
            let (mut state, mut rng) = state_at_first_draft(seed, num_players);
            let trashed_before = state.trash;

            let mut picks = 0;
            while matches!(state.phase, GamePhase::Draft { .. }) {
                let player = actor(&state);
                let choices = enumerate_choices(&state, player);
                apply_choice_to_state(&mut state, player, &choices[rng.random_range(0..choices.len())], &mut rng);
                picks += 1;
            }

            let context = format!("players={num_players}, seed={seed}");
            assert!(matches!(state.phase, GamePhase::Action { .. }), "{context}");
            assert_eq!(picks, num_players * (DRAFT_HAND_SIZE as usize - 1), "{context}");
            for player in state.players.iter() {
                assert_eq!(player.drafted_cards.len(), DRAFT_HAND_SIZE, "{context}");
            }
            assert_eq!(state.trash, trashed_before, "{context}");
        }
    }
}

/// Solo: the phantom hand sheds a card per pick and its last card has nobody
/// to go to, so the whole phantom hand is trashed.
#[test]
fn test_solo_draft_trashes_the_phantom_hand() {
    let (mut state, mut rng) = state_at_first_draft(3, 1);
    let trashed_before = state.trash.len();

    while matches!(state.phase, GamePhase::Draft { .. }) {
        let player = actor(&state);
        let choices = enumerate_choices(&state, player);
        apply_choice_to_state(&mut state, player, &choices[0], &mut rng);
    }

    assert_eq!(state.players[0].drafted_cards.len(), DRAFT_HAND_SIZE);
    assert_eq!(state.trash.len(), trashed_before + DRAFT_HAND_SIZE);
}

// ── Buyers phase ──

fn buyers_of(state: &GameState, player: usize) -> Vec<SellCard> {
    state.players[player].buyers.iter().map(|b| b.sell_card).collect()
}

/// Advance to the buyers phase of `round`, resolving every earlier phase by
/// always taking the first legal option.
fn state_at_buyers_phase(seed: u64, num_players: usize, round: u32) -> (GameState, WyRand) {
    let mut rng = WyRand::seed_from_u64(seed);
    let ai_players = vec![true; num_players];
    let mut state = create_initial_game_state(num_players, &ai_players, &mut rng);
    execute_draw_phase(&mut state, &mut rng);

    while state.round < round {
        let player = actor(&state);
        let choices = enumerate_choices(&state, player);
        apply_choice_to_state(&mut state, player, &choices[0], &mut rng);
    }
    assert!(
        matches!(state.phase, GamePhase::Buyers { .. }),
        "round {round} should open in the buyers phase"
    );
    (state, rng)
}

#[test]
fn test_the_round_opens_with_the_buyers_phase() {
    let mut rng = WyRand::seed_from_u64(3);
    let mut state = create_initial_game_state(3, &[true, true, true], &mut rng);
    assert!(matches!(state.phase, GamePhase::Draw));

    execute_draw_phase(&mut state, &mut rng);
    match &state.phase {
        GamePhase::Buyers { buyers_state } => {
            assert_eq!(buyers_state.current_player_index, 0, "round 1 starts with player 0");
        }
        other => panic!("expected the buyers phase, got {other:?}"),
    }
}

#[test]
fn test_players_claim_up_to_capacity_for_the_round() {
    for (round, capacity) in [(1u32, 1usize), (2, 2), (3, 3)] {
        let (mut state, mut rng) = state_at_buyers_phase(21, 3, round);
        while matches!(state.phase, GamePhase::Buyers { .. }) {
            let player = actor(&state);
            let choices = enumerate_choices(&state, player);
            apply_choice_to_state(&mut state, player, &choices[0], &mut rng);
        }
        for (i, player) in state.players.iter().enumerate() {
            assert_eq!(
                player.buyers.len(),
                capacity,
                "player {i} should hold {capacity} buyers in round {round}"
            );
        }
    }
}

/// Everyone takes one before anyone takes two.
#[test]
fn test_claiming_goes_round_robin() {
    let (mut state, mut rng) = state_at_buyers_phase(21, 3, 2);
    // Round 1's buyers survive into round 2 unless they were sold, and who
    // still needs a card would confound the ordering being tested.
    for player in state.players.iter_mut() {
        while !player.buyers.is_empty() {
            player.buyers.remove(0);
        }
    }
    initialize_buyers_phase(&mut state, &mut rng);

    let mut order = Vec::new();
    while let GamePhase::Buyers { ref buyers_state } = state.phase {
        order.push(buyers_state.current_player_index);
        let player = actor(&state);
        let choices = enumerate_choices(&state, player);
        apply_choice_to_state(&mut state, player, &choices[0], &mut rng);
    }

    // Round 2 starts with player 1, two slots each.
    assert_eq!(order, vec![1, 2, 0, 1, 2, 0]);
}

/// A claimed card is replaced at once, so every player picks from a full
/// display.
#[test]
fn test_the_display_refills_after_every_claim() {
    let (mut state, mut rng) = state_at_buyers_phase(5, 4, 3);
    while matches!(state.phase, GamePhase::Buyers { .. }) {
        assert_eq!(
            state.sell_card_display.len(),
            MAX_SELL_CARD_DISPLAY,
            "the display should be full whenever someone is choosing"
        );
        let player = actor(&state);
        let choices = enumerate_choices(&state, player);
        apply_choice_to_state(&mut state, player, &choices[0], &mut rng);
    }
}

#[test]
fn test_claiming_off_the_deck_takes_a_card_nobody_could_see() {
    let (mut state, mut rng) = state_at_buyers_phase(9, 2, 1);
    let display_before: Vec<SellCard> =
        state.sell_card_display.iter().map(|c| c.sell_card).collect();
    let deck_before = state.sell_card_deck.len();
    let player = match &state.phase {
        GamePhase::Buyers { buyers_state } => buyers_state.current_player_index,
        _ => unreachable!(),
    };

    assert!(enumerate_choices(&state, player).contains(&Choice::DrawBuyer));
    apply_choice_to_state(&mut state, player, &Choice::DrawBuyer, &mut rng);

    assert_eq!(state.players[player].buyers.len(), 1);
    assert_eq!(state.sell_card_deck.len(), deck_before - 1);
    assert_eq!(
        state.sell_card_display.iter().map(|c| c.sell_card).collect::<Vec<_>>(),
        display_before,
        "claiming off the deck should leave the display untouched"
    );
}

/// Selling consumes one of your own buyers. The shared display is not a
/// source any more, so nothing there moves.
#[test]
fn test_selling_takes_from_your_buyers_and_leaves_the_display_alone() {
    let mut rng = WyRand::seed_from_u64(4);
    let mut state = create_initial_game_state(2, &[true, true], &mut rng);

    // Give player 0 a buyer they can certainly afford, and the means to pay.
    let sell_card = state.sell_card_display[0].sell_card;
    let instance = state.sell_card_display.remove(0);
    state.players[0].buyers.push(instance);
    stock_for(&mut state.players[0], sell_card);

    let display_before: Vec<SellCard> =
        state.sell_card_display.iter().map(|c| c.sell_card).collect();
    let deck_before = state.sell_card_deck.len();

    let mut ability_stack = AbilityStack::new();
    ability_stack.push(Ability::Sell);
    state.phase = open_action_phase(&state, ability_stack);

    apply_choice_to_state(&mut state, 0, &Choice::SelectSellCard { sell_card }, &mut rng);

    assert!(state.players[0].buyers.is_empty(), "the slot should be empty now");
    assert_eq!(state.players[0].completed_sell_cards.len(), 1);
    assert_eq!(
        state.sell_card_display.iter().map(|c| c.sell_card).collect::<Vec<_>>(),
        display_before,
        "a sale must not disturb the shared display"
    );
    assert_eq!(state.sell_card_deck.len(), deck_before, "and must not draw");
}

/// A sold buyer leaves a hole, and the next buyers phase is what fills it.
#[test]
fn test_an_emptied_slot_is_refilled_next_round() {
    // Round 2 always opens with a buyers phase, since capacity has just grown.
    let (mut state, mut rng) = state_at_buyers_phase(21, 2, 2);
    while matches!(state.phase, GamePhase::Buyers { .. }) {
        let player = actor(&state);
        let choices = enumerate_choices(&state, player);
        apply_choice_to_state(&mut state, player, &choices[0], &mut rng);
    }
    assert_eq!(state.players[0].buyers.len(), 2);

    let kept = buyers_of(&state, 0)[1];
    state.players[0].buyers.remove(0);

    // Skip ahead to the next buyers phase.
    while !matches!(state.phase, GamePhase::Buyers { .. }) {
        let player = actor(&state);
        let choices = enumerate_choices(&state, player);
        apply_choice_to_state(&mut state, player, &choices[0], &mut rng);
    }
    assert_eq!(state.round, 3);
    while matches!(state.phase, GamePhase::Buyers { .. }) {
        let player = actor(&state);
        let choices = enumerate_choices(&state, player);
        apply_choice_to_state(&mut state, player, &choices[0], &mut rng);
    }

    assert_eq!(state.players[0].buyers.len(), 3, "round 3 capacity is three");
    assert!(
        buyers_of(&state, 0).contains(&kept),
        "the buyer that was not sold should still be there"
    );
}

/// Both piles can run dry in a long game. The phase is then skipped rather
/// than seating a player with nothing to take.
#[test]
fn test_an_exhausted_sell_card_supply_skips_the_buyers_phase() {
    let mut rng = WyRand::seed_from_u64(12);
    let mut state = create_initial_game_state(2, &[true, true], &mut rng);
    state.sell_card_deck = UnorderedSellCards::new();
    while !state.sell_card_display.is_empty() {
        state.sell_card_display.remove(0);
    }

    execute_draw_phase(&mut state, &mut rng);
    assert!(
        matches!(state.phase, GamePhase::Draft { .. }),
        "with nothing to claim the buyers phase should hand straight over to the draft"
    );
    for player in state.players.iter() {
        assert!(player.buyers.is_empty());
    }

    // And the game still finishes, with nobody able to sell.
    let mut guard = 0;
    while !matches!(state.phase, GamePhase::GameOver) {
        let player = actor(&state);
        let choices = enumerate_choices(&state, player);
        apply_choice_to_state(&mut state, player, &choices[0], &mut rng);
        guard += 1;
        assert!(guard < 5_000, "game did not finish with an exhausted sell card supply");
    }
    for player in state.players.iter() {
        assert!(player.completed_sell_cards.is_empty());
    }
}

// ── Draw log recording and replay ──

fn take_recording(state: &mut GameState) -> Vec<DrawEvent> {
    match state.draw_log.take() {
        Some(DrawLog::Recording(events)) => events,
        _ => Vec::new(),
    }
}

/// Everything hidden that a game reveals must be recoverable from the draw
/// log, or a replayed log diverges from the game it recorded.
///
/// The replay runs on a *different* RNG seed on purpose: any draw the log fails
/// to cover falls through to that RNG and the states come apart.
#[test]
fn test_a_recorded_game_replays_to_the_same_state() {
    // Four players outdraw the draft deck, so the replay has to follow the
    // trash being shuffled back in.
    for num_players in 1..=4 {
        for seed in 0..4 {
            let mut rng = WyRand::seed_from_u64(seed * 31 + num_players as u64);
            let ai_players = vec![true; num_players];
            let mut state = create_initial_game_state(num_players, &ai_players, &mut rng);
            let initial = state.clone();

            state.draw_log = Some(DrawLog::Recording(Vec::new()));
            execute_draw_phase(&mut state, &mut rng);
            let initial_draws = take_recording(&mut state);

            let mut script: Vec<(usize, Choice, Vec<DrawEvent>)> = Vec::new();
            while !matches!(state.phase, GamePhase::GameOver) {
                let player = random_actor(&state, &mut rng);
                let choices = enumerate_choices(&state, player);
                assert!(!choices.is_empty(), "no legal choice (players={num_players}, seed={seed})");
                let choice = choices[rng.random_range(0..choices.len())].clone();
                state.draw_log = Some(DrawLog::Recording(Vec::new()));
                apply_choice_to_state(&mut state, player, &choice, &mut rng);
                script.push((player, choice, take_recording(&mut state)));
            }
            let recorded = state;

            let mut replay_rng = WyRand::seed_from_u64(0xDEAD_BEEF);
            let mut replayed = initial;
            replayed.draw_log = Some(DrawLog::Replaying(initial_draws.into_iter().collect()));
            execute_draw_phase(&mut replayed, &mut replay_rng);
            replayed.draw_log = None;

            for (player, choice, draws) in script {
                replayed.draw_log = Some(DrawLog::Replaying(draws.into_iter().collect()));
                apply_choice_to_state(&mut replayed, player, &choice, &mut replay_rng);
                replayed.draw_log = None;
            }

            assert_states_match(
                &recorded,
                &replayed,
                &format!("replay diverged (players={num_players}, seed={seed})"),
            );
        }
    }
}

// ── Ducat income and spending ──

#[test]
fn test_income_arrives_in_rounds_four_to_seven() {
    for round in 1..=7u32 {
        let mut rng = WyRand::seed_from_u64(round as u64);
        let mut state = create_initial_game_state(2, &[true, true], &mut rng);
        state.round = round;
        let before: Vec<(u32, u32, u32)> = state
            .players
            .iter()
            .map(|p| (p.ducats, p.boats, p.wild_primaries))
            .collect();

        execute_draw_phase(&mut state, &mut rng);

        let expected = match round {
            4 | 6 => RoundIncome { ducats: 1, boats: 0, wild_primaries: 0 },
            5 => RoundIncome { ducats: 0, boats: 1, wild_primaries: 0 },
            7 => RoundIncome { ducats: 1, boats: 1, wild_primaries: 1 },
            _ => RoundIncome::default(),
        };
        assert_eq!(round_income(round), expected);
        for (i, player) in state.players.iter().enumerate() {
            let (ducats, boats, wild_primaries) = before[i];
            assert_eq!(
                (player.ducats, player.boats, player.wild_primaries),
                (
                    ducats + expected.ducats,
                    boats + expected.boats,
                    wild_primaries + expected.wild_primaries
                ),
                "player {i} ducats, boats and wild primaries after the round {round} draw phase"
            );
            assert_eq!(
                player.cached_whole_points,
                calculate_whole_points(player),
                "income must move the cached score with it (round {round})"
            );
        }
    }
}

/// A ducat is a point, so paying one has to come off the score as well.
#[test]
fn test_spending_a_ducat_costs_a_point_and_stacks_the_purchase() {
    let (mut state, _, _, _) =
        setup_action_state_with(false, None, Some(Card::StarterCeramics), None);
    state.players[0].ducats = 2;
    state.players[0].cached_whole_points = calculate_whole_points(&state.players[0]);

    let choice = Choice::SpendDucat { purchase: DucatPurchase::Workshop };
    assert!(enumerate_choices(&state, 0).contains(&choice));
    assert!(check_choice_available(&state, 0, &choice));

    let mut rng = WyRand::seed_from_u64(1);
    apply_choice_to_state(&mut state, 0, &choice, &mut rng);

    assert_eq!(state.players[0].ducats, 1);
    assert_eq!(state.players[0].cached_whole_points, calculate_whole_points(&state.players[0]));
    match &state.phase {
        GamePhase::Action { action_state } => assert_eq!(
            action_state.turns[0].ability_stack.last(),
            Some(&Ability::Workshop { count: 1 }),
            "the purchase should be waiting for input"
        ),
        other => panic!("expected the action phase, got {other:?}"),
    }
}

#[test]
fn test_a_purchase_is_only_offered_when_it_would_do_something() {
    let spend = |p| Choice::SpendDucat { purchase: p };

    // No ducats, nothing on offer.
    let (state, _, _, _) = setup_action_state_with(false, None, Some(Card::StarterCeramics), None);
    assert!(!enumerate_choices(&state, 0).iter().any(|c| matches!(c, Choice::SpendDucat { .. })));

    // A ducat and a workshop card, but a bare colour wheel and no buyers.
    let (mut state, _, _, _) =
        setup_action_state_with(false, None, Some(Card::StarterCeramics), None);
    state.players[0].ducats = 1;
    state.players[0].color_wheel = ColorWheel::new();
    state.players[0].cached_whole_points = calculate_whole_points(&state.players[0]);

    let choices = enumerate_choices(&state, 0);
    assert!(choices.contains(&spend(DucatPurchase::Workshop)));
    assert!(!choices.contains(&spend(DucatPurchase::MixColors)), "no two colours to mix");
    assert!(!choices.contains(&spend(DucatPurchase::Sell)), "no buyer to sell to");

    // A mixable pair brings the mix back.
    state.players[0].color_wheel.increment(Color::Red);
    state.players[0].color_wheel.increment(Color::Yellow);
    assert!(enumerate_choices(&state, 0).contains(&spend(DucatPurchase::MixColors)));

    // Emptying the workshop takes the workshop pick away.
    state.players[0].workshop_cards = UnorderedCards::new();
    assert!(!enumerate_choices(&state, 0).contains(&spend(DucatPurchase::Workshop)));
}

/// Buying is a turn action, not something to interleave with an ability that
/// is already waiting for input.
#[test]
fn test_a_ducat_cannot_be_spent_while_an_ability_is_pending() {
    let (mut state, _, _, _) = setup_action_state_with(
        false,
        None,
        Some(Card::StarterCeramics),
        Some(Ability::Workshop { count: 2 }),
    );
    state.players[0].ducats = 3;
    state.players[0].cached_whole_points = calculate_whole_points(&state.players[0]);

    let choices = enumerate_choices(&state, 0);
    assert!(!choices.iter().any(|c| matches!(c, Choice::SpendDucat { .. })));
    for purchase in colori_core::types::ALL_DUCAT_PURCHASES {
        assert!(!check_choice_available(&state, 0, &Choice::SpendDucat { purchase }));
    }
}

/// The whole point of the Sell purchase: one ducat in, a sell card worth two
/// to five points out.
#[test]
fn test_a_ducat_buys_a_sale_that_pays_for_itself() {
    let mut rng = WyRand::seed_from_u64(4);
    let mut state = create_initial_game_state(2, &[true, true], &mut rng);

    let instance = state.sell_card_display.remove(0);
    let sell_card = instance.sell_card;
    state.players[0].buyers.push(instance);
    stock_for(&mut state.players[0], sell_card);
    state.players[0].ducats = 1;
    state.players[0].cached_whole_points = calculate_whole_points(&state.players[0]);
    let score_before = state.players[0].cached_whole_points;

    state.phase = open_action_phase(&state, AbilityStack::new());

    let spend = Choice::SpendDucat { purchase: DucatPurchase::Sell };
    assert!(enumerate_choices(&state, 0).contains(&spend));
    apply_choice_to_state(&mut state, 0, &spend, &mut rng);
    apply_choice_to_state(&mut state, 0, &Choice::SelectSellCard { sell_card }, &mut rng);

    assert_eq!(state.players[0].ducats, 0, "the ducat was spent");
    assert_eq!(state.players[0].completed_sell_cards.len(), 1);
    assert_eq!(
        state.players[0].cached_whole_points,
        score_before - 1 + sell_card.points(),
        "net gain is the sell card's value less the ducat"
    );
    assert_eq!(state.players[0].cached_whole_points, calculate_whole_points(&state.players[0]));
}

// ── Simultaneous action phase ──

/// The first action phase of a game, reached by random play.
fn state_at_first_action_phase(seed: u64, num_players: usize) -> (GameState, WyRand) {
    let mut rng = WyRand::seed_from_u64(seed);
    let mut state = create_initial_game_state(num_players, &vec![true; num_players], &mut rng);
    execute_draw_phase(&mut state, &mut rng);
    while !matches!(state.phase, GamePhase::Action { .. }) {
        let player = actor(&state);
        let choices = enumerate_choices(&state, player);
        let choice = choices[rng.random_range(0..choices.len())].clone();
        apply_choice_to_state(&mut state, player, &choice, &mut rng);
    }
    (state, rng)
}

/// Each player's whole turn, chosen at random on their own view of the phase.
fn random_turns(state: &GameState, rng: &mut WyRand) -> Vec<Vec<Choice>> {
    let mut views = ActionPhaseViews::new(state);
    (0..state.players.len())
        .map(|player| {
            let mut turn = Vec::new();
            loop {
                let choices = enumerate_choices(views.view(player), player);
                let choice = choices[rng.random_range(0..choices.len())].clone();
                views.apply(player, &choice);
                let ended = matches!(choice, Choice::EndTurn);
                turn.push(choice);
                if ended {
                    return turn;
                }
            }
        })
        .collect()
}

/// Nobody's turn touches anyone else's, so the order turns are interleaved in
/// cannot change where the round ends up.
#[test]
fn test_interleaving_turns_does_not_change_the_outcome() {
    for num_players in 2..=4 {
        for seed in 0..10 {
            let (start, mut rng) = state_at_first_action_phase(seed, num_players);
            let turns = random_turns(&start, &mut rng);

            let mut one_after_another = start.clone();
            for (player, turn) in turns.iter().enumerate() {
                for choice in turn {
                    apply_choice(&mut one_after_another, player, choice, &mut rng);
                }
            }

            let mut interleaved = start.clone();
            let mut next = vec![0usize; num_players];
            while next.iter().zip(&turns).any(|(&i, turn)| i < turn.len()) {
                let open: Vec<usize> = (0..num_players).filter(|&p| next[p] < turns[p].len()).collect();
                let player = open[rng.random_range(0..open.len())];
                apply_choice(&mut interleaved, player, &turns[player][next[player]], &mut rng);
                next[player] += 1;
            }

            let context = format!("players={num_players}, seed={seed}");
            assert!(matches!(one_after_another.phase, GamePhase::Draw), "{context}");
            assert_states_match(&one_after_another, &interleaved, &context);
        }
    }
}

#[test]
fn test_the_round_ends_with_the_last_turn_whoever_takes_it() {
    let (mut state, mut rng) = state_at_first_action_phase(6, 3);
    for player in [2, 0] {
        while !matches!(&state.phase, GamePhase::Action { action_state } if action_state.turns[player].ended) {
            let choices = enumerate_choices(&state, player);
            let choice = choices.last().unwrap().clone();
            apply_choice_to_state(&mut state, player, &choice, &mut rng);
        }
        assert!(matches!(state.phase, GamePhase::Action { .. }), "player 1 has not ended");
    }
    assert_eq!(actor(&state), 1);

    let round = state.round;
    while state.round == round {
        let choices = enumerate_choices(&state, 1);
        apply_choice_to_state(&mut state, 1, choices.last().unwrap(), &mut rng);
    }
}

#[test]
fn test_a_player_whose_turn_is_over_has_nothing_left_to_choose() {
    let (mut state, mut rng) = state_at_first_action_phase(8, 2);
    let mut stack = AbilityStack::new();
    stack.push(Ability::Sell);
    // Player 1 is mid-turn with an ability pending while player 0 ends theirs.
    if let GamePhase::Action { ref mut action_state } = state.phase {
        action_state.turns[1].ability_stack = stack;
    }
    apply_choice_to_state(&mut state, 0, &Choice::EndTurn, &mut rng);
    // Enough to buy a workshop pick, were the turn still open.
    state.players[0].ducats = 3;
    state.players[0].cached_whole_points = calculate_whole_points(&state.players[0]);
    assert!(!state.players[0].workshop_cards.is_empty());

    assert!(enumerate_choices(&state, 0).is_empty());
    assert!(!check_choice_available(&state, 0, &Choice::EndTurn));
    assert!(!check_choice_available(
        &state,
        0,
        &Choice::SpendDucat { purchase: DucatPurchase::Workshop }
    ));
    assert!(
        !check_choice_available(&state, 1, &Choice::EndTurn),
        "player 1's own pending ability still stands in the way of ending"
    );
}

/// A view holds the phase as it began plus that player's own moves: their own
/// state matches the live game, and nobody else has moved.
#[test]
fn test_a_view_shows_only_its_own_players_moves() {
    for seed in 0..5 {
        let (start, mut rng) = state_at_first_action_phase(seed, 3);
        let turns = random_turns(&start, &mut rng);
        let mut views = ActionPhaseViews::new(&start);
        let mut live = start.clone();

        for (player, turn) in turns.iter().enumerate().take(2) {
            for choice in turn {
                apply_choice_to_state(&mut live, player, choice, &mut rng);
                views.apply(player, choice);
            }
        }

        for player in 0..2 {
            let view = views.view(player);
            assert_eq!(format!("{:?}", view.players[player]), format!("{:?}", live.players[player]));
            let other = 1 - player;
            assert_eq!(format!("{:?}", view.players[other]), format!("{:?}", start.players[other]));
            match &view.phase {
                GamePhase::Action { action_state } => {
                    assert!(action_state.turns[player].ended);
                    assert!(!action_state.turns[other].ended, "the other turn is unseen");
                }
                other => panic!("expected the action phase, got {other:?}"),
            }
        }
    }
}

/// Game logs and saved games from before "destroy" became "trash" still load.
#[test]
fn test_logs_written_with_destroy_names_still_load() {
    let choices = [
        (r#"{"type":"destroyDraftedCard","card":"Alum"}"#, "trashDraftedCard"),
        (r#"{"type":"destroyAndMix","card":"Woad","mixes":[]}"#, "trashAndMix"),
        (r#"{"type":"destroyAndMixAll","card":"Woad","mixes":[]}"#, "trashAndMix"),
        (r#"{"type":"destroyAndSell","card":"Indigo","sellCard":"DyerGreen"}"#, "trashAndSell"),
        (r#"{"type":"destroyAndWorkshop","card":"Cinnabar","workshopCards":[]}"#, "trashAndWorkshop"),
        (r#"{"type":"destroyAndMoveToDraftPool","card":"Alum","target":null}"#, "trashAndMoveToDraftPool"),
    ];
    for (old, new_type) in choices {
        let choice: Choice = serde_json::from_str(old).unwrap_or_else(|e| panic!("{old}: {e}"));
        let written = serde_json::to_value(&choice).unwrap();
        assert_eq!(written["type"], new_type, "{old}");
    }

    let mut rng = WyRand::seed_from_u64(1);
    let state = create_initial_game_state(2, &[false, false], &mut rng);
    let mut json = serde_json::to_value(&state).unwrap();
    let trash = json.as_object_mut().unwrap().remove("trash").expect("the trash is written as `trash`");
    json["destroyedPile"] = trash;
    let reloaded: GameState = serde_json::from_value(json).unwrap();
    assert_eq!(reloaded.trash.len(), state.trash.len());
}
