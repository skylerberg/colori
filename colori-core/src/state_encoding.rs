use crate::colori_game::enumerate_choices;
use crate::types::*;

pub const STATE_ENCODING_SIZE: usize = 768;
pub const ACTION_ENCODING_SIZE: usize = 86;

const NUM_CARD_TYPES: usize = 42;
const NUM_BUYER_TYPES: usize = 54;
const NUM_BUYER_CATEGORIES: usize = 18;
const NUM_PENDING_CHOICE_TYPES: usize = 7;
const NUM_CHOICE_TYPES: usize = 14;

const FEATURES_PER_PLAYER: usize = 199;
const MAX_ENCODED_PLAYERS: usize = 3;
const PLAYER_SECTION_SIZE: usize = FEATURES_PER_PLAYER * MAX_ENCODED_PLAYERS; // 597

/// Encode the full game state from the perspective of `perspective_player`.
///
/// Returns a 768-float vector. Players are rotated so that the perspective
/// player occupies slot 0. If there are fewer than 3 other players, remaining
/// slots are filled with zeros.
pub fn encode_state(state: &GameState, perspective_player: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; STATE_ENCODING_SIZE];
    let num_players = state.players.len();

    // Encode per-player features (rotated)
    for slot in 0..MAX_ENCODED_PLAYERS {
        let actual_player = (perspective_player + slot) % num_players;
        if slot >= num_players {
            // Leave zeros for missing players
            break;
        }
        let offset = slot * FEATURES_PER_PLAYER;
        encode_player(&state.players[actual_player], state, &mut out[offset..]);
    }

    // Encode global features
    let global_offset = PLAYER_SECTION_SIZE;
    encode_global(state, perspective_player, &mut out[global_offset..]);

    out
}

fn encode_player(player: &PlayerState, state: &GameState, buf: &mut [f32]) {
    let mut idx = 0;

    // Color wheel: 12 floats (raw counts / 5.0)
    for i in 0..12 {
        buf[idx] = player.color_wheel.counts[i] as f32 / 5.0;
        idx += 1;
    }

    // Materials: 3 floats (/ 10.0)
    for i in 0..3 {
        buf[idx] = player.materials.counts[i] as f32 / 10.0;
        idx += 1;
    }

    // Ducats: 1 float (/ 15.0)
    buf[idx] = player.ducats as f32 / 15.0;
    idx += 1;

    // Score: 1 float (/ 15.0)
    buf[idx] = player.cached_score as f32 / 15.0;
    idx += 1;

    // Completed buyers: 54 floats (one-hot per BuyerCard variant, sum counts if duplicates)
    for buyer_inst in player.completed_buyers.iter() {
        let buyer_type_idx = buyer_inst.buyer as usize;
        buf[idx + buyer_type_idx] += 1.0;
    }
    idx += NUM_BUYER_TYPES;

    // Workshop cards: 42 floats (count per Card variant)
    for id in player.workshop_cards.iter() {
        let card = state.card_lookup[id as usize];
        let card_type_idx = card as usize;
        buf[idx + card_type_idx] += 1.0;
    }
    idx += NUM_CARD_TYPES;

    // Drafted cards: 42 floats (count per Card variant)
    for id in player.drafted_cards.iter() {
        let card = state.card_lookup[id as usize];
        let card_type_idx = card as usize;
        buf[idx + card_type_idx] += 1.0;
    }
    idx += NUM_CARD_TYPES;

    // Used cards: 42 floats (count per Card variant)
    for id in player.used_cards.iter() {
        let card = state.card_lookup[id as usize];
        let card_type_idx = card as usize;
        buf[idx + card_type_idx] += 1.0;
    }
    idx += NUM_CARD_TYPES;

    // Deck size: 1 float (/ 20.0)
    buf[idx] = player.deck.len() as f32 / 20.0;
    idx += 1;

    // Discard size: 1 float (/ 20.0)
    buf[idx] = player.discard.len() as f32 / 20.0;
    // idx += 1; // last field
}

fn encode_global(state: &GameState, perspective_player: usize, buf: &mut [f32]) {
    let mut idx = 0;
    let num_players = state.players.len();

    // Buyer display: 54 floats (one-hot by BuyerCard variant, summed)
    for buyer_inst in state.buyer_display.iter() {
        let buyer_type_idx = buyer_inst.buyer as usize;
        buf[idx + buyer_type_idx] += 1.0;
    }
    idx += NUM_BUYER_TYPES;

    // Round: 1 float (/ 10.0)
    buf[idx] = state.round as f32 / 10.0;
    idx += 1;

    // Phase: 4 floats (one-hot: Draw/Draft/Action/Cleanup - GameOver maps to 0s)
    match &state.phase {
        GamePhase::Draw => buf[idx] = 1.0,
        GamePhase::Draft { .. } => buf[idx + 1] = 1.0,
        GamePhase::Action { .. } => buf[idx + 2] = 1.0,
        GamePhase::Cleanup { .. } => buf[idx + 3] = 1.0,
        GamePhase::GameOver => {} // all zeros
    }
    idx += 4;

    // Num players: 3 floats (one-hot: 2/3/4)
    match num_players {
        2 => buf[idx] = 1.0,
        3 => buf[idx + 1] = 1.0,
        4 => buf[idx + 2] = 1.0,
        _ => {}
    }
    idx += 3;

    // Draft hand (own): 42 floats (card type counts of perspective player's current
    // draft hand, zeros if not in draft)
    if let GamePhase::Draft { ref draft_state } = state.phase {
        let hand = draft_state.hands[perspective_player];
        for id in hand.iter() {
            let card = state.card_lookup[id as usize];
            let card_type_idx = card as usize;
            buf[idx + card_type_idx] += 1.0;
        }
    }
    idx += NUM_CARD_TYPES;

    // Draft pick number: 1 float (pick_number / 5.0, 0 if not in draft)
    if let GamePhase::Draft { ref draft_state } = state.phase {
        buf[idx] = draft_state.pick_number as f32 / 5.0;
    }
    idx += 1;

    // Draft direction: 1 float (direction as f32, 0 if not in draft)
    if let GamePhase::Draft { ref draft_state } = state.phase {
        buf[idx] = draft_state.direction as f32;
    }
    idx += 1;

    // Pending choice type: 7 floats (one-hot for each PendingChoice variant)
    if let GamePhase::Action { ref action_state } = state.phase {
        if let Some(ref pending) = action_state.pending_choice {
            match pending {
                PendingChoice::ChooseCardsForWorkshop { .. } => buf[idx] = 1.0,
                PendingChoice::ChooseCardsToDestroy { .. } => buf[idx + 1] = 1.0,
                PendingChoice::ChooseMix { .. } => buf[idx + 2] = 1.0,
                PendingChoice::ChooseBuyer => buf[idx + 3] = 1.0,
                PendingChoice::ChooseSecondaryColor => buf[idx + 4] = 1.0,
                PendingChoice::ChoosePrimaryColor => buf[idx + 5] = 1.0,
                PendingChoice::ChooseTertiaryToLose => buf[idx + 6] = 1.0,
            }
        }
    }
    idx += NUM_PENDING_CHOICE_TYPES;

    // Ability stack summary: 9 floats (count per Ability type in the stack)
    if let GamePhase::Action { ref action_state } = state.phase {
        for ability in action_state.ability_stack.iter() {
            let ability_idx = match ability {
                Ability::Workshop { .. } => 0,
                Ability::DrawCards { .. } => 1,
                Ability::MixColors { .. } => 2,
                Ability::DestroyCards { .. } => 3,
                Ability::Sell => 4,
                Ability::GainDucats { .. } => 5,
                Ability::GainSecondary => 6,
                Ability::GainPrimary => 7,
                Ability::ChangeTertiary => 8,
            };
            buf[idx + ability_idx] += 1.0;
        }
    }
    // idx += NUM_ABILITY_TYPES; // last section
}

/// Encode a single action (choice) into a fixed-size feature vector.
///
/// Returns an 86-float vector.
pub fn encode_action(choice: &ColoriChoice, state: &GameState) -> Vec<f32> {
    let mut out = vec![0.0f32; ACTION_ENCODING_SIZE];
    let mut idx = 0;

    // Choice type: 14 floats (one-hot for all 14 ColoriChoice variants)
    let choice_type_idx = match choice {
        ColoriChoice::DraftPick { .. } => 0,
        ColoriChoice::DestroyDraftedCard { .. } => 1,
        ColoriChoice::EndTurn => 2,
        ColoriChoice::Workshop { .. } => 3,
        ColoriChoice::SkipWorkshop => 4,
        ColoriChoice::DestroyDrawnCards { .. } => 5,
        ColoriChoice::SelectBuyer { .. } => 6,
        ColoriChoice::GainSecondary { .. } => 7,
        ColoriChoice::GainPrimary { .. } => 8,
        ColoriChoice::MixAll { .. } => 9,
        ColoriChoice::SwapTertiary { .. } => 10,
        ColoriChoice::DestroyAndMixAll { .. } => 11,
        ColoriChoice::DestroyAndSell { .. } => 12,
        ColoriChoice::KeepWorkshopCards { .. } => 13,
    };
    out[idx + choice_type_idx] = 1.0;
    idx += NUM_CHOICE_TYPES;

    // Card type involved: 42 floats
    match choice {
        ColoriChoice::DraftPick { card_instance_id }
        | ColoriChoice::DestroyDraftedCard { card_instance_id }
        | ColoriChoice::DestroyAndMixAll { card_instance_id, .. }
        | ColoriChoice::DestroyAndSell { card_instance_id, .. } => {
            let card = state.card_lookup[*card_instance_id as usize];
            out[idx + card as usize] = 1.0;
        }
        ColoriChoice::Workshop { card_instance_ids }
        | ColoriChoice::DestroyDrawnCards { card_instance_ids }
        | ColoriChoice::KeepWorkshopCards { card_instance_ids } => {
            for id in card_instance_ids.iter() {
                let card = state.card_lookup[id as usize];
                out[idx + card as usize] += 1.0;
            }
        }
        _ => {}
    }
    idx += NUM_CARD_TYPES;

    // Buyer type involved: 18 floats (one-hot by buyer category = buyer_index / 3)
    match choice {
        ColoriChoice::SelectBuyer { buyer_instance_id } => {
            let buyer_card = state.buyer_lookup[*buyer_instance_id as usize];
            let category = buyer_card as usize / 3;
            out[idx + category] = 1.0;
        }
        ColoriChoice::DestroyAndSell { buyer_instance_id, .. } => {
            let buyer_card = state.buyer_lookup[*buyer_instance_id as usize];
            let category = buyer_card as usize / 3;
            out[idx + category] = 1.0;
        }
        _ => {}
    }
    idx += NUM_BUYER_CATEGORIES;

    // Colors involved: 12 floats
    match choice {
        ColoriChoice::GainSecondary { color } | ColoriChoice::GainPrimary { color } => {
            out[idx + color.index()] = 1.0;
        }
        ColoriChoice::SwapTertiary { lose, gain } => {
            out[idx + lose.index()] = 1.0;
            out[idx + gain.index()] = 1.0;
        }
        ColoriChoice::MixAll { mixes } | ColoriChoice::DestroyAndMixAll { mixes, .. } => {
            for &(a, b) in mixes.iter() {
                out[idx + a.index()] = 1.0;
                out[idx + b.index()] = 1.0;
            }
        }
        _ => {}
    }
    // idx += NUM_COLORS; // last section

    out
}

/// Encode all legal actions for the current game state.
///
/// Uses `enumerate_choices` to get all legal actions, then encodes each one.
pub fn encode_legal_actions(state: &GameState) -> Vec<Vec<f32>> {
    let choices = enumerate_choices(state);
    choices.iter().map(|c| encode_action(c, state)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw_phase::execute_draw_phase;
    use crate::setup::create_initial_game_state;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    fn make_test_state() -> GameState {
        let mut rng = SmallRng::seed_from_u64(42);
        let mut state = create_initial_game_state(3, &[true, true, true], &mut rng);
        execute_draw_phase(&mut state, &mut rng);
        state
    }

    #[test]
    fn test_encode_state_dimensions() {
        let state = make_test_state();
        let encoded = encode_state(&state, 0);
        assert_eq!(encoded.len(), STATE_ENCODING_SIZE);
        assert_eq!(encoded.len(), 768);
    }

    #[test]
    fn test_encode_state_deterministic() {
        let state = make_test_state();
        let encoded1 = encode_state(&state, 0);
        let encoded2 = encode_state(&state, 0);
        assert_eq!(encoded1, encoded2);
    }

    #[test]
    fn test_encode_state_player_rotation() {
        let state = make_test_state();

        let encoded_p0 = encode_state(&state, 0);
        let encoded_p1 = encode_state(&state, 1);

        // The encodings should differ because different players are in slot 0
        assert_ne!(encoded_p0, encoded_p1);

        // Player 0's features when perspective=0 should match player 1's
        // features when perspective=1 (both are slot 0 = the perspective player).
        // But the actual player data differs, so just verify they are not
        // trivially equal (rotation is applied).
        // More specifically: slot 0 for perspective=0 is player 0,
        // and slot 0 for perspective=1 is player 1.
        let p0_slot0 = &encoded_p0[..FEATURES_PER_PLAYER];
        let p1_slot0 = &encoded_p1[..FEATURES_PER_PLAYER];

        // These encode different players, so they should differ
        // (unless players happen to have identical states, which is unlikely)
        assert_ne!(p0_slot0, p1_slot0);

        // Verify that slot 1 for perspective=0 encodes player 1,
        // which should match slot 0 for perspective=1 (both encode player 1's state).
        let p0_slot1 = &encoded_p0[FEATURES_PER_PLAYER..2 * FEATURES_PER_PLAYER];
        assert_eq!(p0_slot1, p1_slot0);
    }

    #[test]
    fn test_encode_action_dimensions() {
        let state = make_test_state();
        let choices = enumerate_choices(&state);
        assert!(!choices.is_empty(), "should have at least one legal action");

        for choice in &choices {
            let encoded = encode_action(choice, &state);
            assert_eq!(encoded.len(), ACTION_ENCODING_SIZE);
            assert_eq!(encoded.len(), 86);
        }
    }

    #[test]
    fn test_encode_legal_actions() {
        let state = make_test_state();
        let choices = enumerate_choices(&state);
        let encoded_actions = encode_legal_actions(&state);

        assert_eq!(encoded_actions.len(), choices.len());
        for encoded in &encoded_actions {
            assert_eq!(encoded.len(), ACTION_ENCODING_SIZE);
        }
    }
}
