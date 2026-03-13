use crate::cards::*;
use crate::fixed_vec::FixedVec;
use crate::types::*;
use crate::unordered_cards::{set_sell_card_registry, set_card_registry, UnorderedSellCards, UnorderedCards};
use rand::Rng;
use rand::seq::SliceRandom;
use smallvec::SmallVec;
use std::cell::Cell;

thread_local! {
    static NEXT_CARD_ID: Cell<u32> = const { Cell::new(0) };
    static NEXT_SELL_CARD_ID: Cell<u32> = const { Cell::new(0) };
    static NEXT_GLASS_ID: Cell<u32> = const { Cell::new(0) };
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

fn next_glass_id() -> u32 {
    NEXT_GLASS_ID.with(|c| {
        let id = c.get();
        c.set(id + 1);
        id
    })
}

fn reset_id_counters() {
    NEXT_CARD_ID.with(|c| c.set(0));
    NEXT_SELL_CARD_ID.with(|c| c.set(0));
    NEXT_GLASS_ID.with(|c| c.set(0));
}

pub fn create_initial_game_state<R: Rng>(num_players: usize, ai_players: &[bool], rng: &mut R) -> GameState {
    create_initial_game_state_with_expansions(num_players, ai_players, Expansions::default(), rng)
}

pub fn create_initial_game_state_with_expansions<R: Rng>(num_players: usize, ai_players: &[bool], expansions: Expansions, rng: &mut R) -> GameState {
    reset_id_counters();

    let mut card_lookup = [Card::BasicRed; 256];
    let mut sell_card_lookup = [SellCard::Textiles2Vermilion; 256];

    // Build each player's starting state
    let players: FixedVec<PlayerState, MAX_PLAYERS> = (0..num_players)
        .map(|_| {
            let personal_cards = [
                Card::BasicRed, Card::BasicYellow, Card::BasicBlue,
                Card::StarterCeramics, Card::StarterPaintings, Card::StarterTextiles,
                Card::Chalk,
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
                workshopped_cards: UnorderedCards::new(),
                workshop_cards: UnorderedCards::new(),
                drafted_cards: UnorderedCards::new(),
                color_wheel,
                materials: Materials::new(),
                completed_sell_cards: SmallVec::new(),
                completed_glass: SmallVec::new(),
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

    // Build sell card deck
    let mut sell_card_deck = UnorderedSellCards::new();
    for &sell_card in &generate_all_sell_cards() {
        let id = next_sell_card_id();
        sell_card_lookup[id as usize] = sell_card;
        sell_card_deck.insert(id);
    }

    // Deal 6 sell cards from sell_card_deck to sell_card_display
    let mut sell_card_display: FixedVec<SellCardInstance, MAX_SELL_CARD_DISPLAY> = FixedVec::new();
    let drawn_sell_cards = sell_card_deck.draw_multiple(6, rng);
    for id in drawn_sell_cards.iter() {
        sell_card_display.push(SellCardInstance {
            instance_id: id as u32,
            sell_card: sell_card_lookup[id as usize],
        });
    }

    // Build glass deck and display
    let mut glass_deck: SmallVec<[GlassInstance; 11]> = SmallVec::new();
    let mut glass_display: FixedVec<GlassInstance, MAX_GLASS_DISPLAY> = FixedVec::new();

    if expansions.glass {
        let mut glass_cards: Vec<GlassInstance> = generate_all_glass()
            .iter()
            .map(|&glass| GlassInstance {
                instance_id: next_glass_id(),
                glass,
            })
            .collect();
        glass_cards.shuffle(rng);
        for gi in glass_cards {
            if glass_display.len() < MAX_GLASS_DISPLAY {
                glass_display.push(gi);
            } else {
                glass_deck.push(gi);
            }
        }
    }

    set_card_registry(&card_lookup);
    set_sell_card_registry(&sell_card_lookup);

    GameState {
        players,
        draft_deck,
        destroyed_pile: UnorderedCards::new(),
        sell_card_deck,
        sell_card_display,
        expansions,
        glass_deck,
        glass_display,
        phase: GamePhase::Draw,
        round: 1,
        ai_players: FixedVec::from_slice(ai_players),
        card_lookup,
        sell_card_lookup,
    }
}
