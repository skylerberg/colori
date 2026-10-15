use crate::cards::*;
use crate::deck::Deck;
use crate::fixed_vec::FixedVec;
use crate::types::*;
use crate::unordered_cards::{set_sell_card_registry, set_card_registry, UnorderedSellCards, UnorderedCards};
use rand::Rng;
use smallvec::SmallVec;
use std::cell::Cell;

thread_local! {
    static NEXT_CARD_ID: Cell<u32> = const { Cell::new(0) };
    static NEXT_SELL_CARD_ID: Cell<u32> = const { Cell::new(0) };
}

fn next_card_id() -> u8 {
    NEXT_CARD_ID.with(|c| {
        let id = c.get();
        c.set(id + 1);
        id as u8
    })
}

fn next_sell_card_id() -> u8 {
    NEXT_SELL_CARD_ID.with(|c| {
        let id = c.get();
        c.set(id + 1);
        id as u8
    })
}

fn reset_id_counters() {
    NEXT_CARD_ID.with(|c| c.set(0));
    NEXT_SELL_CARD_ID.with(|c| c.set(0));
}

pub fn create_initial_game_state<R: Rng>(num_players: usize, ai_players: &[bool], rng: &mut R) -> GameState {
    reset_id_counters();

    let mut card_lookup = [Card::BasicRed; 256];
    let mut sell_card_lookup = [ALL_SELL_CARDS[0]; 256];

    // Starter cards take the first ids, a deck's worth per seat in seat order.
    // The frontend reads a starter card's character portrait off its id.
    let players: FixedVec<PlayerState, MAX_PLAYERS> = (0..num_players)
        .map(|_| {
            let mut starting_cards = UnorderedCards::new();
            for card in starter_deck() {
                let id = next_card_id();
                card_lookup[id as usize] = card;
                starting_cards.insert(id);
            }

            let mut color_wheel = ColorWheel::new();
            color_wheel.set(Color::Red, 1);
            color_wheel.set(Color::Yellow, 1);
            color_wheel.set(Color::Blue, 1);

            PlayerState {
                deck: Deck::from_cards(starting_cards),
                workshopped_cards: UnorderedCards::new(),
                workshop_cards: UnorderedCards::new(),
                drafted_cards: UnorderedCards::new(),
                color_wheel,
                materials: Materials::new(),
                buyers: FixedVec::new(),
                completed_sell_cards: SmallVec::new(),
                ducats: 0,
                wild_primaries: 0,
                boats: 0,
                cached_ducat_score: 0,
            }
        })
        .collect();

    let mut draft_deck = UnorderedCards::new();
    for (card, copies) in draft_deck_cards() {
        for _ in 0..copies {
            let id = next_card_id();
            card_lookup[id as usize] = card;
            draft_deck.insert(id);
        }
    }

    // Build sell card deck
    let mut sell_card_deck = UnorderedSellCards::new();
    for (sell_card, copies) in sell_deck_cards() {
        for _ in 0..copies {
            let id = next_sell_card_id();
            sell_card_lookup[id as usize] = sell_card;
            sell_card_deck.insert(id);
        }
    }

    let mut sell_card_display: FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY> = FixedVec::new();
    let drawn_sell_cards = sell_card_deck.draw_multiple(MAX_SELL_CARD_DISPLAY as u32, rng);
    for id in drawn_sell_cards.iter() {
        sell_card_display.push(SellCardInstance {
            instance_id: id as u32,
            sell_card: sell_card_lookup[id as usize],
        });
    }

    set_card_registry(&card_lookup);
    set_sell_card_registry(&sell_card_lookup);

    GameState {
        players,
        draft_deck,
        destroyed_pile: UnorderedCards::new(),
        sell_card_deck,
        sell_card_display,
        phase: GamePhase::Draw,
        round: 1,
        max_rounds: 6,
        ai_players: FixedVec::from_slice(ai_players),
        card_lookup,
        sell_card_lookup,
        draw_log: None,
        force_max_workshop: true,
    }
}
