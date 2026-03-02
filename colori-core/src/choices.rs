use crate::action_phase::can_sell_to_any_buyer;
use crate::colors::{can_mix, perform_mix_unchecked, PRIMARIES, SECONDARIES, TERTIARIES, VALID_MIX_PAIRS};
use crate::types::*;
use crate::unordered_cards::UnorderedCards;
use smallvec::SmallVec;

// ── Multiset subset enumeration ──

/// Count occurrences of each card type in an UnorderedCards bitset,
/// returning a sorted array of (Card, count) pairs and the number of distinct types.
fn count_card_types(
    mask: UnorderedCards,
    card_lookup: &[Card; 256],
) -> ([Card; 49], [u8; 49], usize) {
    let mut types = [Card::BasicRed; 49];
    let mut counts = [0u8; 49];
    let mut len = 0usize;
    let mut seen: u64 = 0;
    for id in mask.iter() {
        let card = card_lookup[id as usize];
        let bit = 1u64 << (card as u64);
        if seen & bit == 0 {
            seen |= bit;
            types[len] = card;
            counts[len] = 1;
            len += 1;
        } else {
            for i in 0..len {
                if types[i] == card {
                    counts[i] += 1;
                    break;
                }
            }
        }
    }
    // Sort by card discriminant for deterministic output
    for i in 1..len {
        let mut j = i;
        while j > 0 && (types[j] as usize) < (types[j - 1] as usize) {
            types.swap(j, j - 1);
            counts.swap(j, j - 1);
            j -= 1;
        }
    }
    (types, counts, len)
}

/// Enumerate all non-empty subsets of a card-type multiset up to max_size.
/// Produces unique sorted SmallVec<[Card; 4]> entries without needing deduplication.
fn enumerate_multiset_subsets(
    types: &[Card],
    counts: &[u8],
    max_remaining: usize,
    current: &mut SmallVec<[Card; 4]>,
    choices: &mut Vec<Choice>,
    make_choice: &impl Fn(SmallVec<[Card; 4]>) -> Choice,
) {
    if types.is_empty() || max_remaining == 0 {
        if !current.is_empty() {
            choices.push(make_choice(current.clone()));
        }
        return;
    }
    let card = types[0];
    let count = counts[0] as usize;
    let max_take = max_remaining.min(count);
    let base_len = current.len();
    for take in 0..=max_take {
        enumerate_multiset_subsets(
            &types[1..],
            &counts[1..],
            max_remaining - take,
            current,
            choices,
            make_choice,
        );
        current.push(card);
    }
    current.truncate(base_len);
}

// ── Buyer affordability ──

#[inline]
pub(crate) fn can_afford_buyer(player: &PlayerState, buyer: &BuyerCard) -> bool {
    player.materials.get(buyer.required_material()) >= 1
        && crate::colors::can_pay_cost(&player.color_wheel, buyer.color_cost())
}

// ── Mix sequence enumeration ──

fn enumerate_mix_sequences<F>(
    wheel: &ColorWheel,
    remaining: u32,
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

pub fn enumerate_choices_into(state: &GameState, choices: &mut Vec<Choice>) {
    choices.clear();
    match &state.phase {
        GamePhase::Draft { draft_state } => {
            if draft_state.waiting_for_pass {
                return;
            }
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
            let pending = &action_state.pending_choice;

            match pending {
                None => {
                    let mut seen: u64 = 0;
                    for id in player.drafted_cards.iter() {
                        let card = state.card_lookup[id as usize];
                        let bit = 1u64 << (card as u64);
                        if seen & bit != 0 { continue; }
                        seen |= bit;
                        match card.ability() {
                            Ability::MixColors { count } => {
                                enumerate_mix_sequences(
                                    &player.color_wheel,
                                    count,
                                    choices,
                                    |mixes| Choice::DestroyAndMixAll {
                                        card,
                                        mixes,
                                    },
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
                                if !has_buyer {
                                    choices.push(Choice::DestroyDraftedCard { card });
                                }
                            }
                            _ => {
                                choices.push(Choice::DestroyDraftedCard { card });
                            }
                        }
                    }
                    choices.push(Choice::EndTurn);
                }
                Some(PendingChoice::ChooseCardsForWorkshop { remaining_picks }) => {
                    choices.push(Choice::SkipWorkshop);
                    let (types, counts, len) = count_card_types(player.workshop_cards, &state.card_lookup);
                    enumerate_multiset_subsets(
                        &types[..len],
                        &counts[..len],
                        *remaining_picks as usize,
                        &mut SmallVec::new(),
                        choices,
                        &|card_types| Choice::Workshop { card_types },
                    );
                }
                Some(PendingChoice::ChooseCardsToDestroy) => {
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
                Some(PendingChoice::ChooseMix { remaining }) => {
                    enumerate_mix_sequences(
                        &player.color_wheel,
                        *remaining,
                        choices,
                        |mixes| Choice::MixAll { mixes },
                    );
                }
                Some(PendingChoice::ChooseBuyer) => {
                    for buyer in state.buyer_display.iter() {
                        if can_afford_buyer(player, &buyer.buyer) {
                            choices.push(Choice::SelectBuyer {
                                buyer: buyer.buyer,
                            });
                        }
                    }
                }
                Some(PendingChoice::ChooseSecondaryColor) => {
                    for &c in SECONDARIES.iter() {
                        choices.push(Choice::GainSecondary { color: c });
                    }
                }
                Some(PendingChoice::ChoosePrimaryColor) => {
                    for &c in PRIMARIES.iter() {
                        choices.push(Choice::GainPrimary { color: c });
                    }
                }
                Some(PendingChoice::ChooseTertiaryToLose) => {
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
            }
        }
        GamePhase::Cleanup { cleanup_state } => {
            let player = &state.players[cleanup_state.current_player_index];
            // Empty set (discard all)
            choices.push(Choice::KeepWorkshopCards {
                card_types: SmallVec::new(),
            });
            // All non-empty subsets via multiset enumeration
            let (types, counts, len) = count_card_types(player.workshop_cards, &state.card_lookup);
            let total = player.workshop_cards.len() as usize;
            enumerate_multiset_subsets(
                &types[..len],
                &counts[..len],
                total,
                &mut SmallVec::new(),
                choices,
                &|card_types| Choice::KeepWorkshopCards { card_types },
            );
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

pub fn check_choice_available(state: &GameState, choice: &Choice) -> bool {
    match choice {
        Choice::DraftPick { card } => {
            if let GamePhase::Draft { ref draft_state } = state.phase {
                if draft_state.waiting_for_pass {
                    return false;
                }
                let hand = draft_state.hands[draft_state.current_player_index];
                hand.iter().any(|id| state.card_lookup[id as usize] == *card)
            } else {
                false
            }
        }
        Choice::DestroyDraftedCard { card } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if action_state.pending_choice.is_some() {
                    return false;
                }
                let player = &state.players[action_state.current_player_index];
                let has_card = player.drafted_cards.iter().any(|id| state.card_lookup[id as usize] == *card);
                if !has_card {
                    return false;
                }
                match card.ability() {
                    Ability::Sell => !can_sell_to_any_buyer(state),
                    Ability::MixColors { .. } => false,
                    _ => true,
                }
            } else {
                false
            }
        }
        Choice::EndTurn => {
            if let GamePhase::Action { ref action_state } = state.phase {
                action_state.pending_choice.is_none()
            } else {
                false
            }
        }
        Choice::Workshop { card_types } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match &action_state.pending_choice {
                    Some(PendingChoice::ChooseCardsForWorkshop { .. }) => {
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
                    action_state.pending_choice,
                    Some(PendingChoice::ChooseCardsForWorkshop { .. })
                )
            } else {
                false
            }
        }
        Choice::DestroyDrawnCards { card } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match &action_state.pending_choice {
                    Some(PendingChoice::ChooseCardsToDestroy) => match card {
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
                match &action_state.pending_choice {
                    Some(PendingChoice::ChooseBuyer) => {
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
                    action_state.pending_choice,
                    Some(PendingChoice::ChooseSecondaryColor)
                ) && SECONDARIES.contains(color)
            } else {
                false
            }
        }
        Choice::GainPrimary { color } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                matches!(
                    action_state.pending_choice,
                    Some(PendingChoice::ChoosePrimaryColor)
                ) && PRIMARIES.contains(color)
            } else {
                false
            }
        }
        Choice::MixAll { mixes } => {
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
        Choice::SwapTertiary { lose, gain } => {
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
        Choice::DestroyAndMixAll { card, mixes } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if action_state.pending_choice.is_some() {
                    return false;
                }
                let player = &state.players[action_state.current_player_index];
                if !player.drafted_cards.iter().any(|id| state.card_lookup[id as usize] == *card) {
                    return false;
                }
                if mixes.is_empty() {
                    return true;
                }
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
            } else {
                false
            }
        }
        Choice::DestroyAndSell { card, buyer } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                if action_state.pending_choice.is_some() {
                    return false;
                }
                let player = &state.players[action_state.current_player_index];
                if !player.drafted_cards.iter().any(|id| state.card_lookup[id as usize] == *card) {
                    return false;
                }
                state.buyer_display.iter().any(|b| b.buyer == *buyer && can_afford_buyer(player, &b.buyer))
            } else {
                false
            }
        }
        Choice::KeepWorkshopCards { card_types } => {
            if let GamePhase::Cleanup { ref cleanup_state } = state.phase {
                if card_types.is_empty() {
                    return true;
                }
                let player = &state.players[cleanup_state.current_player_index];
                resolve_card_types_to_ids(card_types, &player.workshop_cards, &state.card_lookup).is_some()
            } else {
                false
            }
        }
    }
}

/// Resolve a list of card types to instance IDs from a card set.
/// Returns None if any card type can't be found.
fn resolve_card_types_to_ids(
    card_types: &[Card],
    available: &UnorderedCards,
    card_lookup: &[Card; 256],
) -> Option<UnorderedCards> {
    let mut ids = UnorderedCards::new();
    let mut used = UnorderedCards::new();
    for &ct in card_types.iter() {
        let mut found = false;
        for id in available.iter() {
            if !used.contains(id) && card_lookup[id as usize] == ct {
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
    Some(ids)
}
