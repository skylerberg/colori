use crate::action_phase::*;
use crate::draft_phase::player_pick;
use crate::types::{Ability, ColoriChoice, CompoundFollowUp, GamePhase, GameState, PendingChoice};
use crate::unordered_cards::UnorderedCards;
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
            resolve_workshop_choice(state, *card_instance_ids, rng);
        }
        ColoriChoice::SkipWorkshop => {
            skip_workshop(state, rng);
        }
        ColoriChoice::DestroyDrawnCards { card_instance_ids } => {
            resolve_destroy_cards(state, *card_instance_ids, rng);
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
        ColoriChoice::KeepWorkshopCards { card_instance_ids } => {
            resolve_keep_workshop_cards(state, *card_instance_ids, rng);
        }
        ColoriChoice::CompoundDestroy { card_instance_id, targets, follow_up } => {
            // Step 1: Destroy the drafted card
            destroy_drafted_card(state, *card_instance_id, rng);

            // Step 2: Resolve destroy chain
            for &target_id in targets.iter() {
                let mut target_set = UnorderedCards::new();
                target_set.insert(target_id as u8);
                resolve_destroy_cards(state, target_set, rng);
            }

            // Step 3: If chain ended with DestroyCards + empty workshop, resolve empty
            if targets.is_empty() {
                resolve_destroy_cards(state, UnorderedCards::new(), rng);
            } else {
                let last_card = state.card_lookup[targets[targets.len() - 1] as usize];
                if matches!(last_card.ability(), Ability::DestroyCards { .. }) {
                    resolve_destroy_cards(state, UnorderedCards::new(), rng);
                }
            }

            // Step 4: Resolve follow-up
            match follow_up {
                CompoundFollowUp::None => {}
                CompoundFollowUp::Sell { buyer_instance_id } => {
                    resolve_select_buyer(state, *buyer_instance_id, rng);
                }
                CompoundFollowUp::MixAll { mixes } => {
                    for &(a, b) in mixes.iter() {
                        resolve_mix_colors(state, a, b, rng);
                    }
                    if let GamePhase::Action { ref action_state } = state.phase {
                        if matches!(action_state.pending_choice, Some(PendingChoice::ChooseMix { .. })) {
                            skip_mix(state, rng);
                        }
                    }
                }
                CompoundFollowUp::GainSecondary { color } => {
                    resolve_gain_secondary(state, *color, rng);
                }
                CompoundFollowUp::GainPrimary { color } => {
                    resolve_gain_primary(state, *color, rng);
                }
                CompoundFollowUp::SwapTertiary { lose, gain } => {
                    resolve_choose_tertiary_to_lose(state, *lose);
                    resolve_choose_tertiary_to_gain(state, *gain, rng);
                }
            }
        }
    }
}
