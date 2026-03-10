use crate::action_phase::{can_afford_buyer, can_sell_to_any_buyer};
use crate::apply_choice::resolve_card_types_to_ids;
use crate::colors::{can_mix, is_primary, is_tertiary, perform_mix_unchecked, PRIMARIES, SECONDARIES, TERTIARIES, VALID_MIX_PAIRS};
use crate::types::*;
use crate::unordered_cards::UnorderedCards;
use smallvec::SmallVec;

// ── Multiset subset enumeration ──

/// Count occurrences of each card type in an UnorderedCards bitset,
/// returning a sorted array of (Card, count) pairs and the number of distinct types.
fn count_card_types(
    mask: UnorderedCards,
    card_lookup: &[Card; 256],
) -> ([Card; 46], [u8; 46], usize) {
    let mut card_types = [Card::BasicRed; 46];
    let mut type_counts = [0u8; 46];
    let mut len = 0usize;
    let mut seen: u64 = 0;
    for id in mask.iter() {
        let card = card_lookup[id as usize];
        let bit = 1u64 << (card as u64);
        if seen & bit == 0 {
            seen |= bit;
            card_types[len] = card;
            type_counts[len] = 1;
            len += 1;
        } else {
            for i in 0..len {
                if card_types[i] == card {
                    type_counts[i] += 1;
                    break;
                }
            }
        }
    }
    // Sort by card discriminant for deterministic output
    for i in 1..len {
        let mut j = i;
        while j > 0 && (card_types[j] as usize) < (card_types[j - 1] as usize) {
            card_types.swap(j, j - 1);
            type_counts.swap(j, j - 1);
            j -= 1;
        }
    }
    (card_types, type_counts, len)
}

/// Enumerate all non-empty subsets of a card-type multiset up to max_size.
/// Produces unique sorted SmallVec<[Card; 4]> entries without needing deduplication.
fn enumerate_multiset_subsets(
    types: &[Card],
    counts: &[u8],
    max_remaining: usize,
    current_subset: &mut SmallVec<[Card; 4]>,
    choices: &mut Vec<Choice>,
    make_choice: &impl Fn(SmallVec<[Card; 4]>) -> Choice,
) {
    if types.is_empty() || max_remaining == 0 {
        if !current_subset.is_empty() {
            choices.push(make_choice(current_subset.clone()));
        }
        return;
    }
    let card = types[0];
    let count = counts[0] as usize;
    let max_take = max_remaining.min(count);
    let base_len = current_subset.len();
    for take in 0..=max_take {
        enumerate_multiset_subsets(
            &types[1..],
            &counts[1..],
            max_remaining - take,
            current_subset,
            choices,
            make_choice,
        );
        current_subset.push(card);
    }
    current_subset.truncate(base_len);
}

// ── Mix sequence enumeration ──

fn enumerate_mix_sequences<F>(
    wheel: &ColorWheel,
    remaining_mixes: u32,
    choices: &mut Vec<Choice>,
    make_choice: F,
) where
    F: Fn(SmallVec<[(Color, Color); 2]>) -> Choice,
{
    // Always include skip-all (empty mixes)
    choices.push(make_choice(SmallVec::new()));

    for &(a, b) in &VALID_MIX_PAIRS {
        if wheel.get(a) > 0 && wheel.get(b) > 0 {
            let mut mixes1 = SmallVec::new();
            mixes1.push((a, b));
            choices.push(make_choice(mixes1));

            if remaining_mixes > 1 {
                let mut wheel2 = wheel.clone();
                perform_mix_unchecked(&mut wheel2, a, b);
                for &(c, d) in &VALID_MIX_PAIRS {
                    if wheel2.get(c) > 0 && wheel2.get(d) > 0 {
                        let mut mixes2 = SmallVec::new();
                        mixes2.push((a, b));
                        mixes2.push((c, d));
                        choices.push(make_choice(mixes2));
                    }
                }
            }
        }
    }
}

// ── Destroy choice dispatch ──

fn enumerate_destroy_choices(
    state: &GameState,
    player: &PlayerState,
    card: Card,
    choices: &mut Vec<Choice>,
) {
    match card.ability() {
        Ability::MixColors { count } => {
            enumerate_mix_sequences(
                &player.color_wheel,
                count,
                choices,
                |mixes| Choice::DestroyAndMix { card, mixes },
            );
        }
        Ability::Sell => {
            let mut has_buyer = false;
            for buyer in state.buyer_display.iter() {
                if can_afford_buyer(player, &buyer.buyer) {
                    has_buyer = true;
                    choices.push(Choice::DestroyAndSell {
                        card,
                        buyer: buyer.buyer,
                    });
                }
            }
            // Glass card acquisition
            if state.expansions.glass {
                for gi in state.glass_display.iter() {
                    for &color in &PRIMARIES {
                        if player.color_wheel.get(color) >= 4 {
                            has_buyer = true;
                            choices.push(Choice::DestroyAndSelectGlass {
                                card,
                                glass: gi.glass,
                                pay_color: color,
                            });
                        }
                    }
                }
            }
            if !has_buyer {
                choices.push(Choice::DestroyDraftedCard { card });
            }
        }
        Ability::Workshop { count } => {
            // Skip option (empty workshop_cards)
            choices.push(Choice::DestroyAndWorkshop {
                card,
                workshop_cards: SmallVec::new(),
            });
            let (card_types, type_counts, len) =
                count_card_types(player.workshop_cards, &state.card_lookup);
            if len > 0 {
                enumerate_multiset_subsets(
                    &card_types[..len],
                    &type_counts[..len],
                    count as usize,
                    &mut SmallVec::new(),
                    choices,
                    &|workshop_cards| Choice::DestroyAndWorkshop {
                        card,
                        workshop_cards,
                    },
                );
            }
        }
        Ability::DestroyCards => {
            if player.workshop_cards.is_empty() {
                choices.push(Choice::DestroyAndDestroyCards {
                    card,
                    target: None,
                });
            } else {
                let mut seen: u64 = 0;
                for id in player.workshop_cards.iter() {
                    let target_card = state.card_lookup[id as usize];
                    let bit = 1u64 << (target_card as u64);
                    if seen & bit != 0 {
                        continue;
                    }
                    seen |= bit;
                    choices.push(Choice::DestroyAndDestroyCards {
                        card,
                        target: Some(target_card),
                    });
                }
            }
        }
        _ => {
            choices.push(Choice::DestroyDraftedCard { card });
        }
    }
}

fn glass_ability_available_ext(state: &GameState, player: &PlayerState, glass: GlassCard) -> bool {
    let action_state = match &state.phase {
        GamePhase::Action { action_state } => action_state,
        _ => return false,
    };
    let bit = 1u16 << (glass as u16);
    if action_state.used_glass & bit != 0 {
        return false;
    }
    player.completed_glass.iter().any(|g| g.glass == glass)
}

fn enumerate_glass_choices(
    state: &GameState,
    player: &PlayerState,
    _player_index: usize,
    choices: &mut Vec<Choice>,
) {
    // GlassWorkshop - only if player has workshop_cards
    if glass_ability_available_ext(state, player, GlassCard::GlassWorkshop)
        && !player.workshop_cards.is_empty()
    {
        choices.push(Choice::ActivateGlassWorkshop);
    }
    // GlassDraw
    if glass_ability_available_ext(state, player, GlassCard::GlassDraw) {
        choices.push(Choice::ActivateGlassDraw);
    }
    // GlassMix - only if player has colors that can be mixed
    if glass_ability_available_ext(state, player, GlassCard::GlassMix) {
        let can_mix_any = VALID_MIX_PAIRS.iter().any(|&(a, b)| {
            player.color_wheel.get(a) > 0 && player.color_wheel.get(b) > 0
        });
        if can_mix_any {
            choices.push(Choice::ActivateGlassMix);
        }
    }
    // GlassExchange
    if glass_ability_available_ext(state, player, GlassCard::GlassExchange) {
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
    if glass_ability_available_ext(state, player, GlassCard::GlassMoveDrafted) {
        let mut seen: u64 = 0;
        for id in player.drafted_cards.iter() {
            let card = state.card_lookup[id as usize];
            let bit = 1u64 << (card as u64);
            if seen & bit != 0 { continue; }
            seen |= bit;
            choices.push(Choice::ActivateGlassMoveDrafted { card });
        }
    }
    // GlassUnmix
    if glass_ability_available_ext(state, player, GlassCard::GlassUnmix) {
        for &color in &ALL_COLORS {
            if !is_primary(color) && player.color_wheel.get(color) > 0 {
                choices.push(Choice::ActivateGlassUnmix { color });
            }
        }
    }
    // GlassTertiaryDucat
    if glass_ability_available_ext(state, player, GlassCard::GlassTertiaryDucat) {
        for &color in &TERTIARIES {
            if player.color_wheel.get(color) > 0 {
                choices.push(Choice::ActivateGlassTertiaryDucat { color });
            }
        }
    }
    // GlassReworkshop
    if glass_ability_available_ext(state, player, GlassCard::GlassReworkshop) {
        let mut seen: u64 = 0;
        for id in player.workshopped_cards.iter() {
            let card = state.card_lookup[id as usize];
            let bit = 1u64 << (card as u64);
            if seen & bit != 0 { continue; }
            seen |= bit;
            choices.push(Choice::ActivateGlassReworkshop { card });
        }
    }
    // GlassGainPrimary
    if glass_ability_available_ext(state, player, GlassCard::GlassGainPrimary) {
        choices.push(Choice::ActivateGlassGainPrimary);
    }
    // GlassDestroyClean
    if glass_ability_available_ext(state, player, GlassCard::GlassDestroyClean) {
        let mut seen: u64 = 0;
        for id in player.workshop_cards.iter() {
            let card = state.card_lookup[id as usize];
            let bit = 1u64 << (card as u64);
            if seen & bit != 0 { continue; }
            seen |= bit;
            choices.push(Choice::ActivateGlassDestroyClean { card });
        }
    }
    // GlassKeepBoth is passive - no activation choice
}

// ── Choice enumeration ──

pub fn enumerate_choices_into(state: &GameState, choices: &mut Vec<Choice>) {
    choices.clear();
    match &state.phase {
        GamePhase::Draft { draft_state } => {
            let hand = draft_state.hands[draft_state.current_player_index];
            let mut seen: u64 = 0;
            for id in hand.iter() {
                let card = state.card_lookup[id as usize];
                let bit = 1u64 << (card as u64);
                if seen & bit != 0 { continue; }
                seen |= bit;
                choices.push(Choice::DraftPick { card });
            }
        }
        GamePhase::Action { action_state } => {
            let player = &state.players[action_state.current_player_index];

            match action_state.ability_stack.last() {
                None => {
                    let mut seen: u64 = 0;
                    for id in player.drafted_cards.iter() {
                        let card = state.card_lookup[id as usize];
                        let bit = 1u64 << (card as u64);
                        if seen & bit != 0 { continue; }
                        seen |= bit;
                        enumerate_destroy_choices(state, player, card, choices);
                    }
                    // Glass ability choices (when stack is empty)
                    if state.expansions.glass {
                        enumerate_glass_choices(state, player, action_state.current_player_index, choices);
                    }
                    choices.push(Choice::EndTurn);
                }
                Some(Ability::Workshop { count }) => {
                    choices.push(Choice::SkipWorkshop);
                    let (card_types, type_counts, len) = count_card_types(player.workshop_cards, &state.card_lookup);
                    enumerate_multiset_subsets(
                        &card_types[..len],
                        &type_counts[..len],
                        *count as usize,
                        &mut SmallVec::new(),
                        choices,
                        &|card_types| Choice::Workshop { card_types },
                    );
                    // GlassReworkshop can also be used during Workshop
                    if state.expansions.glass && glass_ability_available_ext(state, player, GlassCard::GlassReworkshop) {
                        let mut seen: u64 = 0;
                        for id in player.workshopped_cards.iter() {
                            let card = state.card_lookup[id as usize];
                            let bit = 1u64 << (card as u64);
                            if seen & bit != 0 { continue; }
                            seen |= bit;
                            choices.push(Choice::ActivateGlassReworkshop { card });
                        }

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
                        let mut seen: u64 = 0;
                        for id in player.workshop_cards.iter() {
                            let card = state.card_lookup[id as usize];
                            let bit = 1u64 << (card as u64);
                            if seen & bit != 0 { continue; }
                            seen |= bit;
                            choices.push(Choice::DestroyDrawnCards { card: Some(card) });
                        }
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
                    for buyer in state.buyer_display.iter() {
                        if can_afford_buyer(player, &buyer.buyer) {
                            choices.push(Choice::SelectBuyer {
                                buyer: buyer.buyer,
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
                                        glass: gi.glass,
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
        Choice::DestroyDraftedCard { card } => {
            if !check_destroy_preconditions(state, card) {
                return false;
            }
            match card.ability() {
                Ability::Sell => !can_sell_to_any_buyer(state),
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
        Choice::SelectBuyer { buyer } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match action_state.ability_stack.last() {
                    Some(Ability::Sell) => {
                        let player = &state.players[action_state.current_player_index];
                        state.buyer_display.iter().any(|b| b.buyer == *buyer && can_afford_buyer(player, &b.buyer))
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
        Choice::DestroyAndSell { card, buyer } => {
            if !check_destroy_preconditions(state, card) {
                return false;
            }
            let player = &state.players[match &state.phase {
                GamePhase::Action { action_state } => action_state.current_player_index,
                _ => return false,
            }];
            state.buyer_display.iter().any(|b| b.buyer == *buyer && can_afford_buyer(player, &b.buyer))
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
                state.glass_display.iter().any(|g| g.glass == *glass)
            } else {
                false
            }
        }
        Choice::ActivateGlassWorkshop => {
            if let GamePhase::Action { ref action_state } = state.phase {
                action_state.ability_stack.is_empty()
                    && glass_ability_available_ext(state, &state.players[action_state.current_player_index], GlassCard::GlassWorkshop)
                    && !state.players[action_state.current_player_index].workshop_cards.is_empty()
            } else {
                false
            }
        }
        Choice::ActivateGlassDraw => {
            if let GamePhase::Action { ref action_state } = state.phase {
                action_state.ability_stack.is_empty()
                    && glass_ability_available_ext(state, &state.players[action_state.current_player_index], GlassCard::GlassDraw)
            } else {
                false
            }
        }
        Choice::ActivateGlassMix => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if !action_state.ability_stack.is_empty() { return false; }
                let player = &state.players[action_state.current_player_index];
                glass_ability_available_ext(state, player, GlassCard::GlassMix)
                    && VALID_MIX_PAIRS.iter().any(|&(a, b)| player.color_wheel.get(a) > 0 && player.color_wheel.get(b) > 0)
            } else {
                false
            }
        }
        Choice::ActivateGlassGainPrimary => {
            if let GamePhase::Action { ref action_state } = state.phase {
                action_state.ability_stack.is_empty()
                    && glass_ability_available_ext(state, &state.players[action_state.current_player_index], GlassCard::GlassGainPrimary)
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
                    && glass_ability_available_ext(state, player, GlassCard::GlassExchange)
            } else {
                false
            }
        }
        Choice::ActivateGlassMoveDrafted { card } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if !action_state.ability_stack.is_empty() { return false; }
                let player = &state.players[action_state.current_player_index];
                glass_ability_available_ext(state, player, GlassCard::GlassMoveDrafted)
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
                    && glass_ability_available_ext(state, player, GlassCard::GlassUnmix)
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
                    && glass_ability_available_ext(state, player, GlassCard::GlassTertiaryDucat)
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
                glass_ability_available_ext(state, player, GlassCard::GlassReworkshop)
                    && player.workshopped_cards.iter().any(|id| state.card_lookup[id as usize] == *card)
            } else {
                false
            }
        }
        Choice::ActivateGlassDestroyClean { card } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if !action_state.ability_stack.is_empty() { return false; }
                let player = &state.players[action_state.current_player_index];
                glass_ability_available_ext(state, player, GlassCard::GlassDestroyClean)
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
                if !glass_ability_available_ext(state, player, GlassCard::GlassReworkshop) { return false; }
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
            state.glass_display.iter().any(|g| g.glass == *glass)
        }
    }
}

