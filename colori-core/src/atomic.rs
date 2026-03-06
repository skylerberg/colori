use crate::action_phase::{
    can_afford_buyer, destroy_drafted_card, end_player_turn, process_ability_stack,
    resolve_destroy_cards, resolve_gain_primary, resolve_gain_secondary, resolve_mix_colors,
    resolve_select_buyer, skip_mix, skip_workshop, resolve_choose_tertiary_to_lose,
    resolve_choose_tertiary_to_gain,
};
use crate::colors::{PRIMARIES, SECONDARIES, TERTIARIES, VALID_MIX_PAIRS};
use crate::draft_phase::player_pick;
use crate::draw_phase::execute_draw_phase;
use crate::types::*;
use crate::unordered_cards::UnorderedCards;
use rand::Rng;

pub const NUM_ATOMIC_ACTIONS: usize = 300;


// Index ranges:
// DraftPick(Card)          0..49
// DestroyDraftedCard(Card) 49..98
// EndTurn                  98
// SelectBuyer(BuyerCard)   99..153
// MixPair(Color, Color)    153..162
// SkipMix                  162
// WorkshopCard(Card)       163..212
// SkipWorkshop             212
// DestroyTarget(Card)      213..262
// SkipDestroy              262
// GainSecondary(Color)     263..266
// GainPrimary(Color)       266..269
// SwapTertiary(lose, gain) 269..299
// SkipSwap                 299

const DRAFT_PICK_START: usize = 0;
const DESTROY_DRAFTED_START: usize = 49;
const END_TURN_INDEX: usize = 98;
const SELECT_BUYER_START: usize = 99;
const MIX_PAIR_START: usize = 153;
const SKIP_MIX_INDEX: usize = 162;
const WORKSHOP_CARD_START: usize = 163;
const SKIP_WORKSHOP_INDEX: usize = 212;
const DESTROY_TARGET_START: usize = 213;
const SKIP_DESTROY_INDEX: usize = 262;
const GAIN_SECONDARY_START: usize = 263;
const GAIN_PRIMARY_START: usize = 266;
const SWAP_TERTIARY_START: usize = 269;
const SKIP_SWAP_INDEX: usize = 299;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AtomicChoice {
    DraftPick(Card),
    DestroyDraftedCard(Card),
    EndTurn,
    SelectBuyer(BuyerCard),
    MixPair(Color, Color),
    SkipMix,
    WorkshopCard(Card),
    SkipWorkshop,
    DestroyTarget(Card),
    SkipDestroy,
    GainSecondary(Color),
    GainPrimary(Color),
    SwapTertiary { lose: Color, gain: Color },
    SkipSwap,
}

const ALL_CARDS: [Card; 49] = [
    Card::BasicRed, Card::BasicYellow, Card::BasicBlue,
    Card::Kermes, Card::Weld, Card::Woad,
    Card::Lac, Card::Brazilwood, Card::Pomegranate,
    Card::Sumac, Card::Elderberry, Card::Turnsole,
    Card::Madder, Card::Turmeric, Card::DyersGreenweed,
    Card::Verdigris, Card::Orchil, Card::Logwood,
    Card::VermilionDye, Card::Saffron, Card::PersianBerries,
    Card::Azurite, Card::IndigoDye, Card::Cochineal,
    Card::StarterCeramics, Card::StarterPaintings, Card::StarterTextiles,
    Card::FineCeramics, Card::FinePaintings, Card::FineTextiles,
    Card::TerraCotta, Card::OchreWare, Card::CobaltWare,
    Card::CinnabarCanvas, Card::OrpimentCanvas, Card::UltramarineCanvas,
    Card::AlizarinFabric, Card::FusticFabric, Card::PastelFabric,
    Card::ClayCanvas, Card::ClayFabric, Card::CanvasFabric,
    Card::Alum, Card::CreamOfTartar, Card::GumArabic,
    Card::Potash, Card::Vinegar, Card::Argol, Card::Chalk,
];

const ALL_BUYERS: [BuyerCard; 54] = [
    BuyerCard::Textiles2Vermilion, BuyerCard::Textiles2Amber, BuyerCard::Textiles2Chartreuse,
    BuyerCard::Textiles2Teal, BuyerCard::Textiles2Indigo, BuyerCard::Textiles2Magenta,
    BuyerCard::Textiles2OrangeRed, BuyerCard::Textiles2OrangeYellow, BuyerCard::Textiles2OrangeBlue,
    BuyerCard::Textiles2GreenRed, BuyerCard::Textiles2GreenYellow, BuyerCard::Textiles2GreenBlue,
    BuyerCard::Textiles2PurpleRed, BuyerCard::Textiles2PurpleYellow, BuyerCard::Textiles2PurpleBlue,
    BuyerCard::Textiles2RedRedRed, BuyerCard::Textiles2YellowYellowYellow, BuyerCard::Textiles2BlueBlueBlue,
    BuyerCard::Ceramics3VermilionRed, BuyerCard::Ceramics3VermilionYellow, BuyerCard::Ceramics3VermilionBlue,
    BuyerCard::Ceramics3AmberRed, BuyerCard::Ceramics3AmberYellow, BuyerCard::Ceramics3AmberBlue,
    BuyerCard::Ceramics3ChartreuseRed, BuyerCard::Ceramics3ChartreuseYellow, BuyerCard::Ceramics3ChartreuseBlue,
    BuyerCard::Ceramics3TealRed, BuyerCard::Ceramics3TealYellow, BuyerCard::Ceramics3TealBlue,
    BuyerCard::Ceramics3IndigoRed, BuyerCard::Ceramics3IndigoYellow, BuyerCard::Ceramics3IndigoBlue,
    BuyerCard::Ceramics3MagentaRed, BuyerCard::Ceramics3MagentaYellow, BuyerCard::Ceramics3MagentaBlue,
    BuyerCard::Paintings4VermilionOrange, BuyerCard::Paintings4VermilionGreen, BuyerCard::Paintings4VermilionPurple,
    BuyerCard::Paintings4AmberOrange, BuyerCard::Paintings4AmberGreen, BuyerCard::Paintings4AmberPurple,
    BuyerCard::Paintings4ChartreuseOrange, BuyerCard::Paintings4ChartreuseGreen, BuyerCard::Paintings4ChartreusePurple,
    BuyerCard::Paintings4TealOrange, BuyerCard::Paintings4TealGreen, BuyerCard::Paintings4TealPurple,
    BuyerCard::Paintings4IndigoOrange, BuyerCard::Paintings4IndigoGreen, BuyerCard::Paintings4IndigoPurple,
    BuyerCard::Paintings4MagentaOrange, BuyerCard::Paintings4MagentaGreen, BuyerCard::Paintings4MagentaPurple,
];

pub fn atomic_choice_to_index(choice: &AtomicChoice) -> usize {
    match choice {
        AtomicChoice::DraftPick(card) => DRAFT_PICK_START + *card as usize,
        AtomicChoice::DestroyDraftedCard(card) => DESTROY_DRAFTED_START + *card as usize,
        AtomicChoice::EndTurn => END_TURN_INDEX,
        AtomicChoice::SelectBuyer(buyer) => SELECT_BUYER_START + *buyer as usize,
        AtomicChoice::MixPair(a, b) => {
            let pair_index = VALID_MIX_PAIRS.iter().position(|&(x, y)| x == *a && y == *b)
                .expect("Invalid mix pair");
            MIX_PAIR_START + pair_index
        }
        AtomicChoice::SkipMix => SKIP_MIX_INDEX,
        AtomicChoice::WorkshopCard(card) => WORKSHOP_CARD_START + *card as usize,
        AtomicChoice::SkipWorkshop => SKIP_WORKSHOP_INDEX,
        AtomicChoice::DestroyTarget(card) => DESTROY_TARGET_START + *card as usize,
        AtomicChoice::SkipDestroy => SKIP_DESTROY_INDEX,
        AtomicChoice::GainSecondary(color) => {
            let i = SECONDARIES.iter().position(|&c| c == *color).expect("Invalid secondary");
            GAIN_SECONDARY_START + i
        }
        AtomicChoice::GainPrimary(color) => {
            let i = PRIMARIES.iter().position(|&c| c == *color).expect("Invalid primary");
            GAIN_PRIMARY_START + i
        }
        AtomicChoice::SwapTertiary { lose, gain } => {
            let li = TERTIARIES.iter().position(|&c| c == *lose).expect("Invalid tertiary lose");
            let gi = TERTIARIES.iter().position(|&c| c == *gain).expect("Invalid tertiary gain");
            // Map (lose_idx, gain_idx) where gain_idx != lose_idx to a flat index.
            // For each lose_idx (0..6), there are 5 valid gain indices.
            let adjusted_gi = if gi < li { gi } else { gi - 1 };
            SWAP_TERTIARY_START + li * 5 + adjusted_gi
        }
        AtomicChoice::SkipSwap => SKIP_SWAP_INDEX,
    }
}

pub fn index_to_atomic_choice(index: usize) -> AtomicChoice {
    match index {
        i if i < DESTROY_DRAFTED_START => AtomicChoice::DraftPick(ALL_CARDS[i]),
        i if i < END_TURN_INDEX => AtomicChoice::DestroyDraftedCard(ALL_CARDS[i - DESTROY_DRAFTED_START]),
        END_TURN_INDEX => AtomicChoice::EndTurn,
        i if i < MIX_PAIR_START => AtomicChoice::SelectBuyer(ALL_BUYERS[i - SELECT_BUYER_START]),
        i if i < SKIP_MIX_INDEX => {
            let (a, b) = VALID_MIX_PAIRS[i - MIX_PAIR_START];
            AtomicChoice::MixPair(a, b)
        }
        SKIP_MIX_INDEX => AtomicChoice::SkipMix,
        i if i < SKIP_WORKSHOP_INDEX => AtomicChoice::WorkshopCard(ALL_CARDS[i - WORKSHOP_CARD_START]),
        SKIP_WORKSHOP_INDEX => AtomicChoice::SkipWorkshop,
        i if i < SKIP_DESTROY_INDEX => AtomicChoice::DestroyTarget(ALL_CARDS[i - DESTROY_TARGET_START]),
        SKIP_DESTROY_INDEX => AtomicChoice::SkipDestroy,
        i if i < GAIN_PRIMARY_START => AtomicChoice::GainSecondary(SECONDARIES[i - GAIN_SECONDARY_START]),
        i if i < SWAP_TERTIARY_START => AtomicChoice::GainPrimary(PRIMARIES[i - GAIN_PRIMARY_START]),
        i if i < SKIP_SWAP_INDEX => {
            let flat = i - SWAP_TERTIARY_START;
            let li = flat / 5;
            let adjusted_gi = flat % 5;
            let gi = if adjusted_gi < li { adjusted_gi } else { adjusted_gi + 1 };
            AtomicChoice::SwapTertiary {
                lose: TERTIARIES[li],
                gain: TERTIARIES[gi],
            }
        }
        SKIP_SWAP_INDEX => AtomicChoice::SkipSwap,
        _ => panic!("Invalid atomic action index: {}", index),
    }
}

pub fn enumerate_atomic_choices(state: &GameState) -> Vec<AtomicChoice> {
    let mut choices = Vec::new();
    match &state.phase {
        GamePhase::Draft { draft_state } => {
            let hand = draft_state.hands[draft_state.current_player_index];
            let mut seen: u64 = 0;
            for id in hand.iter() {
                let card = state.card_lookup[id as usize];
                let bit = 1u64 << (card as u64);
                if seen & bit != 0 { continue; }
                seen |= bit;
                choices.push(AtomicChoice::DraftPick(card));
            }
        }
        GamePhase::Action { action_state } => {
            let player = &state.players[action_state.current_player_index];
            match action_state.ability_stack.last() {
                None => {
                    // Can destroy drafted cards or end turn
                    let mut seen: u64 = 0;
                    for id in player.drafted_cards.iter() {
                        let card = state.card_lookup[id as usize];
                        let bit = 1u64 << (card as u64);
                        if seen & bit != 0 { continue; }
                        seen |= bit;
                        choices.push(AtomicChoice::DestroyDraftedCard(card));
                    }
                    choices.push(AtomicChoice::EndTurn);
                }
                Some(Ability::Workshop { .. }) => {
                    choices.push(AtomicChoice::SkipWorkshop);
                    let mut seen: u64 = 0;
                    for id in player.workshop_cards.iter() {
                        let card = state.card_lookup[id as usize];
                        let bit = 1u64 << (card as u64);
                        if seen & bit != 0 { continue; }
                        seen |= bit;
                        choices.push(AtomicChoice::WorkshopCard(card));
                    }
                }
                Some(Ability::MixColors { .. }) => {
                    choices.push(AtomicChoice::SkipMix);
                    for &(a, b) in &VALID_MIX_PAIRS {
                        if player.color_wheel.get(a) > 0 && player.color_wheel.get(b) > 0 {
                            choices.push(AtomicChoice::MixPair(a, b));
                        }
                    }
                }
                Some(Ability::DestroyCards) => {
                    choices.push(AtomicChoice::SkipDestroy);
                    let mut seen: u64 = 0;
                    for id in player.workshop_cards.iter() {
                        let card = state.card_lookup[id as usize];
                        let bit = 1u64 << (card as u64);
                        if seen & bit != 0 { continue; }
                        seen |= bit;
                        choices.push(AtomicChoice::DestroyTarget(card));
                    }
                }
                Some(Ability::Sell) => {
                    for buyer in state.buyer_display.iter() {
                        if can_afford_buyer(player, &buyer.buyer) {
                            choices.push(AtomicChoice::SelectBuyer(buyer.buyer));
                        }
                    }
                }
                Some(Ability::GainSecondary) => {
                    for &c in SECONDARIES.iter() {
                        choices.push(AtomicChoice::GainSecondary(c));
                    }
                }
                Some(Ability::GainPrimary) => {
                    for &c in PRIMARIES.iter() {
                        choices.push(AtomicChoice::GainPrimary(c));
                    }
                }
                Some(Ability::ChangeTertiary) => {
                    let has_any = TERTIARIES.iter().any(|&c| player.color_wheel.get(c) > 0);
                    if has_any {
                        choices.push(AtomicChoice::SkipSwap);
                        for &lose in TERTIARIES.iter() {
                            if player.color_wheel.get(lose) > 0 {
                                for &gain in TERTIARIES.iter() {
                                    if gain != lose {
                                        choices.push(AtomicChoice::SwapTertiary { lose, gain });
                                    }
                                }
                            }
                        }
                    }
                }
                // Instant abilities (DrawCards, GainDucats) should never be on top
                Some(_) => {}
            }
        }
        _ => {}
    }
    choices
}

pub fn enumerate_atomic_legal_mask(state: &GameState) -> [bool; NUM_ATOMIC_ACTIONS] {
    let mut mask = [false; NUM_ATOMIC_ACTIONS];
    for choice in enumerate_atomic_choices(state) {
        mask[atomic_choice_to_index(&choice)] = true;
    }
    mask
}

/// Find the first card instance ID matching a card type in a card set.
fn find_card_instance(state: &GameState, card: Card, cards: &UnorderedCards) -> u8 {
    for id in cards.iter() {
        if state.card_lookup[id as usize] == card {
            return id;
        }
    }
    panic!("Card type {:?} not found in card set", card);
}

pub fn apply_atomic_choice<R: Rng>(state: &mut GameState, choice: &AtomicChoice, rng: &mut R) {
    match choice {
        AtomicChoice::DraftPick(card) => {
            let hand = match &state.phase {
                GamePhase::Draft { draft_state } => {
                    draft_state.hands[draft_state.current_player_index]
                }
                _ => panic!("Expected draft phase"),
            };
            let card_instance_id = find_card_instance(state, *card, &hand) as u32;
            player_pick(state, card_instance_id);
        }
        AtomicChoice::DestroyDraftedCard(card) => {
            let drafted = match &state.phase {
                GamePhase::Action { action_state } => {
                    state.players[action_state.current_player_index].drafted_cards
                }
                _ => panic!("Expected action phase"),
            };
            let card_instance_id = find_card_instance(state, *card, &drafted) as u32;
            destroy_drafted_card(state, card_instance_id, rng);
        }
        AtomicChoice::EndTurn => {
            end_player_turn(state, rng);
            // Auto-advance draw phase
            if matches!(state.phase, GamePhase::Draw) {
                execute_draw_phase(state, rng);
            }
        }
        AtomicChoice::SelectBuyer(buyer) => {
            let buyer_instance_id = state.buyer_display.iter()
                .find(|b| b.buyer == *buyer)
                .expect("Buyer not found in display")
                .instance_id;
            resolve_select_buyer(state, buyer_instance_id, rng);
        }
        AtomicChoice::MixPair(a, b) => {
            resolve_mix_colors(state, *a, *b, rng);
        }
        AtomicChoice::SkipMix => {
            skip_mix(state, rng);
        }
        AtomicChoice::WorkshopCard(card) => {
            let player_index = match &state.phase {
                GamePhase::Action { action_state } => action_state.current_player_index,
                _ => panic!("Expected action phase"),
            };
            let workshop = state.players[player_index].workshop_cards;
            let id = find_card_instance(state, *card, &workshop);

            // Resolve single-card workshop: extract materials/colors, handle action cards
            let card_obj = state.card_lookup[id as usize];
            state.players[player_index].workshop_cards.remove(id);

            if card_obj.is_action() {
                // Action card: move to workshopped, push workshop abilities
                state.players[player_index].workshopped_cards.insert(id);
                let abilities: Vec<Ability> = card_obj.workshop_abilities().to_vec();
                // Decrement workshop count
                let action_state = match &mut state.phase {
                    GamePhase::Action { action_state } => action_state,
                    _ => panic!("Expected action phase"),
                };
                match action_state.ability_stack.last_mut() {
                    Some(Ability::Workshop { count }) => {
                        *count -= 1;
                        if *count == 0 {
                            action_state.ability_stack.pop();
                        }
                    }
                    _ => panic!("Expected workshop on stack"),
                }
                // Push action card abilities onto stack
                let action_state = match &mut state.phase {
                    GamePhase::Action { action_state } => action_state,
                    _ => unreachable!(),
                };
                for &ability in abilities.iter() {
                    action_state.ability_stack.push(ability);
                }
            } else {
                // Non-action card: extract materials and colors
                let player = &mut state.players[player_index];
                for mt in card_obj.material_types() {
                    player.materials.increment(*mt);
                }
                for pip in card_obj.pips() {
                    player.color_wheel.increment(*pip);
                }
                player.workshopped_cards.insert(id);

                // Decrement workshop count
                let action_state = match &mut state.phase {
                    GamePhase::Action { action_state } => action_state,
                    _ => panic!("Expected action phase"),
                };
                match action_state.ability_stack.last_mut() {
                    Some(Ability::Workshop { count }) => {
                        *count -= 1;
                        if *count == 0 {
                            action_state.ability_stack.pop();
                        }
                    }
                    _ => panic!("Expected workshop on stack"),
                }
            }
            process_ability_stack(state, rng);
        }
        AtomicChoice::SkipWorkshop => {
            skip_workshop(state, rng);
        }
        AtomicChoice::DestroyTarget(card) => {
            let player_index = match &state.phase {
                GamePhase::Action { action_state } => action_state.current_player_index,
                _ => panic!("Expected action phase"),
            };
            let workshop = state.players[player_index].workshop_cards;
            let id = find_card_instance(state, *card, &workshop);
            let mut selected = UnorderedCards::new();
            selected.insert(id);
            resolve_destroy_cards(state, selected, rng);
        }
        AtomicChoice::SkipDestroy => {
            resolve_destroy_cards(state, UnorderedCards::new(), rng);
        }
        AtomicChoice::GainSecondary(color) => {
            resolve_gain_secondary(state, *color, rng);
        }
        AtomicChoice::GainPrimary(color) => {
            resolve_gain_primary(state, *color, rng);
        }
        AtomicChoice::SwapTertiary { lose, gain } => {
            resolve_choose_tertiary_to_lose(state, *lose);
            resolve_choose_tertiary_to_gain(state, *gain, rng);
        }
        AtomicChoice::SkipSwap => {
            // Pop ChangeTertiary without doing anything
            let action_state = match &mut state.phase {
                GamePhase::Action { action_state } => action_state,
                _ => panic!("Expected action phase"),
            };
            action_state.ability_stack.pop();
            process_ability_stack(state, rng);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw_phase::execute_draw_phase;
    use crate::setup::create_initial_game_state;
    use rand::{RngExt, SeedableRng};
    use wyrand::WyRand;

    #[test]
    fn test_index_roundtrip() {
        for i in 0..NUM_ATOMIC_ACTIONS {
            let choice = index_to_atomic_choice(i);
            let j = atomic_choice_to_index(&choice);
            assert_eq!(i, j, "Roundtrip failed for index {}: {:?} -> {}", i, choice, j);
        }
    }

    #[test]
    fn test_random_games_with_atomic_choices() {
        for seed in 0..100 {
            let mut rng = WyRand::seed_from_u64(seed);
            let mut state = create_initial_game_state(2, &[true, true], &mut rng);
            execute_draw_phase(&mut state, &mut rng);

            let mut steps = 0;
            loop {
                if matches!(state.phase, GamePhase::GameOver) {
                    break;
                }
                let choices = enumerate_atomic_choices(&state);
                assert!(!choices.is_empty(), "No choices available at step {} seed {}, phase: {:?}", steps, seed, state.phase);
                let idx = rng.random_range(0..choices.len());
                apply_atomic_choice(&mut state, &choices[idx], &mut rng);
                steps += 1;
                assert!(steps < 10000, "Game did not terminate after 10000 steps, seed {}", seed);
            }
        }
    }

    #[test]
    fn test_legal_mask_matches_enumerate() {
        for seed in 0..20 {
            let mut rng = WyRand::seed_from_u64(seed);
            let mut state = create_initial_game_state(2, &[true, true], &mut rng);
            execute_draw_phase(&mut state, &mut rng);

            for _ in 0..50 {
                if matches!(state.phase, GamePhase::GameOver) {
                    break;
                }
                let choices = enumerate_atomic_choices(&state);
                let mask = enumerate_atomic_legal_mask(&state);

                let mask_count = mask.iter().filter(|&&b| b).count();
                assert_eq!(mask_count, choices.len(),
                    "Mask count {} != choices count {} at seed {}", mask_count, choices.len(), seed);

                for choice in &choices {
                    let idx = atomic_choice_to_index(choice);
                    assert!(mask[idx], "Choice {:?} not in mask at seed {}", choice, seed);
                }

                let idx = rng.random_range(0..choices.len());
                apply_atomic_choice(&mut state, &choices[idx], &mut rng);
            }
        }
    }

    #[test]
    fn test_three_player_games() {
        for seed in 0..50 {
            let mut rng = WyRand::seed_from_u64(seed + 1000);
            let mut state = create_initial_game_state(3, &[true, true, true], &mut rng);
            execute_draw_phase(&mut state, &mut rng);

            let mut steps = 0;
            loop {
                if matches!(state.phase, GamePhase::GameOver) {
                    break;
                }
                let choices = enumerate_atomic_choices(&state);
                assert!(!choices.is_empty(), "No choices at step {} seed {}", steps, seed);
                let idx = rng.random_range(0..choices.len());
                apply_atomic_choice(&mut state, &choices[idx], &mut rng);
                steps += 1;
                assert!(steps < 10000, "Game did not terminate, seed {}", seed);
            }
        }
    }

    #[test]
    fn test_four_player_games() {
        for seed in 0..20 {
            let mut rng = WyRand::seed_from_u64(seed + 2000);
            let mut state = create_initial_game_state(4, &[true, true, true, true], &mut rng);
            execute_draw_phase(&mut state, &mut rng);

            let mut steps = 0;
            loop {
                if matches!(state.phase, GamePhase::GameOver) {
                    break;
                }
                let choices = enumerate_atomic_choices(&state);
                assert!(!choices.is_empty());
                let idx = rng.random_range(0..choices.len());
                apply_atomic_choice(&mut state, &choices[idx], &mut rng);
                steps += 1;
                assert!(steps < 10000);
            }
        }
    }
}
