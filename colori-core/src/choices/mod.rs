mod destroy;
mod glass;
mod mix_sequences;
mod multiset;

use crate::action_phase::{can_afford_sell_card, can_sell_to_any_sell_card, for_each_unique_card_type};
use crate::apply_choice::resolve_card_types_to_ids;
use crate::colors::{can_mix, is_primary, is_tertiary, perform_mix_unchecked, PRIMARIES, SECONDARIES, TERTIARIES, VALID_MIX_PAIRS};
use crate::types::*;
use crate::unordered_cards::UnorderedCards;
use smallvec::SmallVec;

use destroy::enumerate_destroy_choices;
use glass::enumerate_glass_choices;
use mix_sequences::enumerate_mix_sequences;
use multiset::{count_card_types, enumerate_multiset_subsets, enumerate_multiset_subsets_exact};

pub(crate) fn is_glass_ability_available(state: &GameState, player: &PlayerState, glass: GlassCard) -> bool {
    let action_state = match &state.phase {
        GamePhase::Action { action_state } => action_state,
        _ => return false,
    };
    let bit = 1u16 << (glass as u16);
    if action_state.used_glass & bit != 0 {
        return false;
    }
    player.completed_glass.iter().any(|g| g.card == glass)
}

/// Check if we can skip enumerating sub-maximum workshop subsets.
/// Returns true when the player has no reason to workshop fewer than the max:
/// - No DestroyCards abilities in drafted cards (no need to keep workshop targets)
/// - No DrawCards abilities in drafted cards (destroying won't draw to workshop)
/// - No DrawCards workshop abilities in workshop cards (workshopping won't draw new cards)
pub(crate) fn should_force_max_workshop(state: &GameState, player: &PlayerState) -> bool {
    if !state.force_max_workshop {
        return false;
    }
    for id in player.drafted_cards.iter() {
        match state.card_lookup[id as usize].ability() {
            Ability::DestroyCards | Ability::DrawCards { .. } => return false,
            _ => {}
        }
    }
    for id in player.workshop_cards.iter() {
        let card = state.card_lookup[id as usize];
        for &wa in card.workshop_abilities() {
            if matches!(wa, Ability::DrawCards { .. }) {
                return false;
            }
        }
    }
    true
}

/// Map an ability to a unique bit for deduplication in abstract draft enumeration.
fn ability_bit(ability: Ability) -> u32 {
    match ability {
        Ability::Workshop { count } => count, // 1..=4 for workshop counts
        Ability::DrawCards { count } => 5 + count,
        Ability::MixColors { count } => 10 + count,
        Ability::DestroyCards => 1 << 15,
        Ability::Sell => 1 << 16,
        Ability::GainDucats { .. } => 1 << 17,
        Ability::GainSecondary => 1 << 18,
        Ability::GainPrimary => 1 << 19,
        Ability::ChangeTertiary => 1 << 20,
        Ability::MoveToDrafted => 1 << 21,
    }
}

/// Check if the current draft hand should use abstract (ability-based) picks.
/// Returns true when the hand at the perspective player's position contains
/// randomized cards that differ across determinizations.
fn is_draft_hand_abstract(state: &GameState, draft_state: &DraftState) -> bool {
    let perspective = match state.abstract_draft_perspective {
        Some(p) => p,
        None => return false,
    };
    let n = state.players.len();
    let initial_pick = state.abstract_draft_initial_pick;
    let current_pick = draft_state.pick_number;
    if current_pick <= initial_pick {
        return false; // no rotations yet, hand is known
    }
    let k = (current_pick - initial_pick) as usize; // rotations since determinization
    // Hand at position perspective came from original position (perspective - k + n*big) % n
    let orig_pos = (perspective + n - (k % n)) % n;
    // Known positions at determinization: {(perspective + m) % n : m = 0..=min(initial_pick, n-1)}
    let known_count = (initial_pick as usize + 1).min(n);
    for m in 0..known_count {
        if (perspective + m) % n == orig_pos {
            return false; // found in known set — hand is known, not abstract
        }
    }
    true // not found in known set — hand is abstract
}

// ── Choice enumeration ──

pub fn enumerate_choices_into(state: &GameState, choices: &mut Vec<Choice>) {
    choices.clear();
    match &state.phase {
        GamePhase::Draft { draft_state } => {
            let hand = draft_state.hands[draft_state.current_player_index];
            if is_draft_hand_abstract(state, draft_state) {
                // Enumerate unique abilities in the hand
                let mut seen_abilities: u32 = 0;
                for id in hand.iter() {
                    let ability = state.card_lookup[id as usize].ability();
                    let bit = ability_bit(ability);
                    if seen_abilities & bit == 0 {
                        seen_abilities |= bit;
                        choices.push(Choice::DraftPickAbility { ability });
                    }
                }
            } else {
                for_each_unique_card_type(&hand, &state.card_lookup, |card| {
                    choices.push(Choice::DraftPick { card });
                });
            }
        }
        GamePhase::Action { action_state } => {
            let player = &state.players[action_state.current_player_index];

            match action_state.ability_stack.last() {
                None => {
                    for_each_unique_card_type(&player.drafted_cards, &state.card_lookup, |card| {
                        enumerate_destroy_choices(state, player, card, choices);
                    });
                    // Glass ability choices (when stack is empty)
                    if state.expansions.glass {
                        enumerate_glass_choices(state, player, choices);
                    }
                    choices.push(Choice::EndTurn);
                }
                Some(Ability::Workshop { count }) => {
                    let force_max = should_force_max_workshop(state, player);
                    let (card_types, type_counts, len) = count_card_types(player.workshop_cards, &state.card_lookup);
                    let total_available: usize = type_counts[..len].iter().map(|&c| c as usize).sum();
                    if force_max {
                        if total_available <= *count as usize {
                            // Only one option: workshop everything
                            let mut all_cards = SmallVec::new();
                            for i in 0..len {
                                for _ in 0..type_counts[i] {
                                    all_cards.push(card_types[i]);
                                }
                            }
                            choices.push(Choice::Workshop { card_types: all_cards });
                        } else {
                            enumerate_multiset_subsets_exact(
                                &card_types[..len],
                                &type_counts[..len],
                                *count as usize,
                                &mut SmallVec::new(),
                                choices,
                                &|card_types| Choice::Workshop { card_types },
                            );
                        }
                    } else {
                        choices.push(Choice::SkipWorkshop);
                        enumerate_multiset_subsets(
                            &card_types[..len],
                            &type_counts[..len],
                            *count as usize,
                            &mut SmallVec::new(),
                            choices,
                            &|card_types| Choice::Workshop { card_types },
                        );
                    }
                    // GlassReworkshop can also be used during Workshop
                    if state.expansions.glass && is_glass_ability_available(state, player, GlassCard::GlassReworkshop) {
                        for_each_unique_card_type(&player.workshopped_cards, &state.card_lookup, |card| {
                            choices.push(Choice::ActivateGlassReworkshop { card });
                        });

                        // WorkshopWithReworkshop: workshop a card x2 via GlassReworkshop
                        if *count >= 2 {
                            let (card_types, type_counts, len) = count_card_types(player.workshop_cards, &state.card_lookup);
                            for i in 0..len {
                                let reworkshop_card = card_types[i];
                                // Size-0 case: just the reworkshop card x2 (uses 2 workshop slots)
                                choices.push(Choice::WorkshopWithReworkshop {
                                    reworkshop_card,
                                    other_cards: SmallVec::new(),
                                });
                                let remaining_slots = (*count as usize) - 2;
                                if remaining_slots > 0 {
                                    // Build reduced pool: same cards minus one instance of reworkshop_card
                                    let mut reduced_counts = type_counts;
                                    reduced_counts[i] -= 1;
                                    enumerate_multiset_subsets(
                                        &card_types[..len],
                                        &reduced_counts[..len],
                                        remaining_slots,
                                        &mut SmallVec::new(),
                                        choices,
                                        &|other_cards| Choice::WorkshopWithReworkshop {
                                            reworkshop_card,
                                            other_cards,
                                        },
                                    );
                                }
                            }
                        }
                    }
                }
                Some(Ability::DestroyCards) => {
                    if player.workshop_cards.is_empty() {
                        choices.push(Choice::DestroyDrawnCards { card: None });
                    } else {
                        for_each_unique_card_type(&player.workshop_cards, &state.card_lookup, |card| {
                            choices.push(Choice::DestroyDrawnCards { card: Some(card) });
                        });
                    }
                }
                Some(Ability::MixColors { count }) => {
                    enumerate_mix_sequences(
                        &player.color_wheel,
                        *count,
                        choices,
                        |mixes| Choice::MixAll { mixes },
                    );
                }
                Some(Ability::Sell) => {
                    for sell_card in state.sell_card_display.iter() {
                        if can_afford_sell_card(player, &sell_card.sell_card) {
                            choices.push(Choice::SelectSellCard {
                                sell_card: sell_card.sell_card,
                            });
                        }
                    }
                    // Glass card acquisition
                    if state.expansions.glass {
                        let player_index = action_state.current_player_index;
                        for gi in state.glass_display.iter() {
                            for &color in &PRIMARIES {
                                if state.players[player_index].color_wheel.get(color) >= 4 {
                                    choices.push(Choice::SelectGlass {
                                        glass: gi.card,
                                        pay_color: color,
                                    });
                                }
                            }
                        }
                    }
                }
                Some(Ability::GainSecondary) => {
                    for &c in SECONDARIES.iter() {
                        choices.push(Choice::GainSecondary { color: c });
                    }
                }
                Some(Ability::GainPrimary) => {
                    for &c in PRIMARIES.iter() {
                        choices.push(Choice::GainPrimary { color: c });
                    }
                }
                Some(Ability::ChangeTertiary) => {
                    for &lose in TERTIARIES.iter() {
                        if player.color_wheel.get(lose) > 0 {
                            for &gain in TERTIARIES.iter() {
                                if gain != lose {
                                    choices.push(Choice::SwapTertiary { lose, gain });
                                }
                            }
                        }
                    }
                }
                Some(Ability::MoveToDrafted) => {
                    choices.push(Choice::SkipMoveToDrafted);
                    for_each_unique_card_type(&player.workshop_cards, &state.card_lookup, |card| {
                        choices.push(Choice::SelectMoveToDrafted { card });
                    });
                }
                // Instant abilities (DrawCards, GainDucats) should never be on top
                // when waiting for a choice — they get processed immediately.
                Some(_) => {}
            }
        }
        _ => {}
    }
}

pub fn enumerate_choices(state: &GameState) -> Vec<Choice> {
    let mut choices = Vec::new();
    enumerate_choices_into(state, &mut choices);
    choices
}

// ── Choice availability ──

/// Check common preconditions for all destroy-drafted-card variants:
/// ability stack is empty and the card exists in the current player's drafted_cards.
fn check_destroy_preconditions(state: &GameState, card: &Card) -> bool {
    if let GamePhase::Action { ref action_state } = state.phase {
        action_state.ability_stack.is_empty()
            && state.players[action_state.current_player_index]
                .drafted_cards
                .iter()
                .any(|id| state.card_lookup[id as usize] == *card)
    } else {
        false
    }
}

pub fn check_choice_available(state: &GameState, choice: &Choice) -> bool {
    match choice {
        Choice::DraftPick { card } => {
            if let GamePhase::Draft { ref draft_state } = state.phase {
                let hand = draft_state.hands[draft_state.current_player_index];
                hand.iter().any(|id| state.card_lookup[id as usize] == *card)
            } else {
                false
            }
        }
        Choice::DraftPickAbility { ability } => {
            if let GamePhase::Draft { ref draft_state } = state.phase {
                let hand = draft_state.hands[draft_state.current_player_index];
                hand.iter().any(|id| state.card_lookup[id as usize].ability() == *ability)
            } else {
                false
            }
        }
        Choice::DestroyDraftedCard { card } => {
            if !check_destroy_preconditions(state, card) {
                return false;
            }
            match card.ability() {
                Ability::Sell => !can_sell_to_any_sell_card(state),
                Ability::MixColors { .. } => false,
                Ability::Workshop { .. } => false,
                Ability::DestroyCards => false,
                _ => true,
            }
        }
        Choice::EndTurn => {
            if let GamePhase::Action { ref action_state } = state.phase {
                action_state.ability_stack.is_empty()
            } else {
                false
            }
        }
        Choice::Workshop { card_types } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match action_state.ability_stack.last() {
                    Some(Ability::Workshop { .. }) => {
                        if card_types.is_empty() {
                            return false;
                        }
                        let player = &state.players[action_state.current_player_index];
                        resolve_card_types_to_ids(card_types, &player.workshop_cards, &state.card_lookup).is_some()
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
        Choice::SkipWorkshop => {
            if let GamePhase::Action { ref action_state } = state.phase {
                matches!(
                    action_state.ability_stack.last(),
                    Some(Ability::Workshop { .. })
                )
            } else {
                false
            }
        }
        Choice::DestroyDrawnCards { card } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match action_state.ability_stack.last() {
                    Some(Ability::DestroyCards) => match card {
                        None => true,
                        Some(card) => {
                            let player = &state.players[action_state.current_player_index];
                            player.workshop_cards.iter().any(|id| state.card_lookup[id as usize] == *card)
                        }
                    },
                    _ => false,
                }
            } else {
                false
            }
        }
        Choice::SelectSellCard { sell_card } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match action_state.ability_stack.last() {
                    Some(Ability::Sell) => {
                        let player = &state.players[action_state.current_player_index];
                        state.sell_card_display.iter().any(|b| b.sell_card == *sell_card && can_afford_sell_card(player, &b.sell_card))
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
        Choice::GainSecondary { color } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                matches!(
                    action_state.ability_stack.last(),
                    Some(Ability::GainSecondary)
                ) && SECONDARIES.contains(color)
            } else {
                false
            }
        }
        Choice::GainPrimary { color } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                matches!(
                    action_state.ability_stack.last(),
                    Some(Ability::GainPrimary)
                ) && PRIMARIES.contains(color)
            } else {
                false
            }
        }
        Choice::MixAll { mixes } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match action_state.ability_stack.last() {
                    Some(Ability::MixColors { .. }) => {
                        if mixes.is_empty() {
                            return true;
                        }
                        let player = &state.players[action_state.current_player_index];
                        let (a, b) = mixes[0];
                        player.color_wheel.get(a) > 0
                            && player.color_wheel.get(b) > 0
                            && can_mix(a, b)
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
        Choice::SwapTertiary { lose, gain } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match action_state.ability_stack.last() {
                    Some(Ability::ChangeTertiary) => {
                        let player = &state.players[action_state.current_player_index];
                        TERTIARIES.contains(lose)
                            && player.color_wheel.get(*lose) > 0
                            && TERTIARIES.contains(gain)
                            && *lose != *gain
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
        Choice::DestroyAndMix { card, mixes } => {
            if !check_destroy_preconditions(state, card) {
                return false;
            }
            if mixes.is_empty() {
                return true;
            }
            let player = &state.players[match &state.phase {
                GamePhase::Action { action_state } => action_state.current_player_index,
                _ => return false,
            }];
            let (a, b) = mixes[0];
            if player.color_wheel.get(a) == 0 || player.color_wheel.get(b) == 0 || !can_mix(a, b) {
                return false;
            }
            if mixes.len() > 1 {
                let mut wheel = player.color_wheel.clone();
                perform_mix_unchecked(&mut wheel, a, b);
                let (c, d) = mixes[1];
                if wheel.get(c) == 0 || wheel.get(d) == 0 || !can_mix(c, d) {
                    return false;
                }
            }
            true
        }
        Choice::DestroyAndSell { card, sell_card } => {
            if !check_destroy_preconditions(state, card) {
                return false;
            }
            let player = &state.players[match &state.phase {
                GamePhase::Action { action_state } => action_state.current_player_index,
                _ => return false,
            }];
            state.sell_card_display.iter().any(|b| b.sell_card == *sell_card && can_afford_sell_card(player, &b.sell_card))
        }
        Choice::DestroyAndWorkshop { card, workshop_cards } => {
            if !check_destroy_preconditions(state, card) {
                return false;
            }
            if workshop_cards.is_empty() {
                return true;
            }
            let player = &state.players[match &state.phase {
                GamePhase::Action { action_state } => action_state.current_player_index,
                _ => return false,
            }];
            resolve_card_types_to_ids(workshop_cards, &player.workshop_cards, &state.card_lookup).is_some()
        }
        Choice::DestroyAndDestroyCards { card, target } => {
            if !check_destroy_preconditions(state, card) {
                return false;
            }
            match target {
                None => true,
                Some(target_card) => {
                    let player = &state.players[match &state.phase {
                        GamePhase::Action { action_state } => action_state.current_player_index,
                        _ => return false,
                    }];
                    player.workshop_cards.iter().any(|id| state.card_lookup[id as usize] == *target_card)
                }
            }
        }
        Choice::SelectGlass { glass, pay_color } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if !matches!(action_state.ability_stack.last(), Some(Ability::Sell)) {
                    return false;
                }
                if !state.expansions.glass {
                    return false;
                }
                let player = &state.players[action_state.current_player_index];
                if !is_primary(*pay_color) || player.color_wheel.get(*pay_color) < 4 {
                    return false;
                }
                state.glass_display.iter().any(|g| g.card == *glass)
            } else {
                false
            }
        }
        Choice::ActivateGlassWorkshop => {
            if let GamePhase::Action { ref action_state } = state.phase {
                action_state.ability_stack.is_empty()
                    && is_glass_ability_available(state, &state.players[action_state.current_player_index], GlassCard::GlassWorkshop)
                    && !state.players[action_state.current_player_index].workshop_cards.is_empty()
            } else {
                false
            }
        }
        Choice::ActivateGlassDraw => {
            if let GamePhase::Action { ref action_state } = state.phase {
                action_state.ability_stack.is_empty()
                    && is_glass_ability_available(state, &state.players[action_state.current_player_index], GlassCard::GlassDraw)
            } else {
                false
            }
        }
        Choice::ActivateGlassMix => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if !action_state.ability_stack.is_empty() { return false; }
                let player = &state.players[action_state.current_player_index];
                is_glass_ability_available(state, player, GlassCard::GlassMix)
                    && VALID_MIX_PAIRS.iter().any(|&(a, b)| player.color_wheel.get(a) > 0 && player.color_wheel.get(b) > 0)
            } else {
                false
            }
        }
        Choice::ActivateGlassGainPrimary => {
            if let GamePhase::Action { ref action_state } = state.phase {
                action_state.ability_stack.is_empty()
                    && is_glass_ability_available(state, &state.players[action_state.current_player_index], GlassCard::GlassGainPrimary)
            } else {
                false
            }
        }
        Choice::ActivateGlassExchange { lose, gain } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if !action_state.ability_stack.is_empty() { return false; }
                let player = &state.players[action_state.current_player_index];
                *lose != *gain
                    && player.materials.get(*lose) >= 1
                    && is_glass_ability_available(state, player, GlassCard::GlassExchange)
            } else {
                false
            }
        }
        Choice::ActivateGlassMoveDrafted { card } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if !action_state.ability_stack.is_empty() { return false; }
                let player = &state.players[action_state.current_player_index];
                is_glass_ability_available(state, player, GlassCard::GlassMoveDrafted)
                    && player.drafted_cards.iter().any(|id| state.card_lookup[id as usize] == *card)
            } else {
                false
            }
        }
        Choice::ActivateGlassUnmix { color } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if !action_state.ability_stack.is_empty() { return false; }
                let player = &state.players[action_state.current_player_index];
                !is_primary(*color)
                    && player.color_wheel.get(*color) > 0
                    && is_glass_ability_available(state, player, GlassCard::GlassUnmix)
            } else {
                false
            }
        }
        Choice::ActivateGlassTertiaryDucat { color } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if !action_state.ability_stack.is_empty() { return false; }
                let player = &state.players[action_state.current_player_index];
                is_tertiary(*color)
                    && player.color_wheel.get(*color) > 0
                    && is_glass_ability_available(state, player, GlassCard::GlassTertiaryDucat)
            } else {
                false
            }
        }
        Choice::ActivateGlassReworkshop { card } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                // Can be used when stack is empty OR when Workshop is on top
                let stack_ok = action_state.ability_stack.is_empty()
                    || matches!(action_state.ability_stack.last(), Some(Ability::Workshop { .. }));
                if !stack_ok { return false; }
                let player = &state.players[action_state.current_player_index];
                is_glass_ability_available(state, player, GlassCard::GlassReworkshop)
                    && player.workshopped_cards.iter().any(|id| state.card_lookup[id as usize] == *card)
            } else {
                false
            }
        }
        Choice::ActivateGlassDestroyClean { card } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if !action_state.ability_stack.is_empty() { return false; }
                let player = &state.players[action_state.current_player_index];
                is_glass_ability_available(state, player, GlassCard::GlassDestroyClean)
                    && player.workshop_cards.iter().any(|id| state.card_lookup[id as usize] == *card)
            } else {
                false
            }
        }
        Choice::WorkshopWithReworkshop { reworkshop_card, other_cards } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                let count = match action_state.ability_stack.last() {
                    Some(Ability::Workshop { count }) => *count as usize,
                    _ => return false,
                };
                let total = 2 + other_cards.len();
                if total > count { return false; }
                let player = &state.players[action_state.current_player_index];
                if !is_glass_ability_available(state, player, GlassCard::GlassReworkshop) { return false; }
                // Verify reworkshop_card exists in workshop_cards
                if !player.workshop_cards.iter().any(|id| state.card_lookup[id as usize] == *reworkshop_card) {
                    return false;
                }
                // Verify other_cards can all be resolved from workshop_cards minus one instance of reworkshop_card
                let mut used = UnorderedCards::new();
                for id in player.workshop_cards.iter() {
                    if state.card_lookup[id as usize] == *reworkshop_card {
                        used.insert(id);
                        break;
                    }
                }
                for &ct in other_cards.iter() {
                    let mut found = false;
                    for id in player.workshop_cards.iter() {
                        if !used.contains(id) && state.card_lookup[id as usize] == ct {
                            used.insert(id);
                            found = true;
                            break;
                        }
                    }
                    if !found { return false; }
                }
                true
            } else {
                false
            }
        }
        Choice::DestroyAndSelectGlass { card, glass, pay_color } => {
            if !check_destroy_preconditions(state, card) {
                return false;
            }
            if !state.expansions.glass {
                return false;
            }
            let player = &state.players[match &state.phase {
                GamePhase::Action { action_state } => action_state.current_player_index,
                _ => return false,
            }];
            if !is_primary(*pay_color) || player.color_wheel.get(*pay_color) < 4 {
                return false;
            }
            state.glass_display.iter().any(|g| g.card == *glass)
        }
        Choice::SelectMoveToDrafted { card } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                matches!(action_state.ability_stack.last(), Some(Ability::MoveToDrafted))
                    && state.players[action_state.current_player_index]
                        .workshop_cards
                        .iter()
                        .any(|id| state.card_lookup[id as usize] == *card)
            } else {
                false
            }
        }
        Choice::SkipMoveToDrafted => {
            if let GamePhase::Action { ref action_state } = state.phase {
                matches!(action_state.ability_stack.last(), Some(Ability::MoveToDrafted))
            } else {
                false
            }
        }
    }
}
