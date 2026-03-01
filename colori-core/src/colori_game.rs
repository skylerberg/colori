use crate::action_phase::{
    ability_canonical_phase, destroy_drafted_card, end_player_turn, initialize_action_phase,
    process_queue, resolve_choose_tertiary_to_gain, resolve_choose_tertiary_to_lose,
    resolve_destroy_cards, resolve_gain_primary, resolve_gain_secondary,
    resolve_keep_workshop_cards, resolve_select_buyer, resolve_workshop_choice, skip_workshop,
    can_sell_to_any_buyer,
};
use crate::apply_choice::apply_choice;
use crate::color_wheel::{can_pay_cost, pay_cost, perform_mix_unchecked};
use crate::colors::{can_mix, PRIMARIES, SECONDARIES, TERTIARIES, VALID_MIX_PAIRS};
use crate::deck_utils::draw_from_deck;
use crate::draft_phase::{confirm_pass, player_pick};
use crate::draw_phase::execute_draw_phase;
use crate::types::*;
use crate::unordered_cards::UnorderedCards;
use rand::Rng;
use smallvec::SmallVec;

// ── Subset enumeration ──

fn enumerate_subsets_into(
    mask: UnorderedCards,
    max_size: usize,
    choices: &mut Vec<ColoriChoice>,
    f: impl Fn(UnorderedCards) -> ColoriChoice,
) {
    let mut sub = mask.0;
    while sub[0] != 0 || sub[1] != 0 {
        if (sub[0].count_ones() + sub[1].count_ones()) as usize <= max_size {
            choices.push(f(UnorderedCards(sub)));
        }
        let (new_lo, borrow) = sub[0].overflowing_sub(1);
        let new_hi = if borrow { sub[1].wrapping_sub(1) } else { sub[1] };
        sub = [new_lo & mask.0[0], new_hi & mask.0[1]];
    }
}

// ── Buyer affordability ──

#[inline]
fn can_afford_buyer(player: &PlayerState, buyer: &BuyerCard) -> bool {
    player.materials.get(buyer.required_material()) >= 1
        && can_pay_cost(&player.color_wheel, buyer.color_cost())
}

// ── Mix sequence enumeration ──

fn enumerate_mix_sequences<F>(
    wheel: &ColorWheel,
    remaining: u32,
    choices: &mut Vec<ColoriChoice>,
    make_choice: F,
) where
    F: Fn(SmallVec<[(Color, Color); 2]>) -> ColoriChoice,
{
    // Always include skip-all (empty mixes)
    choices.push(make_choice(SmallVec::new()));

    for &(a, b) in &VALID_MIX_PAIRS {
        if wheel.get(a) > 0 && wheel.get(b) > 0 {
            let mut mixes1 = SmallVec::new();
            mixes1.push((a, b));
            choices.push(make_choice(mixes1));

            if remaining > 1 {
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

// ── Choice enumeration ──

pub fn enumerate_choices_into(state: &GameState, choices: &mut Vec<ColoriChoice>) {
    choices.clear();
    match &state.phase {
        GamePhase::Draft { draft_state } => {
            if draft_state.waiting_for_pass {
                return;
            }
            let hand = draft_state.hands[draft_state.current_player_index];
            for id in hand.iter() {
                choices.push(ColoriChoice::DraftPick {
                    card_instance_id: id as u32,
                });
            }
        }
        GamePhase::Action { action_state } => {
            let player = &state.players[action_state.current_player_index];
            let pending = &action_state.pending_choice;

            match pending {
                None => {
                    for id in player.drafted_cards.iter() {
                        let card = state.card_lookup[id as usize];
                        let card_instance_id = id as u32;
                        match card.ability() {
                            Ability::MixColors { count } => {
                                enumerate_mix_sequences(
                                    &player.color_wheel,
                                    count,
                                    choices,
                                    |mixes| ColoriChoice::DestroyAndMixAll {
                                        card_instance_id,
                                        mixes,
                                    },
                                );
                            }
                            Ability::Sell => {
                                let mut has_buyer = false;
                                for g in state.buyer_display.iter() {
                                    if can_afford_buyer(player, &g.buyer) {
                                        has_buyer = true;
                                        choices.push(ColoriChoice::DestroyAndSell {
                                            card_instance_id,
                                            buyer_instance_id: g.instance_id,
                                        });
                                    }
                                }
                                if !has_buyer {
                                    choices.push(ColoriChoice::DestroyDraftedCard {
                                        card_instance_id,
                                    });
                                }
                            }
                            _ => {
                                choices.push(ColoriChoice::DestroyDraftedCard {
                                    card_instance_id,
                                });
                            }
                        }
                    }
                    choices.push(ColoriChoice::EndTurn);
                }
                Some(PendingChoice::ChooseCardsForWorkshop { count }) => {
                    choices.push(ColoriChoice::SkipWorkshop);
                    enumerate_subsets_into(
                        player.workshop_cards,
                        *count as usize,
                        choices,
                        |ids| ColoriChoice::Workshop { card_instance_ids: ids },
                    );
                }
                Some(PendingChoice::ChooseCardsToDestroy) => {
                    if player.workshop_cards.is_empty() {
                        choices.push(ColoriChoice::DestroyDrawnCards {
                            card_instance_ids: UnorderedCards::new(),
                        });
                    } else {
                        enumerate_subsets_into(
                            player.workshop_cards,
                            1,
                            choices,
                            |ids| ColoriChoice::DestroyDrawnCards { card_instance_ids: ids },
                        );
                    }
                }
                Some(PendingChoice::ChooseMix { remaining }) => {
                    enumerate_mix_sequences(
                        &player.color_wheel,
                        *remaining,
                        choices,
                        |mixes| ColoriChoice::MixAll { mixes },
                    );
                }
                Some(PendingChoice::ChooseBuyer) => {
                    for g in state.buyer_display.iter() {
                        if can_afford_buyer(player, &g.buyer) {
                            choices.push(ColoriChoice::SelectBuyer {
                                buyer_instance_id: g.instance_id,
                            });
                        }
                    }
                }
                Some(PendingChoice::ChooseSecondaryColor) => {
                    for &c in SECONDARIES.iter() {
                        choices.push(ColoriChoice::GainSecondary { color: c });
                    }
                }
                Some(PendingChoice::ChoosePrimaryColor) => {
                    for &c in PRIMARIES.iter() {
                        choices.push(ColoriChoice::GainPrimary { color: c });
                    }
                }
                Some(PendingChoice::ChooseTertiaryToLose) => {
                    for &lose in TERTIARIES.iter() {
                        if player.color_wheel.get(lose) > 0 {
                            for &gain in TERTIARIES.iter() {
                                if gain != lose {
                                    choices.push(ColoriChoice::SwapTertiary { lose, gain });
                                }
                            }
                        }
                    }
                }
            }
        }
        GamePhase::Cleanup { cleanup_state } => {
            let player = &state.players[cleanup_state.current_player_index];
            let mask = player.workshop_cards;
            // Enumerate all subsets (including empty = discard all)
            // Empty set (discard all)
            choices.push(ColoriChoice::KeepWorkshopCards {
                card_instance_ids: UnorderedCards::new(),
            });
            // All non-empty subsets
            let mut sub = mask.0;
            while sub[0] != 0 || sub[1] != 0 {
                choices.push(ColoriChoice::KeepWorkshopCards {
                    card_instance_ids: UnorderedCards(sub),
                });
                let (new_lo, borrow) = sub[0].overflowing_sub(1);
                let new_hi = if borrow { sub[1].wrapping_sub(1) } else { sub[1] };
                sub = [new_lo & mask.0[0], new_hi & mask.0[1]];
            }
        }
        _ => {}
    }
}

pub fn enumerate_choices(state: &GameState) -> Vec<ColoriChoice> {
    let mut choices = Vec::new();
    enumerate_choices_into(state, &mut choices);
    choices
}

pub fn filter_choices_canonical(state: &GameState, choices: &mut Vec<ColoriChoice>) {
    let canonical_phase = match &state.phase {
        GamePhase::Action { action_state } if action_state.pending_choice.is_none() => {
            action_state.canonical_phase
        }
        _ => return,
    };
    if canonical_phase == 0 {
        return;
    }
    choices.retain(|choice| match choice {
        ColoriChoice::EndTurn => true,
        ColoriChoice::DestroyDraftedCard { card_instance_id } => {
            let card = state.card_lookup[*card_instance_id as usize];
            match ability_canonical_phase(&card.ability()) {
                Some(phase) => phase >= canonical_phase,
                None => true,
            }
        }
        ColoriChoice::DestroyAndMixAll { .. } => 1 >= canonical_phase,
        ColoriChoice::DestroyAndSell { .. } => true, // phase 2, always highest
        _ => true,
    });
}

pub fn enumerate_choices_canonical_into(state: &GameState, choices: &mut Vec<ColoriChoice>) {
    enumerate_choices_into(state, choices);
    filter_choices_canonical(state, choices);
}

// ── Apply choice with AI post-processing ──

pub fn apply_choice_to_state<R: Rng>(state: &mut GameState, choice: &ColoriChoice, rng: &mut R) {
    apply_choice(state, choice, rng);

    if matches!(choice, ColoriChoice::DraftPick { .. }) {
        if let GamePhase::Draft { ref mut draft_state } = state.phase {
            if draft_state.waiting_for_pass {
                confirm_pass(state);
            }
        }
    }
    if matches!(choice, ColoriChoice::EndTurn) {
        if matches!(state.phase, GamePhase::Draw) {
            execute_draw_phase(state, rng);
        }
    }
    if matches!(choice, ColoriChoice::KeepWorkshopCards { .. }) {
        if matches!(state.phase, GamePhase::Draw) {
            execute_draw_phase(state, rng);
        }
    }
}

// ── Choice availability ──

pub fn check_choice_available(state: &GameState, choice: &ColoriChoice) -> bool {
    match choice {
        ColoriChoice::DraftPick { card_instance_id } => {
            if let GamePhase::Draft { ref draft_state } = state.phase {
                draft_state.hands[draft_state.current_player_index]
                    .contains(*card_instance_id as u8)
            } else {
                false
            }
        }
        ColoriChoice::DestroyDraftedCard { card_instance_id } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if action_state.pending_choice.is_some() {
                    return false;
                }
                state.players[action_state.current_player_index]
                    .drafted_cards
                    .contains(*card_instance_id as u8)
            } else {
                false
            }
        }
        ColoriChoice::EndTurn => {
            if let GamePhase::Action { ref action_state } = state.phase {
                action_state.pending_choice.is_none()
            } else {
                false
            }
        }
        ColoriChoice::Workshop { card_instance_ids } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match &action_state.pending_choice {
                    Some(PendingChoice::ChooseCardsForWorkshop { .. }) => {
                        let player = &state.players[action_state.current_player_index];
                        !card_instance_ids.is_empty()
                            && card_instance_ids.difference(player.workshop_cards).is_empty()
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
        ColoriChoice::SkipWorkshop => {
            if let GamePhase::Action { ref action_state } = state.phase {
                matches!(
                    action_state.pending_choice,
                    Some(PendingChoice::ChooseCardsForWorkshop { .. })
                )
            } else {
                false
            }
        }
        ColoriChoice::DestroyDrawnCards { card_instance_ids } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match &action_state.pending_choice {
                    Some(PendingChoice::ChooseCardsToDestroy) => {
                        let player = &state.players[action_state.current_player_index];
                        card_instance_ids.difference(player.workshop_cards).is_empty()
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
        ColoriChoice::SelectBuyer {
            buyer_instance_id,
        } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match &action_state.pending_choice {
                    Some(PendingChoice::ChooseBuyer) => {
                        let player = &state.players[action_state.current_player_index];
                        state
                            .buyer_display
                            .iter()
                            .find(|g| g.instance_id == *buyer_instance_id)
                            .map(|g| can_afford_buyer(player, &g.buyer))
                            .unwrap_or(false)
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
        ColoriChoice::GainSecondary { color } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                matches!(
                    action_state.pending_choice,
                    Some(PendingChoice::ChooseSecondaryColor)
                ) && SECONDARIES.contains(color)
            } else {
                false
            }
        }
        ColoriChoice::GainPrimary { color } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                matches!(
                    action_state.pending_choice,
                    Some(PendingChoice::ChoosePrimaryColor)
                ) && PRIMARIES.contains(color)
            } else {
                false
            }
        }
        ColoriChoice::MixAll { mixes } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match &action_state.pending_choice {
                    Some(PendingChoice::ChooseMix { .. }) => {
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
        ColoriChoice::SwapTertiary { lose, gain } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match &action_state.pending_choice {
                    Some(PendingChoice::ChooseTertiaryToLose) => {
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
        ColoriChoice::DestroyAndMixAll {
            card_instance_id,
            mixes,
        } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if action_state.pending_choice.is_some() {
                    return false;
                }
                let player = &state.players[action_state.current_player_index];
                if !player.drafted_cards.contains(*card_instance_id as u8) {
                    return false;
                }
                if mixes.is_empty() {
                    return true;
                }
                let (a, b) = mixes[0];
                player.color_wheel.get(a) > 0
                    && player.color_wheel.get(b) > 0
                    && can_mix(a, b)
            } else {
                false
            }
        }
        ColoriChoice::DestroyAndSell {
            card_instance_id,
            buyer_instance_id,
        } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if action_state.pending_choice.is_some() {
                    return false;
                }
                let player = &state.players[action_state.current_player_index];
                if !player.drafted_cards.contains(*card_instance_id as u8) {
                    return false;
                }
                state
                    .buyer_display
                    .iter()
                    .find(|g| g.instance_id == *buyer_instance_id)
                    .map(|g| can_afford_buyer(player, &g.buyer))
                    .unwrap_or(false)
            } else {
                false
            }
        }
        ColoriChoice::KeepWorkshopCards { card_instance_ids } => {
            if let GamePhase::Cleanup { ref cleanup_state } = state.phase {
                let player = &state.players[cleanup_state.current_player_index];
                card_instance_ids.difference(player.workshop_cards).is_empty()
            } else {
                false
            }
        }
    }
}

// ── Abstract choice conversion ──

pub fn abstract_choice(choice: &ColoriChoice, state: &GameState) -> AbstractChoice {
    match choice {
        ColoriChoice::DraftPick { card_instance_id } => AbstractChoice::DraftPick {
            card: state.card_lookup[*card_instance_id as usize],
        },
        ColoriChoice::DestroyDraftedCard { card_instance_id } => {
            AbstractChoice::DestroyDraftedCard {
                card: state.card_lookup[*card_instance_id as usize],
            }
        }
        ColoriChoice::EndTurn => AbstractChoice::EndTurn,
        ColoriChoice::Workshop { card_instance_ids } => {
            let mut card_types: SmallVec<[Card; 4]> = card_instance_ids
                .iter()
                .map(|id| state.card_lookup[id as usize])
                .collect();
            card_types.sort_by_key(|c| *c as usize);
            AbstractChoice::Workshop { card_types }
        }
        ColoriChoice::SkipWorkshop => AbstractChoice::SkipWorkshop,
        ColoriChoice::DestroyDrawnCards { card_instance_ids } => {
            let mut card_types: SmallVec<[Card; 4]> = card_instance_ids
                .iter()
                .map(|id| state.card_lookup[id as usize])
                .collect();
            card_types.sort_by_key(|c| *c as usize);
            AbstractChoice::DestroyDrawnCards { card_types }
        }
        ColoriChoice::SelectBuyer { buyer_instance_id } => AbstractChoice::SelectBuyer {
            buyer: state.buyer_lookup[*buyer_instance_id as usize],
        },
        ColoriChoice::GainSecondary { color } => AbstractChoice::GainSecondary { color: *color },
        ColoriChoice::GainPrimary { color } => AbstractChoice::GainPrimary { color: *color },
        ColoriChoice::MixAll { mixes } => AbstractChoice::MixAll {
            mixes: mixes.clone(),
        },
        ColoriChoice::SwapTertiary { lose, gain } => AbstractChoice::SwapTertiary {
            lose: *lose,
            gain: *gain,
        },
        ColoriChoice::DestroyAndMixAll {
            card_instance_id,
            mixes,
        } => AbstractChoice::DestroyAndMixAll {
            card: state.card_lookup[*card_instance_id as usize],
            mixes: mixes.clone(),
        },
        ColoriChoice::DestroyAndSell {
            card_instance_id,
            buyer_instance_id,
        } => AbstractChoice::DestroyAndSell {
            card: state.card_lookup[*card_instance_id as usize],
            buyer: state.buyer_lookup[*buyer_instance_id as usize],
        },
        ColoriChoice::KeepWorkshopCards { card_instance_ids } => {
            let mut card_types: SmallVec<[Card; 4]> = card_instance_ids
                .iter()
                .map(|id| state.card_lookup[id as usize])
                .collect();
            card_types.sort_by_key(|c| *c as usize);
            AbstractChoice::KeepWorkshopCards { card_types }
        }
    }
}

pub fn resolve_abstract_choice(abs: &AbstractChoice, state: &GameState) -> Option<ColoriChoice> {
    match abs {
        AbstractChoice::EndTurn => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if action_state.pending_choice.is_none() {
                    return Some(ColoriChoice::EndTurn);
                }
            }
            None
        }
        AbstractChoice::SkipWorkshop => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if matches!(
                    action_state.pending_choice,
                    Some(PendingChoice::ChooseCardsForWorkshop { .. })
                ) {
                    return Some(ColoriChoice::SkipWorkshop);
                }
            }
            None
        }
        AbstractChoice::GainSecondary { color } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if matches!(
                    action_state.pending_choice,
                    Some(PendingChoice::ChooseSecondaryColor)
                ) && SECONDARIES.contains(color)
                {
                    return Some(ColoriChoice::GainSecondary { color: *color });
                }
            }
            None
        }
        AbstractChoice::GainPrimary { color } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if matches!(
                    action_state.pending_choice,
                    Some(PendingChoice::ChoosePrimaryColor)
                ) && PRIMARIES.contains(color)
                {
                    return Some(ColoriChoice::GainPrimary { color: *color });
                }
            }
            None
        }
        AbstractChoice::SwapTertiary { lose, gain } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if let Some(PendingChoice::ChooseTertiaryToLose) = &action_state.pending_choice {
                    let player = &state.players[action_state.current_player_index];
                    if TERTIARIES.contains(lose)
                        && player.color_wheel.get(*lose) > 0
                        && TERTIARIES.contains(gain)
                        && lose != gain
                    {
                        return Some(ColoriChoice::SwapTertiary {
                            lose: *lose,
                            gain: *gain,
                        });
                    }
                }
            }
            None
        }
        AbstractChoice::MixAll { mixes } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if let Some(PendingChoice::ChooseMix { .. }) = &action_state.pending_choice {
                    if mixes.is_empty() {
                        return Some(ColoriChoice::MixAll {
                            mixes: SmallVec::new(),
                        });
                    }
                    let player = &state.players[action_state.current_player_index];
                    let (a, b) = mixes[0];
                    if player.color_wheel.get(a) > 0
                        && player.color_wheel.get(b) > 0
                        && can_mix(a, b)
                    {
                        if mixes.len() > 1 {
                            let mut wheel = player.color_wheel.clone();
                            perform_mix_unchecked(&mut wheel, a, b);
                            let (c, d) = mixes[1];
                            if wheel.get(c) == 0 || wheel.get(d) == 0 || !can_mix(c, d) {
                                return None;
                            }
                        }
                        return Some(ColoriChoice::MixAll {
                            mixes: mixes.clone(),
                        });
                    }
                }
            }
            None
        }
        AbstractChoice::DraftPick { card } => {
            if let GamePhase::Draft { ref draft_state } = state.phase {
                if draft_state.waiting_for_pass {
                    return None;
                }
                let hand = draft_state.hands[draft_state.current_player_index];
                for id in hand.iter() {
                    if state.card_lookup[id as usize] == *card {
                        return Some(ColoriChoice::DraftPick {
                            card_instance_id: id as u32,
                        });
                    }
                }
            }
            None
        }
        AbstractChoice::DestroyDraftedCard { card } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if action_state.pending_choice.is_some() {
                    return None;
                }
                let player = &state.players[action_state.current_player_index];
                for id in player.drafted_cards.iter() {
                    if state.card_lookup[id as usize] == *card {
                        match card.ability() {
                            Ability::Sell => {
                                if can_sell_to_any_buyer(state) {
                                    return None;
                                }
                            }
                            Ability::MixColors { .. } => {
                                return None;
                            }
                            _ => {}
                        }
                        return Some(ColoriChoice::DestroyDraftedCard {
                            card_instance_id: id as u32,
                        });
                    }
                }
            }
            None
        }
        AbstractChoice::DestroyAndMixAll { card, mixes } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if action_state.pending_choice.is_some() {
                    return None;
                }
                let player = &state.players[action_state.current_player_index];
                for id in player.drafted_cards.iter() {
                    if state.card_lookup[id as usize] == *card {
                        if !mixes.is_empty() {
                            let (a, b) = mixes[0];
                            if player.color_wheel.get(a) == 0
                                || player.color_wheel.get(b) == 0
                                || !can_mix(a, b)
                            {
                                return None;
                            }
                            if mixes.len() > 1 {
                                let mut wheel = player.color_wheel.clone();
                                perform_mix_unchecked(&mut wheel, a, b);
                                let (c, d) = mixes[1];
                                if wheel.get(c) == 0 || wheel.get(d) == 0 || !can_mix(c, d) {
                                    return None;
                                }
                            }
                        }
                        return Some(ColoriChoice::DestroyAndMixAll {
                            card_instance_id: id as u32,
                            mixes: mixes.clone(),
                        });
                    }
                }
            }
            None
        }
        AbstractChoice::DestroyAndSell { card, buyer } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if action_state.pending_choice.is_some() {
                    return None;
                }
                let player = &state.players[action_state.current_player_index];
                let mut card_id = None;
                for id in player.drafted_cards.iter() {
                    if state.card_lookup[id as usize] == *card {
                        card_id = Some(id as u32);
                        break;
                    }
                }
                let card_id = card_id?;
                for g in state.buyer_display.iter() {
                    if state.buyer_lookup[g.instance_id as usize] == *buyer
                        && can_afford_buyer(player, &g.buyer)
                    {
                        return Some(ColoriChoice::DestroyAndSell {
                            card_instance_id: card_id,
                            buyer_instance_id: g.instance_id,
                        });
                    }
                }
            }
            None
        }
        AbstractChoice::SelectBuyer { buyer } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if let Some(PendingChoice::ChooseBuyer) = &action_state.pending_choice {
                    let player = &state.players[action_state.current_player_index];
                    for g in state.buyer_display.iter() {
                        if state.buyer_lookup[g.instance_id as usize] == *buyer
                            && can_afford_buyer(player, &g.buyer)
                        {
                            return Some(ColoriChoice::SelectBuyer {
                                buyer_instance_id: g.instance_id,
                            });
                        }
                    }
                }
            }
            None
        }
        AbstractChoice::Workshop { card_types } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if let Some(PendingChoice::ChooseCardsForWorkshop { .. }) =
                    &action_state.pending_choice
                {
                    let player = &state.players[action_state.current_player_index];
                    let mut ids = UnorderedCards::new();
                    let mut used = UnorderedCards::new();
                    for &ct in card_types.iter() {
                        let mut found = false;
                        for id in player.workshop_cards.iter() {
                            if !used.contains(id)
                                && state.card_lookup[id as usize] == ct
                            {
                                ids.insert(id);
                                used.insert(id);
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            return None;
                        }
                    }
                    return Some(ColoriChoice::Workshop {
                        card_instance_ids: ids,
                    });
                }
            }
            None
        }
        AbstractChoice::DestroyDrawnCards { card_types } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if let Some(PendingChoice::ChooseCardsToDestroy) =
                    &action_state.pending_choice
                {
                    let player = &state.players[action_state.current_player_index];
                    let mut ids = UnorderedCards::new();
                    let mut used = UnorderedCards::new();
                    for &ct in card_types.iter() {
                        let mut found = false;
                        for id in player.workshop_cards.iter() {
                            if !used.contains(id)
                                && state.card_lookup[id as usize] == ct
                            {
                                ids.insert(id);
                                used.insert(id);
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            return None;
                        }
                    }
                    return Some(ColoriChoice::DestroyDrawnCards {
                        card_instance_ids: ids,
                    });
                }
            }
            None
        }
        AbstractChoice::KeepWorkshopCards { card_types } => {
            if let GamePhase::Cleanup { ref cleanup_state } = state.phase {
                let player = &state.players[cleanup_state.current_player_index];
                let mut ids = UnorderedCards::new();
                let mut used = UnorderedCards::new();
                for &ct in card_types.iter() {
                    let mut found = false;
                    for id in player.workshop_cards.iter() {
                        if !used.contains(id)
                            && state.card_lookup[id as usize] == ct
                        {
                            ids.insert(id);
                            used.insert(id);
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        return None;
                    }
                }
                return Some(ColoriChoice::KeepWorkshopCards {
                    card_instance_ids: ids,
                });
            }
            None
        }
    }
}

pub fn deduplicate_choices(choices: &mut Vec<ColoriChoice>, state: &GameState) {
    let mut seen: Vec<AbstractChoice> = Vec::with_capacity(choices.len());
    choices.retain(|choice| {
        let abs = abstract_choice(choice, state);
        if seen.contains(&abs) {
            false
        } else {
            seen.push(abs);
            true
        }
    });
}

// ── Game status ──

#[derive(Debug)]
pub enum GameStatus {
    AwaitingAction { player_id: usize },
    Terminated { scores: SmallVec<[f64; 4]> },
}

pub fn get_game_status(state: &GameState, max_round: Option<u32>) -> GameStatus {
    if let Some(mr) = max_round {
        if state.round > mr {
            let scores: SmallVec<[f64; 4]> = state.players.iter().map(|p| p.cached_score as f64).collect();
            let max_score = scores.iter().cloned().fold(0.0f64, f64::max);
            return GameStatus::Terminated {
                scores: scores
                    .iter()
                    .map(|&s| if s == max_score { 1.0 } else { 0.0 })
                    .collect(),
            };
        }
    }

    match &state.phase {
        GamePhase::Draft { draft_state } if !draft_state.waiting_for_pass => {
            GameStatus::AwaitingAction {
                player_id: draft_state.current_player_index,
            }
        }
        GamePhase::Action { action_state } => GameStatus::AwaitingAction {
            player_id: action_state.current_player_index,
        },
        GamePhase::Cleanup { cleanup_state } => GameStatus::AwaitingAction {
            player_id: cleanup_state.current_player_index,
        },
        GamePhase::GameOver => {
            let scores: SmallVec<[f64; 4]> = state.players.iter().map(|p| p.cached_score as f64).collect();
            let max_score = scores.iter().cloned().fold(0.0f64, f64::max);
            GameStatus::Terminated {
                scores: scores
                    .iter()
                    .map(|&s| if s == max_score { 1.0 } else { 0.0 })
                    .collect(),
            }
        }
        _ => GameStatus::AwaitingAction { player_id: 0 },
    }
}

// ── Determinization ──

pub fn determinize_in_place<R: Rng>(
    det: &mut GameState,
    source: &GameState,
    perspective_player: usize,
    seen_hands: &Option<Vec<Vec<CardInstance>>>,
    cached_scores: &[u32; MAX_PLAYERS],
    rng: &mut R,
) {
    det.clone_from(source);

    // Initialize cached scores from pre-computed values
    for (i, p) in det.players.iter_mut().enumerate() {
        p.cached_score = cached_scores[i];
    }

    if let GamePhase::Draft { ref mut draft_state } = det.phase {
        let num_players = det.players.len();
        let direction = draft_state.direction;

        // Determine which hands are known
        let mut known_hands = [false; 4];
        known_hands[perspective_player] = true;

        if let Some(ref sh) = seen_hands {
            // Track which drafted cards we've accounted for per player
            let mut persp_accounted = UnorderedCards::new();
            let mut receiver_accounted = [UnorderedCards::new(); MAX_PLAYERS];

            for round in 0..sh.len() {
                let hand = &sh[round];
                if hand.is_empty() {
                    continue;
                }

                // Convert seen_hands[round] to bitset
                let mut current_hand = UnorderedCards::new();
                for c in hand.iter() {
                    current_hand.insert(c.instance_id as u8);
                }
                let mut receiver = perspective_player;

                // Remove perspective player's pick at this round
                let persp_drafted = source.players[perspective_player].drafted_cards;
                let pick_mask = current_hand.intersection(persp_drafted).difference(persp_accounted);
                if let Some(persp_pick) = pick_mask.iter().next() {
                    persp_accounted.insert(persp_pick);
                    current_hand.remove(persp_pick);
                } else {
                    continue;
                }

                // Chain through subsequent players
                for step in 0..(num_players - 1) {
                    receiver = ((receiver as i32 + direction) as usize + num_players) % num_players;
                    if receiver == perspective_player {
                        break;
                    }

                    let pick_round = round + step + 1;
                    if pick_round > draft_state.pick_number as usize {
                        break;
                    }

                    if pick_round >= draft_state.pick_number as usize
                        && draft_state.current_player_index <= receiver
                    {
                        break;
                    }

                    let recv_drafted = source.players[receiver].drafted_cards;
                    let recv_pick_mask = current_hand
                        .intersection(recv_drafted)
                        .difference(receiver_accounted[receiver]);
                    if let Some(recv_pick) = recv_pick_mask.iter().next() {
                        receiver_accounted[receiver].insert(recv_pick);
                        current_hand.remove(recv_pick);
                        known_hands[receiver] = true;
                    } else {
                        break;
                    }
                }
            }
        }

        // Record hand sizes before pooling unknown hands
        let mut hand_sizes = [0u32; 4];
        for i in 0..num_players {
            hand_sizes[i] = draft_state.hands[i].len();
        }

        // Pool cards from unknown hands, redistribute via random draw
        let mut pool = UnorderedCards::new();
        let mut unknown_players = [0usize; 4];
        let mut unknown_count = 0usize;
        for i in 0..num_players {
            if !known_hands[i] {
                unknown_players[unknown_count] = i;
                unknown_count += 1;
                pool = pool.union(draft_state.hands[i]);
                draft_state.hands[i] = UnorderedCards::new();
            }
        }

        if unknown_count > 0 {
            for k in 0..unknown_count {
                let pi = unknown_players[k];
                let size = hand_sizes[pi];
                draft_state.hands[pi] = pool.draw_multiple(size, &mut *rng);
            }
        }

        // No shuffle calls needed - bitset draw is already uniform random
    }
    // No shuffle calls needed for player decks, buyer_deck, or draft_deck
    // because draw() from bitsets is inherently random
}

// ── Rollout draw+draft shortcut ──

fn rollout_draw_and_draft<R: Rng>(state: &mut GameState, rng: &mut R) {
    let num_players = state.players.len();

    // Step 1: Draw up to 5 cards from each player's personal deck (same as execute_draw_phase)
    for i in 0..num_players {
        let player = &mut state.players[i];
        let current = player.workshop_cards.len() as usize;
        let to_draw = if current >= 5 { 0 } else { 5 - current };
        if to_draw > 0 {
            draw_from_deck(
                &mut player.deck,
                &mut player.discard,
                &mut player.workshop_cards,
                to_draw,
                rng,
            );
        }
    }

    // Step 2: Pool all available draft cards
    let mut pool = state.draft_deck;
    state.draft_deck = UnorderedCards::new();

    let total_needed = 5 * num_players as u32;
    if pool.len() < total_needed && !state.destroyed_pile.is_empty() {
        pool = pool.union(state.destroyed_pile);
        state.destroyed_pile = UnorderedCards::new();
    }

    // Step 3: Check if enough cards for all players
    if pool.len() < 4 * num_players as u32 {
        state.destroyed_pile = state.destroyed_pile.union(pool);
        initialize_action_phase(state);
        return;
    }

    // Step 4: Distribute 4 random cards per player to drafted_cards
    for i in 0..num_players {
        let drawn = pool.draw_multiple(4, rng);
        state.players[i].drafted_cards = drawn;
    }

    // Step 5: Remaining cards go to destroyed_pile
    state.destroyed_pile = state.destroyed_pile.union(pool);

    // Step 6: Go directly to action phase
    initialize_action_phase(state);
}

// ── Fused rollout step ──

pub fn apply_rollout_step<R: Rng>(state: &mut GameState, rng: &mut R) {
    fn random_mix_seq_inline<R2: Rng>(
        wheel: &ColorWheel,
        remaining: u32,
        rng: &mut R2,
    ) -> ([(Color, Color); 2], usize) {
        let mut mixes = [(Color::Red, Color::Red); 2];
        let mut count = 0usize;
        let mut sim_wheel = wheel.clone();
        for _ in 0..remaining {
            if count >= 2 {
                break;
            }
            let mut pairs: [(Color, Color); 9] = [(Color::Red, Color::Red); 9];
            let mut pair_count = 0usize;
            for &(a, b) in &VALID_MIX_PAIRS {
                if sim_wheel.get(a) > 0 && sim_wheel.get(b) > 0 {
                    pairs[pair_count] = (a, b);
                    pair_count += 1;
                }
            }
            if pair_count == 0 {
                break;
            }
            let target = rng.random_range(0..pair_count + 1);
            if target == pair_count {
                break;
            }
            let (a, b) = pairs[target];
            mixes[count] = (a, b);
            count += 1;
            perform_mix_unchecked(&mut sim_wheel, a, b);
        }
        (mixes, count)
    }

    // Small discriminant constants to avoid constructing the large Op enum
    const DESTROY_DRAFTED: u8 = 1;
    const DESTROY_AND_MIX: u8 = 2;
    const DESTROY_AND_SELL: u8 = 3;
    const END_TURN: u8 = 4;
    const SKIP_WORKSHOP: u8 = 5;
    const WORKSHOP: u8 = 6;
    const DESTROY_DRAWN: u8 = 7;
    const MIX_ALL: u8 = 8;
    const SELECT_BUYER: u8 = 9;
    const GAIN_SECONDARY: u8 = 10;
    const GAIN_PRIMARY: u8 = 11;
    const SWAP_TERTIARY: u8 = 12;
    const KEEP_ALL_WORKSHOP: u8 = 13;

    // Flat locals for extracted data (avoids constructing a large tagged union)
    let disc: u8;
    let mut id1: u32 = 0;
    let mut id2: u32 = 0;
    let mut mixes = [(Color::Red, Color::Red); 2];
    let mut mix_count: usize = 0;
    let mut selected = UnorderedCards::new();
    let mut color1 = Color::Red;
    let mut color2 = Color::Red;

    // Fast path: complete entire draft in one step
    if matches!(&state.phase, GamePhase::Draft { .. }) {
        if let GamePhase::Draft { ref mut draft_state } = state.phase {
            draft_state.waiting_for_pass = false;
        }
        loop {
            let card_id = {
                if let GamePhase::Draft { ref draft_state } = state.phase {
                    let hand = draft_state.hands[draft_state.current_player_index];
                    hand.pick_random(rng).unwrap() as u32
                } else {
                    break;
                }
            };
            player_pick(state, card_id);
            if let GamePhase::Draft { ref mut draft_state } = state.phase {
                draft_state.waiting_for_pass = false;
            }
        }
        return;
    }

    match &state.phase {
        GamePhase::Action { action_state } => {
            let player = &state.players[action_state.current_player_index];
            match &action_state.pending_choice {
                None => {
                    let mut copy = player.drafted_cards;
                    let sel = copy.draw_up_to(1, rng);
                    if !sel.is_empty() {
                        let card_id = sel.lowest_bit().unwrap();
                        id1 = card_id as u32;
                        let card = state.card_lookup[card_id as usize];
                        match card.ability() {
                            Ability::MixColors { count } => {
                                let (m, mc) =
                                    random_mix_seq_inline(&player.color_wheel, count, rng);
                                mixes = m;
                                mix_count = mc;
                                disc = DESTROY_AND_MIX;
                            }
                            Ability::Sell => {
                                let mut affordable = [0u32; 6];
                                let mut aff_count = 0usize;
                                for g in &state.buyer_display {
                                    if can_afford_buyer(player, &g.buyer) {
                                        affordable[aff_count] = g.instance_id;
                                        aff_count += 1;
                                    }
                                }
                                if aff_count > 0 {
                                    let buyer_idx = rng.random_range(0..aff_count);
                                    id2 = affordable[buyer_idx];
                                    disc = DESTROY_AND_SELL;
                                } else {
                                    disc = DESTROY_DRAFTED;
                                }
                            }
                            _ => {
                                disc = DESTROY_DRAFTED;
                            }
                        }
                    } else {
                        disc = END_TURN;
                    }
                }
                Some(PendingChoice::ChooseCardsForWorkshop { count }) => {
                    let mut copy = player.workshop_cards;
                    selected = copy.draw_up_to(*count as u8, rng);
                    disc = if selected.is_empty() { SKIP_WORKSHOP } else { WORKSHOP };
                }
                Some(PendingChoice::ChooseCardsToDestroy) => {
                    let mut copy = player.workshop_cards;
                    selected = copy.draw_up_to(1, rng);
                    disc = DESTROY_DRAWN;
                }
                Some(PendingChoice::ChooseMix { remaining }) => {
                    let (m, mc) =
                        random_mix_seq_inline(&player.color_wheel, *remaining, rng);
                    mixes = m;
                    mix_count = mc;
                    disc = MIX_ALL;
                }
                Some(PendingChoice::ChooseBuyer) => {
                    let mut affordable = [0u32; 6];
                    let mut aff_count = 0usize;
                    for g in &state.buyer_display {
                        if can_afford_buyer(player, &g.buyer) {
                            affordable[aff_count] = g.instance_id;
                            aff_count += 1;
                        }
                    }
                    let idx = rng.random_range(0..aff_count);
                    id1 = affordable[idx];
                    disc = SELECT_BUYER;
                }
                Some(PendingChoice::ChooseSecondaryColor) => {
                    let idx = rng.random_range(0..SECONDARIES.len());
                    color1 = SECONDARIES[idx];
                    disc = GAIN_SECONDARY;
                }
                Some(PendingChoice::ChoosePrimaryColor) => {
                    let idx = rng.random_range(0..PRIMARIES.len());
                    color1 = PRIMARIES[idx];
                    disc = GAIN_PRIMARY;
                }
                Some(PendingChoice::ChooseTertiaryToLose) => {
                    let mut owned = [Color::Red; 6];
                    let mut own_count = 0usize;
                    for &c in &TERTIARIES {
                        if player.color_wheel.get(c) > 0 {
                            owned[own_count] = c;
                            own_count += 1;
                        }
                    }
                    let r = rng.random_range(0..own_count * 5);
                    let lose_idx = r / 5;
                    let gain_local_idx = r % 5;
                    color1 = owned[lose_idx];
                    let mut options = [Color::Red; 6];
                    let mut opt_count = 0usize;
                    for &c in &TERTIARIES {
                        if c != color1 {
                            options[opt_count] = c;
                            opt_count += 1;
                        }
                    }
                    color2 = options[gain_local_idx];
                    disc = SWAP_TERTIARY;
                }
            }
        }
        GamePhase::Cleanup { cleanup_state } => {
            // Rollout policy: keep all workshop cards
            selected = state.players[cleanup_state.current_player_index].workshop_cards;
            disc = KEEP_ALL_WORKSHOP;
        }
        _ => panic!("Cannot apply rollout step for current state"),
    }

    match disc {
        DESTROY_DRAFTED => destroy_drafted_card(state, id1, rng),
        DESTROY_AND_MIX => {
            // Fused: ability stack is guaranteed empty when pending_choice is None,
            // so we can skip all process_queue calls.
            let player_index = match &state.phase {
                GamePhase::Action { action_state } => action_state.current_player_index,
                _ => unreachable!(),
            };
            state.players[player_index].drafted_cards.remove(id1 as u8);
            state.destroyed_pile.insert(id1 as u8);
            for i in 0..mix_count {
                let (a, b) = mixes[i];
                perform_mix_unchecked(&mut state.players[player_index].color_wheel, a, b);
            }
        }
        DESTROY_AND_SELL => {
            // Fused: ability stack is guaranteed empty when pending_choice is None,
            // so we can skip all process_queue calls.
            let player_index = match &state.phase {
                GamePhase::Action { action_state } => action_state.current_player_index,
                _ => unreachable!(),
            };
            state.players[player_index].drafted_cards.remove(id1 as u8);
            state.destroyed_pile.insert(id1 as u8);
            let buyer_index = state.buyer_display.iter()
                .position(|c| c.instance_id == id2)
                .unwrap();
            let buyer = state.buyer_display.swap_remove(buyer_index);
            let player = &mut state.players[player_index];
            player.materials.decrement(buyer.buyer.required_material());
            pay_cost(&mut player.color_wheel, buyer.buyer.color_cost());
            player.cached_score += buyer.buyer.stars();
            player.completed_buyers.push(buyer);
            if let Some(id) = state.buyer_deck.draw(rng) {
                state.buyer_display.push(BuyerInstance {
                    instance_id: id as u32,
                    buyer: state.buyer_lookup[id as usize],
                });
            }
        }
        END_TURN => {
            end_player_turn(state, rng);
            if matches!(state.phase, GamePhase::Cleanup { .. }) {
                // Rollout policy: keep all workshop cards for each player
                while matches!(state.phase, GamePhase::Cleanup { .. }) {
                    if let GamePhase::Cleanup { ref cleanup_state } = state.phase {
                        let keep = state.players[cleanup_state.current_player_index].workshop_cards;
                        resolve_keep_workshop_cards(state, keep, rng);
                    }
                }
            }
            if matches!(state.phase, GamePhase::Draw) {
                rollout_draw_and_draft(state, rng);
            }
        }
        SKIP_WORKSHOP => skip_workshop(state, rng),
        WORKSHOP => resolve_workshop_choice(state, selected, rng),
        DESTROY_DRAWN => resolve_destroy_cards(state, selected, rng),
        MIX_ALL => {
            // Fused: apply all mixes directly, then process_queue once.
            // Ability stack may not be empty here, so we must call process_queue.
            let player_index = match &state.phase {
                GamePhase::Action { action_state } => action_state.current_player_index,
                _ => unreachable!(),
            };
            for i in 0..mix_count {
                let (a, b) = mixes[i];
                perform_mix_unchecked(&mut state.players[player_index].color_wheel, a, b);
            }
            if let GamePhase::Action { ref mut action_state } = state.phase {
                action_state.pending_choice = None;
            }
            process_queue(state, rng);
        }
        SELECT_BUYER => resolve_select_buyer(state, id1, rng),
        GAIN_SECONDARY => resolve_gain_secondary(state, color1, rng),
        GAIN_PRIMARY => resolve_gain_primary(state, color1, rng),
        SWAP_TERTIARY => {
            resolve_choose_tertiary_to_lose(state, color1);
            resolve_choose_tertiary_to_gain(state, color2, rng);
        }
        KEEP_ALL_WORKSHOP => {
            resolve_keep_workshop_cards(state, selected, rng);
            if matches!(state.phase, GamePhase::Draw) {
                rollout_draw_and_draft(state, rng);
            }
        }
        _ => unreachable!(),
    }
}
