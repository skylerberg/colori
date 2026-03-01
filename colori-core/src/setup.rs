use crate::cards::*;
use crate::fixed_vec::FixedVec;
use crate::types::*;
use crate::unordered_cards::{set_buyer_registry, set_card_registry, UnorderedBuyers, UnorderedCards};
use rand::Rng;
use smallvec::SmallVec;
use std::cell::Cell;

thread_local! {
    static NEXT_CARD_ID: Cell<u32> = const { Cell::new(0) };
    static NEXT_BUYER_ID: Cell<u32> = const { Cell::new(0) };
}

fn next_card_id() -> u8 {
    NEXT_CARD_ID.with(|c| {
        let id = c.get();
        c.set(id + 1);
        id as u8
    })
}

fn next_buyer_id() -> u8 {
    NEXT_BUYER_ID.with(|c| {
        let id = c.get();
        c.set(id + 1);
        id as u8
    })
}

fn reset_id_counters() {
    NEXT_CARD_ID.with(|c| c.set(0));
    NEXT_BUYER_ID.with(|c| c.set(0));
}

pub fn create_initial_game_state<R: Rng>(num_players: usize, ai_players: &[bool], rng: &mut R) -> GameState {
    reset_id_counters();

    let mut card_lookup = [Card::BasicRed; 256];
    let mut buyer_lookup = [BuyerCard::Textiles2Vermilion; 256];

    // Build each player's starting state
    let players: FixedVec<PlayerState, MAX_PLAYERS> = (0..num_players)
        .map(|_| {
            let personal_cards = [
                Card::BasicRed, Card::BasicYellow, Card::BasicBlue,
                Card::StarterCeramics, Card::StarterPaintings, Card::StarterTextiles,
                Card::Argol,
            ];

            let mut deck = UnorderedCards::new();
            for &card in &personal_cards {
                let id = next_card_id();
                card_lookup[id as usize] = card;
                deck.insert(id);
            }

            let mut color_wheel = ColorWheel::new();
            color_wheel.set(Color::Red, 1);
            color_wheel.set(Color::Yellow, 1);
            color_wheel.set(Color::Blue, 1);

            PlayerState {
                deck,
                discard: UnorderedCards::new(),
                used_cards: UnorderedCards::new(),
                workshop_cards: UnorderedCards::new(),
                drafted_cards: UnorderedCards::new(),
                color_wheel,
                materials: Materials::new(),
                completed_buyers: SmallVec::new(),
                ducats: 0,
                cached_score: 0,
            }
        })
        .collect();

    // Build draft deck
    let mut draft_deck = UnorderedCards::new();

    for dye in dye_cards() {
        for _ in 0..DYE_COPIES {
            let id = next_card_id();
            card_lookup[id as usize] = dye;
            draft_deck.insert(id);
        }
    }

    for &mat in &draft_material_cards() {
        for _ in 0..MATERIAL_COPIES {
            let id = next_card_id();
            card_lookup[id as usize] = mat;
            draft_deck.insert(id);
        }
    }

    for action in action_cards() {
        for _ in 0..ACTION_COPIES {
            let id = next_card_id();
            card_lookup[id as usize] = action;
            draft_deck.insert(id);
        }
    }

    // Build buyer deck
    let mut buyer_deck = UnorderedBuyers::new();
    for &buyer in &generate_all_buyers() {
        let id = next_buyer_id();
        buyer_lookup[id as usize] = buyer;
        buyer_deck.insert(id);
    }

    // Deal 6 buyers from buyer_deck to buyer_display
    let mut buyer_display: FixedVec<BuyerInstance, MAX_BUYER_DISPLAY> = FixedVec::new();
    let drawn_buyers = buyer_deck.draw_multiple(6, rng);
    for id in drawn_buyers.iter() {
        buyer_display.push(BuyerInstance {
            instance_id: id as u32,
            buyer: buyer_lookup[id as usize],
        });
    }

    set_card_registry(&card_lookup);
    set_buyer_registry(&buyer_lookup);

    GameState {
        players,
        draft_deck,
        destroyed_pile: UnorderedCards::new(),
        buyer_deck,
        buyer_display,
        phase: GamePhase::Draw,
        round: 1,
        ai_players: FixedVec::from_slice(ai_players),
        card_lookup,
        buyer_lookup,
    }
}
