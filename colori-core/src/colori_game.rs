use crate::apply_choice::apply_choice;
use crate::color_wheel::can_pay_cost;
use crate::colors::{can_mix, PRIMARIES, PRIMARIES_MASK, SECONDARIES, SECONDARIES_MASK, TERTIARIES, TERTIARIES_MASK};
use crate::draft_phase::confirm_pass;
use crate::draw_phase::execute_draw_phase;
use crate::scoring::calculate_score;
use crate::types::*;
use rand::Rng;
use smallvec::SmallVec;

// ── Buyer affordability ──

fn can_afford_buyer(player: &PlayerState, buyer: &BuyerCard) -> bool {
    player.materials.get(buyer.required_material()) >= 1
        && can_pay_cost(&player.color_wheel, buyer.color_cost())
}

// ── Choice enumeration ──

pub fn enumerate_choices(state: &GameState) -> Vec<ColoriChoice> {
    match &state.phase {
        GamePhase::Draft { draft_state } => {
            if draft_state.waiting_for_pass {
                return vec![];
            }
            let hand = &draft_state.hands[draft_state.current_player_index];
            hand.iter()
                .map(|c| ColoriChoice::DraftPick {
                    card_instance_id: c.instance_id,
                })
                .collect()
        }
        GamePhase::Action { action_state } => {
            let player = &state.players[action_state.current_player_index];
            let pending = &action_state.pending_choice;

            match pending {
                None => {
                    let mut choices: Vec<ColoriChoice> = player
                        .drafted_cards
                        .iter()
                        .map(|c| ColoriChoice::DestroyDraftedCard {
                            card_instance_id: c.instance_id,
                        })
                        .collect();
                    choices.push(ColoriChoice::EndTurn);
                    choices
                }
                Some(PendingChoice::ChooseCardsForWorkshop { count }) => {
                    let mut choices: Vec<ColoriChoice> = vec![ColoriChoice::SkipWorkshop];

                    let mut workshop_mask = 0u128;
                    for c in &player.workshop_cards {
                        workshop_mask |= 1u128 << (c.instance_id - 1);
                    }

                    let mut sub = workshop_mask;
                    while sub != 0 {
                        if (sub.count_ones() as u32) <= *count {
                            choices.push(ColoriChoice::Workshop { card_mask: sub });
                        }
                        sub = (sub - 1) & workshop_mask;
                    }

                    choices
                }
                Some(PendingChoice::ChooseCardsToDestroy { count }) => {
                    let mut workshop_mask = 0u128;
                    for c in &player.workshop_cards {
                        workshop_mask |= 1u128 << (c.instance_id - 1);
                    }
                    if workshop_mask == 0 {
                        return vec![ColoriChoice::DestroyDrawnCards { card_mask: 0 }];
                    }
                    let mut choices = Vec::new();
                    let mut sub = workshop_mask;
                    while sub != 0 {
                        if (sub.count_ones() as u32) <= *count {
                            choices.push(ColoriChoice::DestroyDrawnCards { card_mask: sub });
                        }
                        sub = (sub - 1) & workshop_mask;
                    }
                    choices
                }
                Some(PendingChoice::ChooseMix { .. }) => {
                    let mut choices: Vec<ColoriChoice> = vec![ColoriChoice::SkipMix];
                    for i in 0..ALL_COLORS.len() {
                        for j in (i + 1)..ALL_COLORS.len() {
                            let a = ALL_COLORS[i];
                            let b = ALL_COLORS[j];
                            if player.color_wheel.get(a) > 0
                                && player.color_wheel.get(b) > 0
                                && can_mix(a, b)
                            {
                                choices.push(ColoriChoice::Mix {
                                    color_a: a,
                                    color_b: b,
                                });
                            }
                        }
                    }
                    choices
                }
                Some(PendingChoice::ChooseBuyer) => state
                    .buyer_display
                    .iter()
                    .filter(|g| can_afford_buyer(player, &g.buyer))
                    .map(|g| ColoriChoice::SelectBuyer {
                        buyer_instance_id: g.instance_id,
                    })
                    .collect(),
                Some(PendingChoice::ChooseSecondaryColor) => SECONDARIES
                    .iter()
                    .map(|&c| ColoriChoice::GainSecondary { color: c })
                    .collect(),
                Some(PendingChoice::ChoosePrimaryColor) => PRIMARIES
                    .iter()
                    .map(|&c| ColoriChoice::GainPrimary { color: c })
                    .collect(),
                Some(PendingChoice::ChooseTertiaryToLose) => TERTIARIES
                    .iter()
                    .filter(|&&c| player.color_wheel.get(c) > 0)
                    .map(|&c| ColoriChoice::ChooseTertiaryToLose { color: c })
                    .collect(),
                Some(PendingChoice::ChooseTertiaryToGain { lost_color }) => {
                    let lost = *lost_color;
                    TERTIARIES
                        .iter()
                        .filter(|&&c| c != lost)
                        .map(|&c| ColoriChoice::ChooseTertiaryToGain { color: c })
                        .collect()
                }
            }
        }
        _ => vec![],
    }
}

// ── Apply choice with AI post-processing ──

pub fn apply_choice_to_state<R: Rng>(state: &mut GameState, choice: &ColoriChoice, rng: &mut R) {
    apply_choice(state, choice, rng);

    // AI-specific post-processing
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
}

// ── Choice availability ──

pub fn check_choice_available(state: &GameState, choice: &ColoriChoice) -> bool {
    match choice {
        ColoriChoice::DraftPick { card_instance_id } => {
            if let GamePhase::Draft { ref draft_state } = state.phase {
                draft_state.hands[draft_state.current_player_index]
                    .iter()
                    .any(|c| c.instance_id == *card_instance_id)
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
                    .iter()
                    .any(|c| c.instance_id == *card_instance_id)
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
        ColoriChoice::Workshop { card_mask } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match &action_state.pending_choice {
                    Some(PendingChoice::ChooseCardsForWorkshop { .. }) => {
                        let player = &state.players[action_state.current_player_index];
                        let mut workshop_mask = 0u128;
                        for c in &player.workshop_cards {
                            workshop_mask |= 1u128 << (c.instance_id - 1);
                        }
                        card_mask & workshop_mask == *card_mask && *card_mask != 0
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
        ColoriChoice::DestroyDrawnCards { card_mask } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match &action_state.pending_choice {
                    Some(PendingChoice::ChooseCardsToDestroy { .. }) => {
                        let player = &state.players[action_state.current_player_index];
                        let mut workshop_mask = 0u128;
                        for c in &player.workshop_cards {
                            workshop_mask |= 1u128 << (c.instance_id - 1);
                        }
                        card_mask & workshop_mask == *card_mask
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
        ColoriChoice::Mix { color_a, color_b } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match &action_state.pending_choice {
                    Some(PendingChoice::ChooseMix { .. }) => {
                        let player = &state.players[action_state.current_player_index];
                        player.color_wheel.get(*color_a) > 0
                            && player.color_wheel.get(*color_b) > 0
                            && can_mix(*color_a, *color_b)
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
        ColoriChoice::SkipMix => {
            if let GamePhase::Action { ref action_state } = state.phase {
                matches!(
                    action_state.pending_choice,
                    Some(PendingChoice::ChooseMix { .. })
                )
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
                ) && SECONDARIES_MASK & (1 << color.index()) != 0
            } else {
                false
            }
        }
        ColoriChoice::GainPrimary { color } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                matches!(
                    action_state.pending_choice,
                    Some(PendingChoice::ChoosePrimaryColor)
                ) && PRIMARIES_MASK & (1 << color.index()) != 0
            } else {
                false
            }
        }
        ColoriChoice::ChooseTertiaryToLose { color } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match &action_state.pending_choice {
                    Some(PendingChoice::ChooseTertiaryToLose) => {
                        let player = &state.players[action_state.current_player_index];
                        TERTIARIES_MASK & (1 << color.index()) != 0 && player.color_wheel.get(*color) > 0
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
        ColoriChoice::ChooseTertiaryToGain { color } => {
            if let GamePhase::Action { ref action_state } = state.phase {
                match &action_state.pending_choice {
                    Some(PendingChoice::ChooseTertiaryToGain { lost_color }) => {
                        TERTIARIES_MASK & (1 << color.index()) != 0 && *color != *lost_color
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
    }
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
            let scores: SmallVec<[f64; 4]> = state.players.iter().map(|p| calculate_score(p) as f64).collect();
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
        GamePhase::GameOver => {
            let scores: SmallVec<[f64; 4]> = state.players.iter().map(|p| calculate_score(p) as f64).collect();
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
    rng: &mut R,
) {
    det.clone_from(source);

    if let GamePhase::Draft { ref mut draft_state } = det.phase {
        let num_players = det.players.len();
        let direction = draft_state.direction;

        // Determine which hands are known
        let mut known_hands = [false; 4];
        known_hands[perspective_player] = true;

        if let Some(ref sh) = seen_hands {
            for round in 0..sh.len() {
                let hand = &sh[round];
                if hand.is_empty() {
                    continue;
                }

                let mut current_cards: Vec<u32> =
                    hand.iter().map(|c| c.instance_id).collect();
                let mut receiver = perspective_player;

                // Remove what perspective player picked at this round
                if round < source.players[perspective_player].drafted_cards.len() {
                    let persp_pick =
                        source.players[perspective_player].drafted_cards[round].instance_id;
                    current_cards.retain(|&id| id != persp_pick);
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

                    if pick_round < source.players[receiver].drafted_cards.len() {
                        let receiver_pick =
                            source.players[receiver].drafted_cards[pick_round].instance_id;
                        if current_cards.contains(&receiver_pick) {
                            known_hands[receiver] = true;
                            current_cards.retain(|&id| id != receiver_pick);
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        }

        // Record hand sizes before draining unknown hands
        let mut hand_sizes = [0usize; 4];
        for i in 0..num_players {
            hand_sizes[i] = draft_state.hands[i].len();
        }

        // Pool cards from unknown hands, shuffle, redistribute
        let mut unknown_players = [0usize; 4];
        let mut unknown_count = 0usize;
        let mut pool: Vec<CardInstance> = Vec::new();
        for i in 0..num_players {
            if !known_hands[i] {
                unknown_players[unknown_count] = i;
                unknown_count += 1;
                pool.extend(draft_state.hands[i].drain(..));
            }
        }

        if unknown_count > 0 {
            crate::deck_utils::shuffle_in_place(&mut pool, rng);
            let mut idx = 0;
            for k in 0..unknown_count {
                let pi = unknown_players[k];
                let size = hand_sizes[pi];
                draft_state.hands[pi].extend_from_slice(&pool[idx..idx + size]);
                idx += size;
            }
        }

        // Shuffle hidden decks
        crate::deck_utils::shuffle_in_place(&mut det.draft_deck, rng);
        crate::deck_utils::shuffle_in_place(&mut det.buyer_deck, rng);
        for p in &mut det.players {
            crate::deck_utils::shuffle_in_place(&mut p.deck, rng);
        }
    } else {
        // Outside draft: only shuffle hidden decks
        crate::deck_utils::shuffle_in_place(&mut det.draft_deck, rng);
        crate::deck_utils::shuffle_in_place(&mut det.buyer_deck, rng);
        for p in &mut det.players {
            crate::deck_utils::shuffle_in_place(&mut p.deck, rng);
        }
    }
}

// ── Rollout policy ──

pub fn get_rollout_choice<R: Rng>(state: &GameState, rng: &mut R) -> ColoriChoice {
    match &state.phase {
        GamePhase::Draft { draft_state } => {
            let hand = &draft_state.hands[draft_state.current_player_index];
            let idx = rng.random_range(0..hand.len());
            ColoriChoice::DraftPick {
                card_instance_id: hand[idx].instance_id,
            }
        }
        GamePhase::Action { action_state } => {
            let player = &state.players[action_state.current_player_index];
            let pending = &action_state.pending_choice;

            match pending {
                None => {
                    if !player.drafted_cards.is_empty() && rng.random::<f64>() < 0.8 {
                        let idx = rng.random_range(0..player.drafted_cards.len());
                        ColoriChoice::DestroyDraftedCard {
                            card_instance_id: player.drafted_cards[idx].instance_id,
                        }
                    } else {
                        ColoriChoice::EndTurn
                    }
                }
                Some(PendingChoice::ChooseCardsForWorkshop { count }) => {
                    let cards = &player.workshop_cards;
                    let total = cards.len();

                    if total == 0 {
                        return ColoriChoice::SkipWorkshop;
                    }

                    if rng.random::<f64>() < 0.2 {
                        return ColoriChoice::SkipWorkshop;
                    }

                    let max_pick = (*count as usize).min(total);
                    let pick = rng.random_range(1..=max_pick);
                    let mut indices = [0usize; 16];
                    for k in 0..total {
                        indices[k] = k;
                    }

                    for k in 0..pick {
                        let j = k + rng.random_range(0..(total - k));
                        indices.swap(k, j);
                    }

                    let mut mask = 0u128;
                    for k in 0..pick {
                        mask |= 1u128 << (cards[indices[k]].instance_id - 1);
                    }
                    ColoriChoice::Workshop { card_mask: mask }
                }
                Some(PendingChoice::ChooseCardsToDestroy { count }) => {
                    let ws_cards = &player.workshop_cards;
                    let ws_len = ws_cards.len();
                    let destroy_count = (*count as usize).min(ws_len);
                    if destroy_count == 0 {
                        return ColoriChoice::DestroyDrawnCards { card_mask: 0 };
                    }
                    let destroy_pick = rng.random_range(1..=destroy_count);
                    let mut ws_indices = [0usize; 16];
                    for k in 0..ws_len {
                        ws_indices[k] = k;
                    }
                    for k in 0..destroy_pick {
                        let j = k + rng.random_range(0..(ws_len - k));
                        ws_indices.swap(k, j);
                    }
                    let mut mask = 0u128;
                    for k in 0..destroy_pick {
                        mask |= 1u128 << (ws_cards[ws_indices[k]].instance_id - 1);
                    }
                    ColoriChoice::DestroyDrawnCards { card_mask: mask }
                }
                Some(PendingChoice::ChooseMix { .. }) => {
                    if rng.random::<f64>() < 0.5 {
                        return ColoriChoice::SkipMix;
                    }
                    let mut pairs: [(Color, Color); 9] = [(Color::Red, Color::Red); 9];
                    let mut pair_count = 0usize;
                    for i in 0..ALL_COLORS.len() {
                        for j in (i + 1)..ALL_COLORS.len() {
                            if player.color_wheel.get(ALL_COLORS[i]) > 0
                                && player.color_wheel.get(ALL_COLORS[j]) > 0
                                && can_mix(ALL_COLORS[i], ALL_COLORS[j])
                            {
                                pairs[pair_count] = (ALL_COLORS[i], ALL_COLORS[j]);
                                pair_count += 1;
                            }
                        }
                    }
                    if pair_count == 0 {
                        return ColoriChoice::SkipMix;
                    }
                    let target = rng.random_range(0..pair_count);
                    ColoriChoice::Mix { color_a: pairs[target].0, color_b: pairs[target].1 }
                }
                Some(PendingChoice::ChooseBuyer) => {
                    let mut affordable = [0u32; 6];
                    let mut count = 0usize;
                    for g in &state.buyer_display {
                        if can_afford_buyer(player, &g.buyer) {
                            affordable[count] = g.instance_id;
                            count += 1;
                        }
                    }
                    let idx = rng.random_range(0..count);
                    ColoriChoice::SelectBuyer {
                        buyer_instance_id: affordable[idx],
                    }
                }
                Some(PendingChoice::ChooseSecondaryColor) => {
                    let idx = rng.random_range(0..SECONDARIES.len());
                    ColoriChoice::GainSecondary {
                        color: SECONDARIES[idx],
                    }
                }
                Some(PendingChoice::ChoosePrimaryColor) => {
                    let idx = rng.random_range(0..PRIMARIES.len());
                    ColoriChoice::GainPrimary {
                        color: PRIMARIES[idx],
                    }
                }
                Some(PendingChoice::ChooseTertiaryToLose) => {
                    let mut owned = [Color::Red; 6];
                    let mut count = 0usize;
                    for &c in &TERTIARIES {
                        if player.color_wheel.get(c) > 0 {
                            owned[count] = c;
                            count += 1;
                        }
                    }
                    let idx = rng.random_range(0..count);
                    ColoriChoice::ChooseTertiaryToLose { color: owned[idx] }
                }
                Some(PendingChoice::ChooseTertiaryToGain { lost_color }) => {
                    let mut options = [Color::Red; 6];
                    let mut count = 0usize;
                    for &c in &TERTIARIES {
                        if c != *lost_color {
                            options[count] = c;
                            count += 1;
                        }
                    }
                    let idx = rng.random_range(0..count);
                    ColoriChoice::ChooseTertiaryToGain {
                        color: options[idx],
                    }
                }
            }
        }
        _ => panic!("Cannot get rollout choice for current state"),
    }
}
