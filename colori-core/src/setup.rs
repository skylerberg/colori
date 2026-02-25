use crate::cards::*;
use crate::deck_utils::shuffle_in_place;
use crate::types::*;
use rand::Rng;

fn create_card_instances(cards: &[Card], next_id: &mut u32) -> Vec<CardInstance> {
    cards
        .iter()
        .map(|&card| {
            let id = *next_id;
            *next_id += 1;
            CardInstance {
                instance_id: id,
                card,
            }
        })
        .collect()
}

fn create_buyer_instances(buyers: &[BuyerCard], next_id: &mut u32) -> Vec<BuyerInstance> {
    buyers
        .iter()
        .map(|&buyer| {
            let id = *next_id;
            *next_id += 1;
            BuyerInstance {
                instance_id: id,
                buyer,
            }
        })
        .collect()
}

pub fn create_initial_game_state<R: Rng>(player_names: &[String], ai_players: u8, rng: &mut R) -> GameState {
    let mut next_id: u32 = 1;

    // Build each player's starting state
    let mut players: Vec<PlayerState> = Vec::with_capacity(player_names.len());
    for name in player_names {
        let mut personal_cards: Vec<Card> = Vec::with_capacity(7);
        personal_cards.extend_from_slice(&basic_dye_cards());
        personal_cards.extend_from_slice(&starter_material_cards());
        personal_cards.push(chalk_card());

        let mut deck = create_card_instances(&personal_cards, &mut next_id);
        shuffle_in_place(&mut deck, rng);

        let mut color_wheel = ColorWheel::new();
        color_wheel.set(Color::Red, 1);
        color_wheel.set(Color::Yellow, 1);
        color_wheel.set(Color::Blue, 1);

        players.push(PlayerState {
            name: name.clone(),
            deck,
            discard: Vec::new(),
            workshop_cards: Vec::new(),
            drafted_cards: Vec::new(),
            color_wheel,
            materials: Materials::new(),
            completed_buyers: Vec::new(),
            ducats: 0,
        });
    }

    // Build draft deck: 4x each of 15 dye cards + 1x each of 15 draft materials + 3x each of 5 action cards = 90 cards
    let mut draft_cards: Vec<Card> = Vec::with_capacity(90);

    // 4 copies of each of 15 dye cards (60 total)
    for dye in dye_cards() {
        for _ in 0..4 {
            draft_cards.push(dye);
        }
    }

    // 1 copy of each of 15 draft material cards (15 total)
    draft_cards.extend_from_slice(&draft_material_cards());

    // 3 copies of each of 5 action cards (15 total)
    for action in action_cards() {
        for _ in 0..3 {
            draft_cards.push(action);
        }
    }

    let mut draft_deck = create_card_instances(&draft_cards, &mut next_id);
    shuffle_in_place(&mut draft_deck, rng);

    // Build buyer deck: all 51 buyers shuffled
    let mut buyer_deck = create_buyer_instances(&generate_all_buyers(), &mut next_id);
    shuffle_in_place(&mut buyer_deck, rng);

    // Deal 6 buyers from buyer_deck to buyer_display (pop from end)
    let mut buyer_display: Vec<BuyerInstance> = Vec::with_capacity(6);
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
        ai_players,
    }
}
