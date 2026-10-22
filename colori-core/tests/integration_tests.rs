use colori_core::colori_game::{
    check_choice_available, enumerate_choices, get_game_status,
    ActionPhaseViews, GameStatus,
};
use colori_core::apply_choice::apply_choice;
use colori_core::buyers_phase::initialize_buyers_phase;
use colori_core::deck::{Deck, MAX_DECK_SEGMENTS};
use colori_core::draft_phase::DRAFT_HAND_SIZE;
use colori_core::game_log::{DrawEvent, DrawLog};
use colori_core::round::{round_income, start_round, RoundIncome};
use colori_core::cards::{draft_copies, draft_deck_cards, buyer_copies, buyer_deck_cards, starter_deck};
use colori_core::scoring::{calculate_whole_points, score_in_half_points};
use colori_core::setup::create_initial_game_state;
use colori_core::types::{Ability, AbilityStack, ActionState, Trade, Buyer, Card, Choice, Color, ColorWheel, GoldCoinPurchase, GamePhase, GameState, MaterialType, PlayerState, PlayerTurn, ALL_BUYERS, MAX_BUYER_DISPLAY};
use colori_core::unordered_cards::{UnorderedCards, UnorderedBuyers};
use colori_core::unordered_cards::{
    get_buyer_registry, get_card_registry, set_buyer_registry, set_card_registry,
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
        total += player.used_cards.len();
        total += player.workshop_cards.len();
        total += player.top_cards.len();
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

fn count_all_buyers(state: &GameState) -> u32 {
    let mut total = 0u32;

    total += state.buyer_deck.len();
    total += state.buyer_display.len() as u32;

    for player in state.players.iter() {
        total += player.buyers.len() as u32;
        total += player.completed_orders.len() as u32;
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
        for &count in player.materials_wheel.counts.iter() {
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
    let bot_players = vec![true; num_players];
    let mut state = create_initial_game_state(num_players, &bot_players, &mut rng);

    let initial_cards = count_all_cards(&state);
    let initial_buyers = count_all_buyers(&state);


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
            "Buyer card conservation violated at step {} (seed={}, players={})",
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

        apply_choice(&mut state, player, &choices[0], &mut rng);
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
    let bot_players = vec![true; num_players];
    let mut state = create_initial_game_state(num_players, &bot_players, &mut rng);

    let initial_cards = count_all_cards(&state);
    let initial_buyers = count_all_buyers(&state);


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
            count_all_buyers(&state),
            initial_buyers,
            "Buyer card conservation violated at step {} (seed={}, players={})",
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
        apply_choice(&mut state, player, &choices[idx], &mut rng);
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
            invalid.push(Choice::EndTurn { keep_remaining: true });
            invalid.push(Choice::SkipWorkshop);
            invalid.push(Choice::SellToBuyer {
                buyer: Buyer::IlluminatorVermilion,
            });
            invalid.push(Choice::GainSecondary {
                color: Color::Orange,
            });
            invalid.push(Choice::MixAll {
                mixes: CopyVec::new(),
            });
            invalid.push(Choice::LiftCard { card: None });
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
                invalid.push(Choice::SellToBuyer {
                    buyer: Buyer::IlluminatorVermilion,
                });
                invalid.push(Choice::GainSecondary {
                    color: Color::Orange,
                });
                invalid.push(Choice::MixAll {
                    mixes: CopyVec::new(),
                });
                invalid.push(Choice::LiftCard { card: None });
            } else {
                let top = stack.last().unwrap();
                match top {
                    Ability::Workshop { .. } => {
                        invalid.push(Choice::SellToBuyer {
                            buyer: Buyer::IlluminatorVermilion,
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
                    }
                    Ability::MixColors { .. } => {
                        // Non-adjacent colors are invalid
                        invalid.push(Choice::MixAll {
                            mixes: CopyVec::from_slice(&[(Color::Orange, Color::Green)]),
                        });
                    }
                    Ability::Sell => {
                        invalid.push(Choice::SkipWorkshop);
                        invalid.push(Choice::GainSecondary {
                            color: Color::Orange,
                        });
                    }
                    Ability::LiftCard => {
                        invalid.push(Choice::SkipWorkshop);
                    }
                    Ability::GainGoldCoins { .. }
                    | Ability::GainBowls { .. }
                    | Ability::GainBoats { .. } => {
                        // Auto-resolved, but just in case
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
    set_buyer_registry(&state.buyer_lookup);
    serde_json::to_string(state).unwrap()
}

fn deserialize_state(json: &str) -> GameState {
    let mut state: GameState = serde_json::from_str(json).unwrap();
    state.card_lookup = get_card_registry();
    state.buyer_lookup = get_buyer_registry();
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
            pa.used_cards, pb.used_cards,
            "player {} used_cards mismatch: {}",
            pi, context
        );
        assert_eq!(
            pa.workshop_cards, pb.workshop_cards,
            "player {} workshop_cards mismatch: {}",
            pi, context
        );
        assert_eq!(
            pa.top_cards, pb.top_cards,
            "player {} top_cards mismatch: {}",
            pi, context
        );
        assert_eq!(
            pa.color_wheel.counts, pb.color_wheel.counts,
            "player {} color_wheel mismatch: {}",
            pi, context
        );
        assert_eq!(
            pa.materials_wheel.counts, pb.materials_wheel.counts,
            "player {} materials mismatch: {}",
            pi, context
        );
        assert_eq!(
            pa.gold_coins, pb.gold_coins,
            "player {} gold_coins mismatch: {}",
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
                ba.buyer, bb.buyer,
                "player {} buyer {} card mismatch: {}",
                pi, bi, context
            );
        }
        assert_eq!(
            pa.completed_orders.len(),
            pb.completed_orders.len(),
            "player {} completed_orders length mismatch: {}",
            pi, context
        );
        for (bi, (ba, bb)) in pa
            .completed_orders
            .iter()
            .zip(pb.completed_orders.iter())
            .enumerate()
        {
            assert_eq!(
                ba.instance_id, bb.instance_id,
                "player {} buyer {} instance_id mismatch: {}",
                pi, bi, context
            );
            assert_eq!(
                ba.buyer, bb.buyer,
                "player {} buyer {} card mismatch: {}",
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
        a.buyer_deck, b.buyer_deck,
        "buyer_deck mismatch: {}",
        context
    );
    assert_eq!(
        a.buyer_display.len(),
        b.buyer_display.len(),
        "buyer_display length mismatch: {}",
        context
    );
    for (i, (ba, bb)) in a
        .buyer_display
        .iter()
        .zip(b.buyer_display.iter())
        .enumerate()
    {
        assert_eq!(
            ba.instance_id, bb.instance_id,
            "buyer_display {} instance_id mismatch: {}",
            i, context
        );
        assert_eq!(
            ba.buyer, bb.buyer,
            "buyer_display {} card mismatch: {}",
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
fn test_buyer_conservation() {
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
            let bot_players = vec![true; num_players];
            let mut state = create_initial_game_state(num_players, &bot_players, &mut rng);


            let mut step = 0u32;
            loop {
                match state.phase {
                    GamePhase::GameOver => break,
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
                apply_choice(&mut state, player, &choices[idx], &mut rng);
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
            let bot_players = vec![true; num_players];
            let mut state = create_initial_game_state(num_players, &bot_players, &mut rng);


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
                    _ => {}
                }

                let player = random_actor(&state, &mut rng);
                let choices = enumerate_choices(&state, player);
                assert!(!choices.is_empty());

                let idx = rng.random_range(0..choices.len());
                apply_choice(&mut state, player, &choices[idx], &mut rng);
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
            let bot_players = vec![true; num_players];
            let mut state = create_initial_game_state(num_players, &bot_players, &mut rng);


            let mut step = 0u32;
            loop {
                match state.phase {
                    GamePhase::GameOver => break,
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

                    apply_choice(&mut original, player, choice, &mut rng_a);
                    apply_choice(&mut restored, player, choice, &mut rng_b);

                    assert_states_match(
                        &original,
                        &restored,
                        &format!(
                            "determinism at step {} (seed={}, players={})",
                            step, seed, num_players
                        ),
                    );
                }

                apply_choice(&mut state, player, choice, &mut rng);
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
        "Draft deck should have 100 cards (54 pigment + 21 material + 25 action)"
    );
}

#[test]
fn test_buyer_deck_card_count() {
    for num_players in 2..=4 {
        let mut rng = WyRand::seed_from_u64(0);
        let bot_players = vec![true; num_players];
        let state = create_initial_game_state(num_players, &bot_players, &mut rng);
        let total_buyers = state.buyer_deck.len() + state.buyer_display.len() as u32;
        assert_eq!(
            total_buyers, 75,
            "Total buyers should be 75 (players={})",
            num_players
        );
    }
}

#[test]
fn test_personal_deck_card_count() {
    for num_players in 2..=4 {
        let mut rng = WyRand::seed_from_u64(0);
        let bot_players = vec![true; num_players];
        let state = create_initial_game_state(num_players, &bot_players, &mut rng);
        for (i, player) in state.players.iter().enumerate() {
            assert_eq!(
                player.deck.len() + player.workshop_cards.len(),
                7,
                "Player {} should start with 7 cards (players={})",
                i,
                num_players
            );
            assert_eq!(
                player.workshop_cards.len(),
                5,
                "setup deals five of them into the workshop (players={num_players})"
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
                if player.used_cards.contains(idx as u8) { return idx as u8; }
                if player.top_cards.contains(idx as u8) { return idx as u8; }
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
        if player.used_cards.contains(id) { player.used_cards.remove(id); return; }
        if player.top_cards.contains(id) { player.top_cards.remove(id); return; }
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
    // These fixtures take the board as they find it, so the workshop setup
    // dealt goes back to the deck.
    for p in state.players.iter_mut() {
        let dealt = p.workshop_cards;
        p.workshop_cards = UnorderedCards::new();
        p.deck.push_top(dealt);
    }
    // Count what a card or sale gives from an empty purse.
    let player = &mut state.players[0];
    player.gold_coins = 0;
    player.boats = 0;
    player.cached_whole_points = 0;

    let alum_id = find_card_id(&state, Card::Alum);
    remove_card_anywhere(&mut state, alum_id);
    if alum_in_drafted {
        state.players[0].top_cards.insert(alum_id);
    }

    let ws_id = workshopped_target.map(|c| {
        let id = find_card_id(&state, c);
        remove_card_anywhere(&mut state, id);
        state.players[0].used_cards.insert(id);
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
fn test_lift_card_targets_only_unused_cards() {
    let (state, _, _, _) = setup_action_state_with(
        false,
        Some(Card::StarterCeramics),
        Some(Card::StarterCanvas),
        Some(Ability::LiftCard),
    );

    let canvas = Choice::LiftCard { card: Some(Card::StarterCanvas) };
    let ceramics = Choice::LiftCard { card: Some(Card::StarterCeramics) };
    assert_eq!(enumerate_choices(&state, 0), vec![canvas.clone()]);
    assert!(check_choice_available(&state, 0, &canvas));
    assert!(!check_choice_available(&state, 0, &ceramics), "a workshopped card cannot be moved");
    assert!(
        !check_choice_available(&state, 0, &Choice::LiftCard { card: None }),
        "the move is mandatory while there is a card to move"
    );
}

#[test]
fn test_lift_card_fizzles_when_only_used_cards_remain() {
    let (state, _, _, _) = setup_action_state_with(
        false,
        Some(Card::StarterCeramics),
        None,
        Some(Ability::LiftCard),
    );
    assert_eq!(
        enumerate_choices(&state, 0),
        vec![Choice::LiftCard { card: None }]
    );
}

#[test]
fn test_trashing_alum_targets_only_unused_cards() {
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
        .filter(|c| matches!(c, Choice::TrashAndLift { .. }))
        .collect();
    let canvas = Choice::TrashAndLift {
        card: Card::Alum,
        target: Some(Card::StarterCanvas),
    };
    assert_eq!(moves, vec![&canvas]);
    assert!(check_choice_available(&state, 0, &canvas));
    assert!(!check_choice_available(
        &state,
        0,
        &Choice::TrashAndLift { card: Card::Alum, target: Some(Card::StarterCeramics) }
    ));
    assert!(!check_choice_available(
        &state,
        0,
        &Choice::TrashAndLift { card: Card::Alum, target: None }
    ));
}

/// The move does not trash the card: it lands in the top cards, where the
/// ordinary trash path can pick it up later — or leave it there for end of
/// turn to return to the workshop.
#[test]
fn test_apply_lift_card_moves_the_card_without_trashing_it() {
    let (mut state, _, _, wsc_id) = setup_action_state_with(
        false,
        None,
        Some(Card::StarterCanvas),
        Some(Ability::LiftCard),
    );
    let wsc_id = wsc_id.unwrap();

    let mut rng = WyRand::seed_from_u64(99);
    apply_choice(
        &mut state,
        0,
        &Choice::LiftCard { card: Some(Card::StarterCanvas) },
        &mut rng,
    );

    assert!(!state.players[0].workshop_cards.contains(wsc_id),
        "moved card should have left the workshop");
    assert!(state.players[0].top_cards.contains(wsc_id),
        "moved card should be in the top cards");
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
fn test_score_is_points_plus_half_points_for_bowls_and_boats() {
    for seed in 50..60 {
        for num_players in 2..=4 {
            let state = run_full_game_with_invariants(seed, num_players);
            for (i, player) in state.players.iter().enumerate() {
                let buyer_points: u32 = player
                    .completed_orders
                    .iter()
                    .map(|bi| bi.buyer.points())
                    .sum();
                assert_eq!(
                    calculate_whole_points(player),
                    buyer_points + player.gold_coins,
                    "Whole points mismatch for player {} (seed={}, players={})",
                    i, seed, num_players,
                );
                assert_eq!(
                    score_in_half_points(player),
                    2 * (buyer_points + player.gold_coins) + player.bowls + player.boats,
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
    apply_choice(
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
fn test_workshopping_warehouse_gains_two_bowls() {
    let state = state_after_workshopping(Card::Warehouse);
    assert_eq!(state.players[0].bowls, 2);
    assert_eq!(pending_ability(&state), None, "bowls arrive without a choice");
    assert_eq!(score_in_half_points(&state.players[0]), 2);
}

#[test]
fn test_workshopping_chalk_gains_one_bowl() {
    let state = state_after_workshopping(Card::Chalk);
    assert_eq!(state.players[0].bowls, 1);
    assert_eq!(pending_ability(&state), None);
}

#[test]
fn test_workshopping_primaries_gains_one_of_each_primary() {
    let state = state_after_workshopping(Card::Primaries);
    let wheel = &state.players[0].color_wheel;
    assert_eq!(
        (wheel.get(Color::Red), wheel.get(Color::Yellow), wheel.get(Color::Blue)),
        (2, 2, 2),
        "the wheel starts with one of each primary"
    );
    for color in [Color::Vermilion, Color::Orange, Color::Green] {
        assert_eq!(wheel.get(color), 0, "{:?} is not a primary", color);
    }
    assert_eq!(pending_ability(&state), None, "the primaries arrive without a choice");
}

/// The Primaries card trashes for Workshop ×2, so at most two cards come out
/// of the workshop with it.
#[test]
fn test_trashing_primaries_grants_two_workshop_picks() {
    let mut state = state_with_drafted(Card::Primaries);
    for card in [Card::StarterCeramics, Card::StarterCanvas, Card::StarterFabric] {
        let id = find_card_id(&state, card);
        remove_card_anywhere(&mut state, id);
        state.players[0].workshop_cards.insert(id);
    }

    let two_picks = Choice::TrashAndWorkshop {
        card: Card::Primaries,
        workshop_cards: CopyVec::from_slice(&[Card::StarterCanvas, Card::StarterFabric]),
    };
    let three_picks = Choice::TrashAndWorkshop {
        card: Card::Primaries,
        workshop_cards: CopyVec::from_slice(&[Card::StarterCeramics, Card::StarterCanvas, Card::StarterFabric]),
    };
    let choices = enumerate_choices(&state, 0);
    assert!(choices.contains(&two_picks), "two picks are on offer");
    assert!(!choices.contains(&three_picks), "a third pick is not");

    let mut rng = WyRand::seed_from_u64(3);
    apply_choice(&mut state, 0, &two_picks, &mut rng);

    let player = &state.players[0];
    assert_eq!((player.materials_wheel.get(MaterialType::Textiles), player.materials_wheel.get(MaterialType::Paintings)), (1, 1));
    assert_eq!(player.workshop_cards.len(), 1, "the unpicked card stays in the workshop");
    assert_eq!(state.trash.len(), 1, "the card is trashed, not kept");
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
    apply_choice(&mut state, 0, &choice, &mut rng);

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
        apply_choice(&mut after, 0, choice, &mut rng);
        let outcome = (after.players[0].color_wheel.counts, after.players[0].bowls);
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
    apply_choice(
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
    state.players[0].bowls = 0;
    let trash = Choice::TrashAndWorkshop {
        card: Card::CeramicsRed,
        workshop_cards: CopyVec::new(),
    };
    assert!(enumerate_choices(&state, 0).contains(&trash));

    let mut rng = WyRand::seed_from_u64(0);
    apply_choice(&mut state, 0, &trash, &mut rng);
    assert_eq!(
        pending_ability(&state),
        None,
        "the mix ability went with the trash that could not use it"
    );
    assert!(
        enumerate_choices(&state, 0).contains(&Choice::EndTurn { keep_remaining: true }),
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
    state.players[0].bowls = 0;
    let mut rng = WyRand::seed_from_u64(0);
    apply_choice(
        &mut state,
        0,
        &Choice::TrashAndWorkshop {
            card: Card::CeramicsRed,
            workshop_cards: CopyVec::new(),
        },
        &mut rng,
    );
    let before = state.players[0].clone();

    apply_choice(&mut state, 0, &Choice::MixAll { mixes: CopyVec::new() }, &mut rng);
    assert_eq!(state.players[0].color_wheel, before.color_wheel);
    assert_eq!(state.players[0].bowls, before.bowls);
    assert_eq!(pending_ability(&state), None);
}

/// Put `card` in player 0's top cards with the action phase open and nothing
/// pending.
fn state_with_drafted(card: Card) -> GameState {
    let (mut state, _, _, _) = setup_action_state_with(false, None, None, None);
    let id = find_card_id(&state, card);
    remove_card_anywhere(&mut state, id);
    state.players[0].top_cards.insert(id);
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
    apply_choice(&mut state, 0, &trash, &mut rng);

    assert_eq!(state.players[0].materials_wheel.get(MaterialType::Textiles), 1);
    assert_eq!(
        pending_ability(&state),
        Some(Ability::MixColors { count: 1 }),
        "the mix follows the workshop"
    );

    apply_choice(
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
    apply_choice(&mut state, 0, &trash, &mut rng);

    assert_eq!(
        pending_ability(&state),
        Some(Ability::MixColors { count: 1 }),
        "the workshop fizzling must not take the mix with it"
    );
}

#[test]
fn test_a_bowl_stands_in_for_a_missing_primary_in_a_mix() {
    let mut state = state_with_drafted(Card::Realgar);
    let player = &mut state.players[0];
    player.color_wheel = ColorWheel::new();
    player.color_wheel.set(Color::Orange, 1);

    let mix = Choice::TrashAndMix {
        card: Card::Realgar,
        mixes: CopyVec::from_slice(&[(Color::Red, Color::Orange)]),
    };
    assert!(!enumerate_choices(&state, 0).contains(&mix), "no Red and no bowl");
    assert!(!check_choice_available(&state, 0, &mix));

    state.players[0].bowls = 1;
    assert!(enumerate_choices(&state, 0).contains(&mix));
    assert!(check_choice_available(&state, 0, &mix));

    let mut rng = WyRand::seed_from_u64(3);
    apply_choice(&mut state, 0, &mix, &mut rng);
    assert_eq!(state.players[0].bowls, 0);
    assert_eq!(state.players[0].color_wheel.get(Color::Vermilion), 1);
    assert_eq!(state.players[0].color_wheel.get(Color::Orange), 0);
}

/// Give `player` exactly what `buyer` asks for.
fn stock_for(player: &mut PlayerState, buyer: Buyer) {
    for &material in buyer.required_materials() {
        player.materials_wheel.increment(material);
    }
    player.boats += buyer.required_boats();
    for &color in buyer.color_cost() {
        player.color_wheel.increment(color);
    }
}

/// Give player 0 a buyer for `buyer` and a pending Sell.
fn state_selling(buyer: Buyer) -> GameState {
    let (mut state, _, _, _) = setup_action_state_with(false, None, None, Some(Ability::Sell));
    state.players[0].color_wheel = ColorWheel::new();
    state.players[0].buyers.push(colori_core::types::BuyerInstance {
        instance_id: 200,
        buyer,
    });
    state
}

#[test]
fn test_a_boat_and_a_bowl_stand_in_for_what_a_sale_is_missing() {
    let buyer = Buyer::DyerRedYellow;
    let mut state = state_selling(buyer);
    state.players[0].color_wheel.set(Color::Yellow, 1);
    let select = Choice::SellToBuyer { buyer };

    assert!(enumerate_choices(&state, 0).is_empty(), "no Textiles and no Red");
    state.players[0].boats = 1;
    assert!(!check_choice_available(&state, 0, &select), "still no Red");
    state.players[0].bowls = 1;
    assert!(enumerate_choices(&state, 0).contains(&select));

    let mut rng = WyRand::seed_from_u64(3);
    apply_choice(&mut state, 0, &select, &mut rng);
    let player = &state.players[0];
    assert_eq!((player.boats, player.bowls), (0, 0));
    assert_eq!(player.color_wheel.get(Color::Yellow), 0);
    assert_eq!(player.completed_orders.len(), 1);
    assert_eq!(player.cached_whole_points, calculate_whole_points(player));
    assert_eq!(score_in_half_points(player), 2 * buyer.points());
}

#[test]
fn test_a_sale_spends_what_it_asks_for_before_any_boat_or_bowl() {
    let buyer = Buyer::DyerRedYellow;
    let mut state = state_selling(buyer);
    let player = &mut state.players[0];
    stock_for(player, buyer);
    player.boats = 1;
    player.bowls = 1;

    let mut rng = WyRand::seed_from_u64(3);
    apply_choice(&mut state, 0, &Choice::SellToBuyer { buyer }, &mut rng);
    let player = &state.players[0];
    assert_eq!((player.boats, player.bowls), (1, 1));
    assert_eq!(player.materials_wheel.get(MaterialType::Textiles), 0);
    assert_eq!(player.color_wheel.get(Color::Red), 0);
}

#[test]
fn test_a_shipwright_takes_a_boat_as_well_as_its_textiles() {
    let buyer = Buyer::ShipwrightPurpleGreen;
    let select = Choice::SellToBuyer { buyer };
    let mut state = state_selling(buyer);
    let player = &mut state.players[0];
    player.color_wheel.set(Color::Purple, 1);
    player.color_wheel.set(Color::Green, 1);
    player.materials_wheel.increment(MaterialType::Textiles);
    assert!(!check_choice_available(&state, 0, &select), "no boat");

    state.players[0].boats = 1;
    assert!(check_choice_available(&state, 0, &select));
    let mut rng = WyRand::seed_from_u64(3);
    apply_choice(&mut state, 0, &select, &mut rng);
    let player = &state.players[0];
    assert_eq!(player.boats, 0);
    assert_eq!(player.materials_wheel.get(MaterialType::Textiles), 0);
    assert_eq!(player.cached_whole_points, 4);
}

#[test]
fn test_a_shipwright_without_textiles_takes_a_second_boat() {
    let buyer = Buyer::ShipwrightOrangeGreen;
    let select = Choice::SellToBuyer { buyer };
    let mut state = state_selling(buyer);
    let player = &mut state.players[0];
    player.color_wheel.set(Color::Orange, 1);
    player.color_wheel.set(Color::Green, 1);
    player.boats = 1;
    assert!(!check_choice_available(&state, 0, &select), "one boat cannot be both");

    state.players[0].boats = 2;
    let mut rng = WyRand::seed_from_u64(3);
    apply_choice(&mut state, 0, &select, &mut rng);
    assert_eq!(state.players[0].boats, 0);
}

#[test]
fn test_a_tilemaker_takes_two_ceramics() {
    let buyer = Buyer::TilemakerTeal;
    let select = Choice::SellToBuyer { buyer };
    let mut state = state_selling(buyer);
    let player = &mut state.players[0];
    player.color_wheel.set(Color::Teal, 1);
    player.materials_wheel.increment(MaterialType::Ceramics);
    assert!(!check_choice_available(&state, 0, &select), "one Ceramics short");

    // A boat covers the second.
    state.players[0].boats = 1;
    let mut rng = WyRand::seed_from_u64(3);
    apply_choice(&mut state, 0, &select, &mut rng);
    let player = &state.players[0];
    assert_eq!(player.materials_wheel.get(MaterialType::Ceramics), 0);
    assert_eq!(player.boats, 0);
    assert_eq!(player.cached_whole_points, 4);
}

#[test]
fn test_glassmakers_and_illuminators_take_no_material() {
    for buyer in [Buyer::GlassmakerAmberBlue, Buyer::IlluminatorFuchsia] {
        let mut state = state_selling(buyer);
        for &color in buyer.color_cost() {
            state.players[0].color_wheel.increment(color);
        }
        let mut rng = WyRand::seed_from_u64(3);
        apply_choice(&mut state, 0, &Choice::SellToBuyer { buyer }, &mut rng);
        let player = &state.players[0];
        assert_eq!(player.completed_orders.len(), 1, "{:?}", buyer);
        assert_eq!(player.color_wheel.counts.iter().map(|&c| c as u32).sum::<u32>(), 0);
        assert_eq!(player.cached_whole_points, buyer.points());
    }
}

#[test]
fn test_sell_deck_composition() {
    let mut rng = WyRand::seed_from_u64(1);
    let state = create_initial_game_state(2, &[true, true], &mut rng);
    let in_play = |buyer: Buyer| {
        let in_deck = state
            .buyer_deck
            .iter()
            .filter(|&id| state.buyer_lookup[id as usize] == buyer)
            .count();
        let on_display = state.buyer_display.iter().filter(|c| c.buyer == buyer).count();
        in_deck + on_display
    };
    for (buyer, copies) in buyer_deck_cards() {
        assert_eq!(in_play(buyer), copies, "copies of {:?}", buyer);
    }

    let cards_of = |trade: Trade| ALL_BUYERS.iter().filter(|c| c.trade() == trade).count();
    let expected = [
        (Trade::Shipwright, 3, 1, 4),
        (Trade::Glassmaker, 6, 1, 3),
        (Trade::Dyer, 9, 2, 2),
        (Trade::Illuminator, 6, 1, 2),
        (Trade::Potter, 6, 2, 3),
        (Trade::Tilemaker, 6, 1, 4),
        (Trade::Painter, 18, 1, 4),
        (Trade::MasterPainter, 6, 1, 5),
    ];
    for (trade, cards, copies, points) in expected {
        assert_eq!(cards_of(trade), cards, "{:?} cards", trade);
        assert_eq!(trade.points(), points, "{:?} points", trade);
        for buyer in ALL_BUYERS.iter().filter(|c| c.trade() == trade) {
            assert_eq!(buyer_copies(*buyer), copies, "{:?} copies", buyer);
        }
    }
    assert_eq!(buyer_deck_cards().map(|(_, copies)| copies).sum::<usize>(), 75);
}

#[test]
fn test_starter_deck_composition() {
    let mut rng = WyRand::seed_from_u64(1);
    let state = create_initial_game_state(4, &[true; 4], &mut rng);
    for player in state.players.iter() {
        let mut cards: Vec<Card> = player
            .deck
            .cards()
            .union(player.workshop_cards)
            .iter()
            .map(|id| state.card_lookup[id as usize])
            .collect();
        let mut expected = starter_deck().to_vec();
        cards.sort_by_key(|&c| c as usize);
        expected.sort_by_key(|&c| c as usize);
        assert_eq!(cards, expected);
        assert_eq!((player.bowls, player.boats, player.gold_coins), (0, 1, 1));
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
        let mut ids: Vec<u8> = player.deck.cards().union(player.workshop_cards).iter().collect();
        ids.sort_unstable();
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
    assert_eq!(draft_copies(Card::Primaries), 6);
    for tertiary in [
        Card::Cinnabar,
        Card::Saffron,
        Card::CopperResinate,
        Card::Azurite,
        Card::LapisLazuli,
        Card::Cochineal,
    ] {
        assert_eq!(draft_copies(tertiary), 2, "{:?} is a tertiary pigment", tertiary);
    }
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

    // Claim buyers, then finish the draft, always taking the first option.
    while matches!(state.phase, GamePhase::Buyers { .. } | GamePhase::Draft { .. }) {
        let player = actor(&state);
        let choices = enumerate_choices(&state, player);
        apply_choice(&mut state, player, &choices[0], &mut rng);
    }

    let workshop_area = state.players[0]
        .workshop_cards
        .union(state.players[0].used_cards);
    let drafted = state.players[0].top_cards;
    assert!(!workshop_area.is_empty() && !drafted.is_empty());

    apply_choice(&mut state, 0, &Choice::EndTurn { keep_remaining: true }, &mut rng);
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
    assert!(player.used_cards.is_empty());
}

#[test]
fn test_end_of_turn_puts_undecided_top_cards_on_top_of_the_deck() {
    let (state, _, _) = state_at_first_end_of_turn(4);
    let player = &state.players[0];

    assert!(player.top_cards.is_empty());
    assert!(
        player.used_cards.is_empty(),
        "the workshop area goes to the deck, so the turn leaves nothing used behind"
    );
    assert_eq!(
        player.workshop_cards.len(),
        5,
        "what the turn kept, and the deck under it, are drawn as the turn ends"
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
fn test_a_turn_that_kept_cards_draws_them_back_into_the_workshop() {
    let (state, _, _) = state_at_first_end_of_turn(4);

    assert_eq!(
        state.players[0].workshop_cards.len(),
        5,
        "a turn tops the workshop up to five, however many cards it kept"
    );
    assert_eq!(
        state.players[1].workshop_cards.len(),
        5,
        "and seat 1 still holds the workshop setup dealt it"
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
    while matches!(state.phase, GamePhase::Buyers { .. }) {
        let player = actor(&state);
        let choices = enumerate_choices(&state, player);
        apply_choice(&mut state, player, &choices[0], &mut rng);
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
                apply_choice(&mut state, player, &choices[rng.random_range(0..choices.len())], &mut rng);
                picks += 1;
            }

            let context = format!("players={num_players}, seed={seed}");
            assert!(matches!(state.phase, GamePhase::Action { .. }), "{context}");
            assert_eq!(picks, num_players * (DRAFT_HAND_SIZE as usize - 1), "{context}");
            for player in state.players.iter() {
                assert_eq!(player.top_cards.len(), DRAFT_HAND_SIZE, "{context}");
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
        apply_choice(&mut state, player, &choices[0], &mut rng);
    }

    assert_eq!(state.players[0].top_cards.len(), DRAFT_HAND_SIZE);
    assert_eq!(state.trash.len(), trashed_before + DRAFT_HAND_SIZE);
}

// ── Buyers phase ──

fn buyers_of(state: &GameState, player: usize) -> Vec<Buyer> {
    state.players[player].buyers.iter().map(|b| b.buyer).collect()
}

/// Advance to the buyers phase of `round`, resolving every earlier phase by
/// always taking the first legal option.
fn state_at_buyers_phase(seed: u64, num_players: usize, round: u32) -> (GameState, WyRand) {
    let mut rng = WyRand::seed_from_u64(seed);
    let bot_players = vec![true; num_players];
    let mut state = create_initial_game_state(num_players, &bot_players, &mut rng);

    while state.round < round {
        let player = actor(&state);
        let choices = enumerate_choices(&state, player);
        apply_choice(&mut state, player, &choices[0], &mut rng);
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
    let state = create_initial_game_state(3, &[true, true, true], &mut rng);

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
            apply_choice(&mut state, player, &choices[0], &mut rng);
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
        apply_choice(&mut state, player, &choices[0], &mut rng);
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
            state.buyer_display.len(),
            MAX_BUYER_DISPLAY,
            "the display should be full whenever someone is choosing"
        );
        let player = actor(&state);
        let choices = enumerate_choices(&state, player);
        apply_choice(&mut state, player, &choices[0], &mut rng);
    }
}

#[test]
fn test_claiming_off_the_deck_takes_a_card_nobody_could_see() {
    let (mut state, mut rng) = state_at_buyers_phase(9, 2, 1);
    let display_before: Vec<Buyer> =
        state.buyer_display.iter().map(|c| c.buyer).collect();
    let deck_before = state.buyer_deck.len();
    let player = match &state.phase {
        GamePhase::Buyers { buyers_state } => buyers_state.current_player_index,
        _ => unreachable!(),
    };

    assert!(enumerate_choices(&state, player).contains(&Choice::DrawBuyer));
    apply_choice(&mut state, player, &Choice::DrawBuyer, &mut rng);

    assert_eq!(state.players[player].buyers.len(), 1);
    assert_eq!(state.buyer_deck.len(), deck_before - 1);
    assert_eq!(
        state.buyer_display.iter().map(|c| c.buyer).collect::<Vec<_>>(),
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
    let buyer = state.buyer_display[0].buyer;
    let instance = state.buyer_display.remove(0);
    state.players[0].buyers.push(instance);
    stock_for(&mut state.players[0], buyer);

    let display_before: Vec<Buyer> =
        state.buyer_display.iter().map(|c| c.buyer).collect();
    let deck_before = state.buyer_deck.len();

    let mut ability_stack = AbilityStack::new();
    ability_stack.push(Ability::Sell);
    state.phase = open_action_phase(&state, ability_stack);

    apply_choice(&mut state, 0, &Choice::SellToBuyer { buyer }, &mut rng);

    assert!(state.players[0].buyers.is_empty(), "the slot should be empty now");
    assert_eq!(state.players[0].completed_orders.len(), 1);
    assert_eq!(
        state.buyer_display.iter().map(|c| c.buyer).collect::<Vec<_>>(),
        display_before,
        "a sale must not disturb the shared display"
    );
    assert_eq!(state.buyer_deck.len(), deck_before, "and must not draw");
}

/// A sold buyer leaves a hole, and the next buyers phase is what fills it.
#[test]
fn test_an_emptied_slot_is_refilled_next_round() {
    // Round 2 always opens with a buyers phase, since capacity has just grown.
    let (mut state, mut rng) = state_at_buyers_phase(21, 2, 2);
    while matches!(state.phase, GamePhase::Buyers { .. }) {
        let player = actor(&state);
        let choices = enumerate_choices(&state, player);
        apply_choice(&mut state, player, &choices[0], &mut rng);
    }
    assert_eq!(state.players[0].buyers.len(), 2);

    let kept = buyers_of(&state, 0)[1];
    state.players[0].buyers.remove(0);

    // Skip ahead to the next buyers phase.
    while !matches!(state.phase, GamePhase::Buyers { .. }) {
        let player = actor(&state);
        let choices = enumerate_choices(&state, player);
        apply_choice(&mut state, player, &choices[0], &mut rng);
    }
    assert_eq!(state.round, 3);
    while matches!(state.phase, GamePhase::Buyers { .. }) {
        let player = actor(&state);
        let choices = enumerate_choices(&state, player);
        apply_choice(&mut state, player, &choices[0], &mut rng);
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
fn test_an_exhausted_buyer_supply_skips_the_buyers_phase() {
    let mut rng = WyRand::seed_from_u64(12);
    let mut state = create_initial_game_state(2, &[true, true], &mut rng);
    state.buyer_deck = UnorderedBuyers::new();
    while !state.buyer_display.is_empty() {
        state.buyer_display.remove(0);
    }
    initialize_buyers_phase(&mut state, &mut rng);

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
        apply_choice(&mut state, player, &choices[0], &mut rng);
        guard += 1;
        assert!(guard < 5_000, "game did not finish with an exhausted buyer supply");
    }
    for player in state.players.iter() {
        assert!(player.completed_orders.is_empty());
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
            let bot_players = vec![true; num_players];
            let mut state = create_initial_game_state(num_players, &bot_players, &mut rng);
            let initial = state.clone();

            state.draw_log = Some(DrawLog::Recording(Vec::new()));
            let initial_draws = take_recording(&mut state);

            let mut script: Vec<(usize, Choice, Vec<DrawEvent>)> = Vec::new();
            while !matches!(state.phase, GamePhase::GameOver) {
                let player = random_actor(&state, &mut rng);
                let choices = enumerate_choices(&state, player);
                assert!(!choices.is_empty(), "no legal choice (players={num_players}, seed={seed})");
                let choice = choices[rng.random_range(0..choices.len())].clone();
                state.draw_log = Some(DrawLog::Recording(Vec::new()));
                apply_choice(&mut state, player, &choice, &mut rng);
                script.push((player, choice, take_recording(&mut state)));
            }
            let recorded = state;

            let mut replay_rng = WyRand::seed_from_u64(0xDEAD_BEEF);
            let mut replayed = initial;
            replayed.draw_log = Some(DrawLog::Replaying(initial_draws.into_iter().collect()));
            replayed.draw_log = None;

            for (player, choice, draws) in script {
                replayed.draw_log = Some(DrawLog::Replaying(draws.into_iter().collect()));
                apply_choice(&mut replayed, player, &choice, &mut replay_rng);
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

// ── GoldCoin income and spending ──

#[test]
fn test_income_arrives_in_rounds_four_to_seven() {
    for round in 1..=7u32 {
        let mut rng = WyRand::seed_from_u64(round as u64);
        let mut state = create_initial_game_state(2, &[true, true], &mut rng);
        state.round = round;
        let before: Vec<(u32, u32, u32)> = state
            .players
            .iter()
            .map(|p| (p.gold_coins, p.boats, p.bowls))
            .collect();


        start_round(&mut state, &mut rng);

        let expected = match round {
            4 | 6 => RoundIncome { gold_coins: 1, boats: 0, bowls: 0 },
            5 => RoundIncome { gold_coins: 0, boats: 1, bowls: 0 },
            7 => RoundIncome { gold_coins: 1, boats: 1, bowls: 1 },
            _ => RoundIncome::default(),
        };
        assert_eq!(round_income(round), expected);
        for (i, player) in state.players.iter().enumerate() {
            let (gold_coins, boats, bowls) = before[i];
            assert_eq!(
                (player.gold_coins, player.boats, player.bowls),
                (
                    gold_coins + expected.gold_coins,
                    boats + expected.boats,
                    bowls + expected.bowls
                ),
                "player {i} gold_coins, boats and bowls after round {round} opened"
            );
            assert_eq!(
                player.cached_whole_points,
                calculate_whole_points(player),
                "income must move the cached score with it (round {round})"
            );
        }
    }
}

/// A gold_coin is a point, so paying one has to come off the score as well.
#[test]
fn test_spending_a_gold_coin_costs_a_point_and_stacks_the_purchase() {
    let (mut state, _, _, _) =
        setup_action_state_with(false, None, Some(Card::StarterCeramics), None);
    state.players[0].gold_coins = 2;
    state.players[0].cached_whole_points = calculate_whole_points(&state.players[0]);

    let choice = Choice::SpendGoldCoin { purchase: GoldCoinPurchase::Workshop };
    assert!(enumerate_choices(&state, 0).contains(&choice));
    assert!(check_choice_available(&state, 0, &choice));

    let mut rng = WyRand::seed_from_u64(1);
    apply_choice(&mut state, 0, &choice, &mut rng);

    assert_eq!(state.players[0].gold_coins, 1);
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
    let spend = |p| Choice::SpendGoldCoin { purchase: p };

    // No gold_coins, nothing on offer.
    let (state, _, _, _) = setup_action_state_with(false, None, Some(Card::StarterCeramics), None);
    assert!(!enumerate_choices(&state, 0).iter().any(|c| matches!(c, Choice::SpendGoldCoin { .. })));

    // A gold_coin and a workshop card, but a bare colour wheel and no buyers.
    let (mut state, _, _, _) =
        setup_action_state_with(false, None, Some(Card::StarterCeramics), None);
    state.players[0].gold_coins = 1;
    state.players[0].color_wheel = ColorWheel::new();
    state.players[0].cached_whole_points = calculate_whole_points(&state.players[0]);

    let choices = enumerate_choices(&state, 0);
    assert!(choices.contains(&spend(GoldCoinPurchase::Workshop)));
    assert!(!choices.contains(&spend(GoldCoinPurchase::MixColors)), "no two colours to mix");
    assert!(!choices.contains(&spend(GoldCoinPurchase::Sell)), "no buyer to sell to");

    // A mixable pair brings the mix back.
    state.players[0].color_wheel.increment(Color::Red);
    state.players[0].color_wheel.increment(Color::Yellow);
    assert!(enumerate_choices(&state, 0).contains(&spend(GoldCoinPurchase::MixColors)));

    // Emptying the workshop takes the workshop pick away.
    state.players[0].workshop_cards = UnorderedCards::new();
    assert!(!enumerate_choices(&state, 0).contains(&spend(GoldCoinPurchase::Workshop)));
}

/// Buying is a turn action, not something to interleave with an ability that
/// is already waiting for input.
#[test]
fn test_a_gold_coin_cannot_be_spent_while_an_ability_is_pending() {
    let (mut state, _, _, _) = setup_action_state_with(
        false,
        None,
        Some(Card::StarterCeramics),
        Some(Ability::Workshop { count: 2 }),
    );
    state.players[0].gold_coins = 3;
    state.players[0].cached_whole_points = calculate_whole_points(&state.players[0]);

    let choices = enumerate_choices(&state, 0);
    assert!(!choices.iter().any(|c| matches!(c, Choice::SpendGoldCoin { .. })));
    for purchase in colori_core::types::ALL_GOLD_COIN_PURCHASES {
        assert!(!check_choice_available(&state, 0, &Choice::SpendGoldCoin { purchase }));
    }
}

/// The whole point of the Sell purchase: one gold_coin in, a buyer worth two
/// to five points out.
#[test]
fn test_a_gold_coin_buys_a_sale_that_pays_for_itself() {
    let mut rng = WyRand::seed_from_u64(4);
    let mut state = create_initial_game_state(2, &[true, true], &mut rng);

    let instance = state.buyer_display.remove(0);
    let buyer = instance.buyer;
    state.players[0].buyers.push(instance);
    stock_for(&mut state.players[0], buyer);
    state.players[0].gold_coins = 1;
    state.players[0].cached_whole_points = calculate_whole_points(&state.players[0]);
    let score_before = state.players[0].cached_whole_points;

    state.phase = open_action_phase(&state, AbilityStack::new());

    let spend = Choice::SpendGoldCoin { purchase: GoldCoinPurchase::Sell };
    assert!(enumerate_choices(&state, 0).contains(&spend));
    apply_choice(&mut state, 0, &spend, &mut rng);
    apply_choice(&mut state, 0, &Choice::SellToBuyer { buyer }, &mut rng);

    assert_eq!(state.players[0].gold_coins, 0, "the gold_coin was spent");
    assert_eq!(state.players[0].completed_orders.len(), 1);
    assert_eq!(
        state.players[0].cached_whole_points,
        score_before - 1 + buyer.points(),
        "net gain is the buyer's value less the gold_coin"
    );
    assert_eq!(state.players[0].cached_whole_points, calculate_whole_points(&state.players[0]));
}

// ── Simultaneous action phase ──

/// The first action phase of a game, reached by random play.
fn state_at_first_action_phase(seed: u64, num_players: usize) -> (GameState, WyRand) {
    let mut rng = WyRand::seed_from_u64(seed);
    let mut state = create_initial_game_state(num_players, &vec![true; num_players], &mut rng);
    while !matches!(state.phase, GamePhase::Action { .. }) {
        let player = actor(&state);
        let choices = enumerate_choices(&state, player);
        let choice = choices[rng.random_range(0..choices.len())].clone();
        apply_choice(&mut state, player, &choice, &mut rng);
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
                views.apply(player, &choice, rng);
                let ended = matches!(choice, Choice::EndTurn { .. });
                turn.push(choice);
                if ended {
                    return turn;
                }
            }
        })
        .collect()
}

/// Nobody's turn touches anyone else's, so the order the choices within the
/// round are taken in cannot change where it ends up.
///
/// A turn's last act draws the workshop, and that draw is where the shared
/// generator is spent. So the two runs below end the seats' turns in the same
/// order and only interleave the decisions before them: a different order of
/// draws would sample the decks differently, which is randomness rather than
/// anyone's moves telling.
#[test]
fn test_interleaving_turns_does_not_change_the_outcome() {
    for num_players in 2..=4 {
        for seed in 0..10 {
            let (start, mut rng) = state_at_first_action_phase(seed, num_players);
            let turns = random_turns(&start, &mut rng);

            // Each run gets its own generator, seeded alike: a draw spends the
            // shared one, so one generator cannot serve both.
            let mut one_after_another = start.clone();
            let mut sequential_rng = WyRand::seed_from_u64(0);
            for (player, turn) in turns.iter().enumerate() {
                for choice in turn {
                    apply_choice(&mut one_after_another, player, choice, &mut sequential_rng);
                }
            }

            let mut interleaved = start.clone();
            let mut interleaved_rng = WyRand::seed_from_u64(0);
            let mut next = vec![0usize; num_players];
            let is_ending = |player: usize, next: &[usize], turns: &[Vec<Choice>]| {
                matches!(turns[player].get(next[player]), Some(Choice::EndTurn { .. }))
            };
            while next.iter().zip(&turns).any(|(&i, turn)| i < turn.len()) {
                let open: Vec<usize> = (0..num_players)
                    .filter(|&p| next[p] < turns[p].len() && !is_ending(p, &next, &turns))
                    .collect();
                let player = if open.is_empty() {
                    (0..num_players)
                        .find(|&p| next[p] < turns[p].len())
                        .expect("a turn is still open")
                } else {
                    // Which seat goes next is drawn from `rng`, so both runs
                    // spend their own generator on the game and nothing else.
                    open[rng.random_range(0..open.len())]
                };
                apply_choice(
                    &mut interleaved,
                    player,
                    &turns[player][next[player]],
                    &mut interleaved_rng,
                );
                next[player] += 1;
            }

            let context = format!("players={num_players}, seed={seed}");
            assert!(matches!(one_after_another.phase, GamePhase::Buyers { .. }), "{context}");
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
            apply_choice(&mut state, player, &choice, &mut rng);
        }
        assert!(matches!(state.phase, GamePhase::Action { .. }), "player 1 has not ended");
    }
    assert_eq!(actor(&state), 1);

    let round = state.round;
    while state.round == round {
        let choices = enumerate_choices(&state, 1);
        apply_choice(&mut state, 1, choices.last().unwrap(), &mut rng);
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
    apply_choice(&mut state, 0, &Choice::EndTurn { keep_remaining: true }, &mut rng);
    // Enough to buy a workshop pick, were the turn still open, and a card in
    // the workshop for it to take.
    state.players[0].gold_coins = 3;
    state.players[0].cached_whole_points = calculate_whole_points(&state.players[0]);
    let mut drawn = UnorderedCards::new();
    state.players[0]
        .deck
        .draw_into(&mut drawn, 1, &mut rng);
    state.players[0].workshop_cards = drawn;
    assert!(!state.players[0].workshop_cards.is_empty());

    assert!(enumerate_choices(&state, 0).is_empty());
    assert!(!check_choice_available(&state, 0, &Choice::EndTurn { keep_remaining: true }));
    assert!(!check_choice_available(
        &state,
        0,
        &Choice::SpendGoldCoin { purchase: GoldCoinPurchase::Workshop }
    ));
    assert!(
        check_choice_available(&state, 1, &Choice::EndTurn { keep_remaining: true }),
        "a pending ability no longer stands in the way of ending a turn"
    );
}

/// A turn left mid-ability can end: what it was answering is skipped rather
/// than resolved, so a sale the rules would otherwise demand never happens.
#[test]
fn test_ending_a_turn_skips_the_ability_it_was_answering() {
    let (mut state, mut rng) = state_at_first_action_phase(12, 2);
    // A wheel, boats and wilds that can pay for any buyer, so the pending Sell
    // is one the rules would make mandatory.
    state.players[0].color_wheel = ColorWheel {
        counts: [9; colori_core::types::NUM_COLORS],
    };
    state.players[0].boats = 9;
    state.players[0].bowls = 9;
    let buyers = state.players[0].buyers.clone();
    assert!(!buyers.is_empty(), "round one claims a buyer");
    let mut stack = AbilityStack::new();
    stack.push(Ability::Sell);
    if let GamePhase::Action { ref mut action_state } = state.phase {
        action_state.turns[0].ability_stack = stack;
    }

    apply_choice(&mut state, 0, &Choice::EndTurn { keep_remaining: true }, &mut rng);

    assert!(matches!(
        &state.phase,
        GamePhase::Action { action_state } if action_state.turns[0].ended
    ));
    assert_eq!(state.players[0].buyers, buyers, "the sale was skipped, not made");
    assert_eq!(
        state.players[0].color_wheel.counts,
        [9; colori_core::types::NUM_COLORS],
        "nothing was spent to pay for it"
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
        // A turn that ends draws, so the two runs need a generator each, seeded
        // alike, to draw the same cards.
        let mut live_rng = WyRand::seed_from_u64(0);
        let mut view_rng = WyRand::seed_from_u64(0);

        for (player, turn) in turns.iter().enumerate().take(2) {
            for choice in turn {
                apply_choice(&mut live, player, choice, &mut live_rng);
                views.apply(player, choice, &mut view_rng);
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
        (r#"{"type":"destroyDraftedCard","card":"Alum"}"#, "trashTopCard"),
        (r#"{"type":"destroyAndMix","card":"Woad","mixes":[]}"#, "trashAndMix"),
        (r#"{"type":"destroyAndMixAll","card":"Woad","mixes":[]}"#, "trashAndMix"),
        (r#"{"type":"destroyAndSell","card":"Indigo","sellCard":"DyerGreen"}"#, "trashAndSell"),
        (r#"{"type":"destroyAndWorkshop","card":"Cinnabar","workshopCards":[]}"#, "trashAndWorkshop"),
        (r#"{"type":"destroyAndLiftCard","card":"Alum","target":null}"#, "trashAndLift"),
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

/// Game logs and saved games from before the vocabulary changed still load:
/// the old field and ability names are serde aliases of the new ones.
#[test]
fn states_and_abilities_written_with_the_old_names_still_load() {
    let mut rng = WyRand::seed_from_u64(2);
    let state = create_initial_game_state(2, &[true, false], &mut rng);
    let json = serde_json::to_value(&state).unwrap();
    let old_names = |name: &'static str| -> &'static str {
        match name {
            "usedCards" => "workshoppedCards",
            "topCards" => "draftedCards",
            "materialsWheel" => "materials",
            "completedOrders" => "completedSellCards",
            "goldCoins" => "ducats",
            "bowls" => "wildPrimaries",
            "buyerDeck" => "sellCardDeck",
            "buyerDisplay" => "sellCardDisplay",
            "botPlayers" => "aiPlayers",
            other => other,
        }
    };
    let rename = |value: &mut serde_json::Value| {
        let Some(fields) = value.as_object_mut() else { return };
        for new in [
            "usedCards", "topCards", "materialsWheel", "completedOrders", "goldCoins", "bowls",
            "buyerDeck", "buyerDisplay", "botPlayers",
        ] {
            if let Some(field) = fields.remove(new) {
                fields.insert(old_names(new).to_string(), field);
            }
        }
    };

    let mut renamed = json.clone();
    rename(&mut renamed);
    if let Some(players) = renamed.get_mut("players").and_then(|p| p.as_array_mut()) {
        for player in players {
            rename(player);
        }
    }
    let reloaded: GameState = serde_json::from_value(renamed)
        .unwrap_or_else(|e| panic!("an old save should load: {e}"));
    assert_eq!(
        serde_json::to_value(&reloaded).unwrap(),
        json,
        "the old names say the same game",
    );

    for (old, expected) in [
        (r#"{"type":"gainDucats","count":2}"#, Ability::GainGoldCoins { count: 2 }),
        (r#"{"type":"gainWildPrimaries","count":1}"#, Ability::GainBowls { count: 1 }),
        (r#"{"type":"moveToDraftPool"}"#, Ability::LiftCard),
    ] {
        let ability: Ability = serde_json::from_str(old).unwrap_or_else(|e| panic!("{old}: {e}"));
        assert_eq!(ability, expected, "{old}");
    }
}
