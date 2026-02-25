use crate::cards::*;
use crate::deck_utils::shuffle_in_place;
use crate::types::*;
use rand::Rng;
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_INSTANCE_ID: AtomicU32 = AtomicU32::new(1);

fn create_card_instances(cards: &[AnyCard]) -> Vec<CardInstance> {
    cards
        .iter()
        .map(|card| {
            let id = NEXT_INSTANCE_ID.fetch_add(1, Ordering::Relaxed);
            CardInstance {
                instance_id: id,
                card: card.clone(),
            }
        })
        .collect()
}

pub fn reset_instance_id_counter() {
    NEXT_INSTANCE_ID.store(1, Ordering::Relaxed);
}

pub fn create_initial_game_state<R: Rng>(player_names: &[String], ai_players: &[bool], rng: &mut R) -> GameState {
    // Build each player's starting state
    let players: Vec<PlayerState> = player_names
        .iter()
        .map(|name| {
            // 7 starting cards: 3 basic dyes + 3 starter materials + chalk
            let mut personal_cards: Vec<AnyCard> = Vec::with_capacity(7);
            personal_cards.extend(basic_dye_cards());
            personal_cards.extend(starter_material_cards());
            personal_cards.push(chalk_card());

            let mut deck = create_card_instances(&personal_cards);
            shuffle_in_place(&mut deck, rng);

            // Starting color wheel: Red=1, Yellow=1, Blue=1
            let mut color_wheel = ColorWheel::new();
            color_wheel.set(Color::Red, 1);
            color_wheel.set(Color::Yellow, 1);
            color_wheel.set(Color::Blue, 1);

            // Starting materials: 1 of each type
            let mut materials = Materials::new();
            materials.increment(MaterialType::Textiles);
            materials.increment(MaterialType::Ceramics);
            materials.increment(MaterialType::Paintings);

            PlayerState {
                name: name.clone(),
                deck,
                discard: Vec::new(),
                workshop_cards: Vec::new(),
                drafted_cards: Vec::new(),
                color_wheel,
                materials,
                completed_buyers: Vec::new(),
                ducats: 0,
            }
        })
        .collect();

    // Build draft deck: 4x each of 15 dye cards + 1x each of 15 draft materials + 3x each of 5 action cards = 90 cards
    let mut draft_cards: Vec<AnyCard> = Vec::with_capacity(90);

    // 4 copies of each of 15 dye cards (60 total)
    for dye in dye_cards() {
        for _ in 0..4 {
            draft_cards.push(dye.clone());
        }
    }

    // 1 copy of each of 15 draft material cards (15 total)
    for material in draft_material_cards() {
        draft_cards.push(material);
    }

    // 3 copies of each of 5 action cards (15 total)
    for action in action_cards() {
        for _ in 0..3 {
            draft_cards.push(action.clone());
        }
    }

    let mut draft_deck = create_card_instances(&draft_cards);
    shuffle_in_place(&mut draft_deck, rng);

    // Build buyer deck: all 51 buyers shuffled
    let all_buyers = generate_all_buyers();
    let mut buyer_deck = create_card_instances(&all_buyers);
    shuffle_in_place(&mut buyer_deck, rng);

    // Deal 6 buyers from buyer_deck to buyer_display (pop from end)
    let mut buyer_display: Vec<CardInstance> = Vec::with_capacity(6);
    for _ in 0..6 {
        if let Some(buyer) = buyer_deck.pop() {
            buyer_display.push(buyer);
        }
    }

    GameState {
        players,
        draft_deck,
        destroyed_pile: Vec::new(),
        buyer_deck,
        buyer_display,
        phase: GamePhase::Draw,
        round: 1,
        ai_players: ai_players.to_vec(),
    }
}
