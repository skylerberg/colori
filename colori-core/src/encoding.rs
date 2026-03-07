use crate::atomic::{enumerate_atomic_legal_mask, NUM_ATOMIC_ACTIONS};
use crate::types::*;

pub const OBS_SIZE: usize = 1109;

/// Encode the game state from a specific player's perspective into a fixed-size f32 buffer.
///
/// Layout (1109 floats):
/// Current player block (301):
///   - Cards by type in deck: 46
///   - Cards by type in discard: 46
///   - Cards by type in drafted: 46
///   - Cards by type in workshop: 46
///   - Cards by type in workshopped: 46
///   - Color wheel: 12
///   - Materials: 3
///   - Ducats: 1
///   - Score: 1
///   - Completed buyers by type: 54
///
/// Per opponent block (x3, zero-padded, 211 each = 633):
///   - Score: 1
///   - Completed buyers by type: 54
///   - Materials: 3
///   - Color wheel: 12
///   - Ducats: 1
///   - Workshop cards by type: 46
///   - Drafted cards by type: 46
///   - Workshopped cards by type: 46
///   - Deck size: 1
///   - Discard size: 1
///
/// Shared block (175):
///   - Buyer display by type: 54 (binary)
///   - Round: 1 (normalized to 0..1)
///   - Phase: 4 (one-hot)
///   - Pick number: 1
///   - Ability stack top: 10 (one-hot for 9 types + empty; count as value)
///   - Ability stack second: 10 (same encoding as top)
///   - Stack depth: 1
///   - Draft hand by type: 46
///   - Draft deck size: 1
///   - Destroyed pile by card type: 46
///   - Buyer deck size: 1
pub fn encode_observation(state: &GameState, player_index: usize, buffer: &mut [f32; OBS_SIZE]) {
    buffer.fill(0.0);
    let mut offset = 0;

    // --- Current player block (301) ---
    let player = &state.players[player_index];

    // deck by card type (46)
    write_card_type_counts(player.deck, &state.card_lookup, &mut buffer[offset..offset + 46]);
    offset += 46;

    // discard by card type (46)
    write_card_type_counts(player.discard, &state.card_lookup, &mut buffer[offset..offset + 46]);
    offset += 46;

    // drafted by card type (46)
    write_card_type_counts(player.drafted_cards, &state.card_lookup, &mut buffer[offset..offset + 46]);
    offset += 46;

    // workshop by card type (46)
    write_card_type_counts(player.workshop_cards, &state.card_lookup, &mut buffer[offset..offset + 46]);
    offset += 46;

    // workshopped by card type (46)
    write_card_type_counts(player.workshopped_cards, &state.card_lookup, &mut buffer[offset..offset + 46]);
    offset += 46;

    // color wheel (12)
    for i in 0..12 {
        buffer[offset + i] = player.color_wheel.counts[i] as f32;
    }
    offset += 12;

    // materials (3)
    for i in 0..3 {
        buffer[offset + i] = player.materials.counts[i] as f32;
    }
    offset += 3;

    // ducats (1)
    buffer[offset] = player.ducats as f32;
    offset += 1;

    // score (1)
    buffer[offset] = player.cached_score as f32;
    offset += 1;

    // completed buyers by type (54)
    for bi in player.completed_buyers.iter() {
        buffer[offset + bi.buyer as usize] += 1.0;
    }
    offset += 54;

    // --- Opponent blocks (3 x 211 = 633) ---
    let num_players = state.players.len();
    for opp_slot in 0..3 {
        let opp_idx = (player_index + 1 + opp_slot) % num_players;
        if opp_slot >= num_players - 1 {
            // Zero-pad for missing opponents
            offset += 211;
            continue;
        }
        let opp = &state.players[opp_idx];

        // score (1)
        buffer[offset] = opp.cached_score as f32;
        offset += 1;

        // completed buyers by type (54)
        for bi in opp.completed_buyers.iter() {
            buffer[offset + bi.buyer as usize] += 1.0;
        }
        offset += 54;

        // materials (3)
        for i in 0..3 {
            buffer[offset + i] = opp.materials.counts[i] as f32;
        }
        offset += 3;

        // color wheel (12)
        for i in 0..12 {
            buffer[offset + i] = opp.color_wheel.counts[i] as f32;
        }
        offset += 12;

        // ducats (1)
        buffer[offset] = opp.ducats as f32;
        offset += 1;

        // workshop cards by type (46)
        write_card_type_counts(opp.workshop_cards, &state.card_lookup, &mut buffer[offset..offset + 46]);
        offset += 46;

        // drafted cards by type (46)
        write_card_type_counts(opp.drafted_cards, &state.card_lookup, &mut buffer[offset..offset + 46]);
        offset += 46;

        // workshopped cards by type (46)
        write_card_type_counts(opp.workshopped_cards, &state.card_lookup, &mut buffer[offset..offset + 46]);
        offset += 46;

        // deck size (1)
        buffer[offset] = opp.deck.len() as f32;
        offset += 1;

        // discard size (1)
        buffer[offset] = opp.discard.len() as f32;
        offset += 1;
    }

    // --- Shared block (116) ---

    // buyer display by type (54) - binary
    for bi in state.buyer_display.iter() {
        buffer[offset + bi.buyer as usize] = 1.0;
    }
    offset += 54;

    // round normalized (1)
    buffer[offset] = state.round as f32 / 20.0;
    offset += 1;

    // phase one-hot (4): Draw=0, Draft=1, Action=2, GameOver=3
    let phase_idx = match &state.phase {
        GamePhase::Draw => 0,
        GamePhase::Draft { .. } => 1,
        GamePhase::Action { .. } => 2,
        GamePhase::GameOver => 3,
    };
    buffer[offset + phase_idx] = 1.0;
    offset += 4;

    // pick number (1)
    if let GamePhase::Draft { ref draft_state } = state.phase {
        buffer[offset] = draft_state.pick_number as f32;
    }
    offset += 1;

    // ability stack top (10): one-hot for 9 ability types + empty
    // Index 0 = empty, 1-9 = ability types
    // For abilities with count, use count as the value instead of 1.0
    if let GamePhase::Action { ref action_state } = state.phase {
        match action_state.ability_stack.last() {
            None => buffer[offset] = 1.0,
            Some(Ability::Workshop { count }) => buffer[offset + 1] = *count as f32,
            Some(Ability::DrawCards { count }) => buffer[offset + 2] = *count as f32,
            Some(Ability::MixColors { count }) => buffer[offset + 3] = *count as f32,
            Some(Ability::DestroyCards) => buffer[offset + 4] = 1.0,
            Some(Ability::Sell) => buffer[offset + 5] = 1.0,
            Some(Ability::GainDucats { count }) => buffer[offset + 6] = *count as f32,
            Some(Ability::GainSecondary) => buffer[offset + 7] = 1.0,
            Some(Ability::GainPrimary) => buffer[offset + 8] = 1.0,
            Some(Ability::ChangeTertiary) => buffer[offset + 9] = 1.0,
        }
    } else {
        buffer[offset] = 1.0; // empty stack for non-action phases
    }
    offset += 10;

    // ability stack second (10): same encoding as top, for the second-to-top ability
    if let GamePhase::Action { ref action_state } = state.phase {
        if action_state.ability_stack.len() >= 2 {
            let second = &action_state.ability_stack[action_state.ability_stack.len() - 2];
            match second {
                Ability::Workshop { count } => buffer[offset + 1] = *count as f32,
                Ability::DrawCards { count } => buffer[offset + 2] = *count as f32,
                Ability::MixColors { count } => buffer[offset + 3] = *count as f32,
                Ability::DestroyCards => buffer[offset + 4] = 1.0,
                Ability::Sell => buffer[offset + 5] = 1.0,
                Ability::GainDucats { count } => buffer[offset + 6] = *count as f32,
                Ability::GainSecondary => buffer[offset + 7] = 1.0,
                Ability::GainPrimary => buffer[offset + 8] = 1.0,
                Ability::ChangeTertiary => buffer[offset + 9] = 1.0,
            }
        } else {
            buffer[offset] = 1.0; // empty second slot
        }
    } else {
        buffer[offset] = 1.0;
    }
    offset += 10;

    // stack depth (1)
    if let GamePhase::Action { ref action_state } = state.phase {
        buffer[offset] = action_state.ability_stack.len() as f32;
    }
    offset += 1;

    // draft hand by card type (46)
    if let GamePhase::Draft { ref draft_state } = state.phase {
        let hand = draft_state.hands[player_index];
        write_card_type_counts(hand, &state.card_lookup, &mut buffer[offset..offset + 46]);
    }
    offset += 46;

    // draft deck size (1)
    buffer[offset] = state.draft_deck.len() as f32;
    offset += 1;

    // destroyed pile by card type (46)
    write_card_type_counts(state.destroyed_pile, &state.card_lookup, &mut buffer[offset..offset + 46]);
    offset += 46;

    // buyer deck size (1)
    buffer[offset] = state.buyer_deck.len() as f32;
    offset += 1;

    debug_assert_eq!(offset, OBS_SIZE);
}

/// Encode the legal action mask as f32 (1.0 for legal, 0.0 for illegal).
pub fn encode_legal_mask(state: &GameState, mask: &mut [f32; NUM_ATOMIC_ACTIONS]) {
    let bool_mask = enumerate_atomic_legal_mask(state);
    for i in 0..NUM_ATOMIC_ACTIONS {
        mask[i] = if bool_mask[i] { 1.0 } else { 0.0 };
    }
}

fn write_card_type_counts(
    cards: crate::unordered_cards::UnorderedCards,
    card_lookup: &[Card; 256],
    out: &mut [f32],
) {
    for id in cards.iter() {
        let card = card_lookup[id as usize];
        out[card as usize] += 1.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atomic::{enumerate_atomic_choices, atomic_choice_to_index};
    use crate::draw_phase::execute_draw_phase;
    use crate::setup::create_initial_game_state;
    use rand::SeedableRng;
    use wyrand::WyRand;

    #[test]
    fn test_encoding_deterministic() {
        let mut rng = WyRand::seed_from_u64(42);
        let mut state = create_initial_game_state(2, &[true, true], &mut rng);
        execute_draw_phase(&mut state, &mut rng);

        let mut buf1 = [0.0f32; OBS_SIZE];
        let mut buf2 = [0.0f32; OBS_SIZE];
        encode_observation(&state, 0, &mut buf1);
        encode_observation(&state, 0, &mut buf2);
        assert_eq!(buf1, buf2);
    }

    #[test]
    fn test_encoding_different_perspectives() {
        let mut rng = WyRand::seed_from_u64(42);
        let mut state = create_initial_game_state(2, &[true, true], &mut rng);
        execute_draw_phase(&mut state, &mut rng);

        let mut buf0 = [0.0f32; OBS_SIZE];
        let mut buf1 = [0.0f32; OBS_SIZE];
        encode_observation(&state, 0, &mut buf0);
        encode_observation(&state, 1, &mut buf1);
        // They should differ (different player perspectives)
        assert_ne!(buf0, buf1);
    }

    #[test]
    fn test_legal_mask_matches_enumerate() {
        let mut rng = WyRand::seed_from_u64(42);
        let mut state = create_initial_game_state(2, &[true, true], &mut rng);
        execute_draw_phase(&mut state, &mut rng);

        let mut mask = [0.0f32; NUM_ATOMIC_ACTIONS];
        encode_legal_mask(&state, &mut mask);

        let choices = enumerate_atomic_choices(&state);
        let mask_count = mask.iter().filter(|&&v| v > 0.0).count();
        assert_eq!(mask_count, choices.len());

        for choice in &choices {
            let idx = atomic_choice_to_index(choice);
            assert_eq!(mask[idx], 1.0);
        }
    }

    #[test]
    fn test_encoding_size() {
        let mut rng = WyRand::seed_from_u64(42);
        let mut state = create_initial_game_state(3, &[true, true, true], &mut rng);
        execute_draw_phase(&mut state, &mut rng);

        let mut buf = [0.0f32; OBS_SIZE];
        encode_observation(&state, 0, &mut buf);
        // Should not panic and buffer should be fully written
        assert_eq!(buf.len(), OBS_SIZE);
    }
}
