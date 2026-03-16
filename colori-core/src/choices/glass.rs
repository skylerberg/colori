use crate::action_phase::for_each_unique_card_type;
use crate::colors::{is_primary, TERTIARIES, VALID_MIX_PAIRS};
use crate::types::*;

use super::is_glass_ability_available;

pub(super) fn enumerate_glass_choices(
    state: &GameState,
    player: &PlayerState,
    choices: &mut Vec<Choice>,
) {
    // GlassWorkshop - only if player has workshop_cards
    if is_glass_ability_available(state, player, GlassCard::GlassWorkshop)
        && !player.workshop_cards.is_empty()
    {
        choices.push(Choice::ActivateGlassWorkshop);
    }
    // GlassDraw
    if is_glass_ability_available(state, player, GlassCard::GlassDraw) {
        choices.push(Choice::ActivateGlassDraw);
    }
    // GlassMix - only if player has colors that can be mixed
    if is_glass_ability_available(state, player, GlassCard::GlassMix) {
        let can_mix_any = VALID_MIX_PAIRS.iter().any(|&(a, b)| {
            player.color_wheel.get(a) > 0 && player.color_wheel.get(b) > 0
        });
        if can_mix_any {
            choices.push(Choice::ActivateGlassMix);
        }
    }
    // GlassExchange
    if is_glass_ability_available(state, player, GlassCard::GlassExchange) {
        for &lose in &ALL_MATERIAL_TYPES {
            if player.materials.get(lose) >= 1 {
                for &gain in &ALL_MATERIAL_TYPES {
                    if lose != gain {
                        choices.push(Choice::ActivateGlassExchange { lose, gain });
                    }
                }
            }
        }
    }
    // GlassMoveDrafted
    if is_glass_ability_available(state, player, GlassCard::GlassMoveDrafted) {
        for_each_unique_card_type(&player.drafted_cards, &state.card_lookup, |card| {
            choices.push(Choice::ActivateGlassMoveDrafted { card });
        });
    }
    // GlassUnmix
    if is_glass_ability_available(state, player, GlassCard::GlassUnmix) {
        for &color in &ALL_COLORS {
            if !is_primary(color) && player.color_wheel.get(color) > 0 {
                choices.push(Choice::ActivateGlassUnmix { color });
            }
        }
    }
    // GlassTertiaryDucat
    if is_glass_ability_available(state, player, GlassCard::GlassTertiaryDucat) {
        for &color in &TERTIARIES {
            if player.color_wheel.get(color) > 0 {
                choices.push(Choice::ActivateGlassTertiaryDucat { color });
            }
        }
    }
    // GlassReworkshop
    if is_glass_ability_available(state, player, GlassCard::GlassReworkshop) {
        for_each_unique_card_type(&player.workshopped_cards, &state.card_lookup, |card| {
            choices.push(Choice::ActivateGlassReworkshop { card });
        });
    }
    // GlassGainPrimary
    if is_glass_ability_available(state, player, GlassCard::GlassGainPrimary) {
        choices.push(Choice::ActivateGlassGainPrimary);
    }
    // GlassDestroyClean
    if is_glass_ability_available(state, player, GlassCard::GlassDestroyClean) {
        for_each_unique_card_type(&player.workshop_cards, &state.card_lookup, |card| {
            choices.push(Choice::ActivateGlassDestroyClean { card });
        });
    }
    // GlassKeepBoth is passive - no activation choice
}
