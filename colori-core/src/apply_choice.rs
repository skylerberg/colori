use crate::action_phase::*;
use crate::draft_phase::player_pick;
use crate::types::{ColoriChoice, GamePhase, GameState, PendingChoice};
use rand::Rng;

pub fn apply_choice<R: Rng>(state: &mut GameState, choice: &ColoriChoice, rng: &mut R) {
    match choice {
        ColoriChoice::DraftPick { card_instance_id } => {
            player_pick(state, *card_instance_id);
        }
        ColoriChoice::DestroyDraftedCard { card_instance_id } => {
            destroy_drafted_card(state, *card_instance_id, rng);
        }
        ColoriChoice::EndTurn => {
            end_player_turn(state, rng);
        }
        ColoriChoice::Workshop { card_instance_ids } => {
            resolve_workshop_choice(state, card_instance_ids, rng);
        }
        ColoriChoice::SkipWorkshop => {
            skip_workshop(state, rng);
        }
        ColoriChoice::DestroyDrawnCards { card_instance_ids } => {
            resolve_destroy_cards(state, card_instance_ids, rng);
        }
        ColoriChoice::SelectBuyer {
            buyer_instance_id,
        } => {
            resolve_select_buyer(state, *buyer_instance_id, rng);
        }
        ColoriChoice::GainSecondary { color } => {
            resolve_gain_secondary(state, *color, rng);
        }
        ColoriChoice::GainPrimary { color } => {
            resolve_gain_primary(state, *color, rng);
        }
        ColoriChoice::MixAll { mixes } => {
            for &(a, b) in mixes.iter() {
                resolve_mix_colors(state, a, b, rng);
            }
            // Skip any remaining mixes not used
            if let GamePhase::Action { ref action_state } = state.phase {
                if matches!(action_state.pending_choice, Some(PendingChoice::ChooseMix { .. })) {
                    skip_mix(state, rng);
                }
            }
        }
        ColoriChoice::SwapTertiary { lose, gain } => {
            resolve_choose_tertiary_to_lose(state, *lose);
            resolve_choose_tertiary_to_gain(state, *gain, rng);
        }
        ColoriChoice::DestroyAndMixAll {
            card_instance_id,
            mixes,
        } => {
            destroy_drafted_card(state, *card_instance_id, rng);
            for &(a, b) in mixes.iter() {
                resolve_mix_colors(state, a, b, rng);
            }
            if let GamePhase::Action { ref action_state } = state.phase {
                if matches!(action_state.pending_choice, Some(PendingChoice::ChooseMix { .. })) {
                    skip_mix(state, rng);
                }
            }
        }
        ColoriChoice::DestroyAndSell {
            card_instance_id,
            buyer_instance_id,
        } => {
            destroy_drafted_card(state, *card_instance_id, rng);
            resolve_select_buyer(state, *buyer_instance_id, rng);
        }
    }
}
