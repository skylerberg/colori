use crate::action_phase::{
    destroy_drafted_card, end_player_turn, resolve_choose_tertiary_to_gain,
    resolve_choose_tertiary_to_lose, resolve_destroy_cards, resolve_gain_primary,
    resolve_gain_secondary, resolve_mix_colors, resolve_select_buyer, resolve_workshop_choice,
    skip_mix, skip_workshop,
};
use crate::apply_choice::apply_choice;
use crate::color_wheel::{can_pay_cost, perform_mix};
use crate::colors::{can_mix, PRIMARIES, SECONDARIES, TERTIARIES, VALID_MIX_PAIRS};
use crate::draft_phase::{confirm_pass, player_pick};
use crate::draw_phase::execute_draw_phase;
use crate::scoring::calculate_score;
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
    while sub > 0 {
        if (sub.count_ones() as usize) <= max_size {
            choices.push(f(UnorderedCards(sub)));
        }
        sub = (sub - 1) & mask.0;
    }
}

// ── Buyer affordability ──

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
                perform_mix(&mut wheel2, a, b);
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
                Some(PendingChoice::ChooseCardsToDestroy { count }) => {
                    if player.workshop_cards.is_empty() {
                        choices.push(ColoriChoice::DestroyDrawnCards {
                            card_instance_ids: UnorderedCards::new(),
                        });
                    } else {
                        enumerate_subsets_into(
                            player.workshop_cards,
                            *count as usize,
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
        _ => {}
    }
}

pub fn enumerate_choices(state: &GameState) -> Vec<ColoriChoice> {
    let mut choices = Vec::new();
    enumerate_choices_into(state, &mut choices);
    choices
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
                    Some(PendingChoice::ChooseCardsToDestroy { .. }) => {
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
    rng: &mut R,
) {
    det.clone_from(source);

    // Initialize cached scores for ISMCTS usage
    for p in &mut det.players {
        p.cached_score = calculate_score(p);
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

// ── Fused rollout step ──

pub fn apply_rollout_step<R: Rng>(state: &mut GameState, rng: &mut R) {
    enum Op {
        DraftPick(u32),
        DestroyDrafted(u32),
        DestroyAndMix { card_id: u32, mixes: [(Color, Color); 2], mix_count: usize },
        DestroyAndSell { card_id: u32, buyer_id: u32 },
        EndTurn,
        SkipWorkshop,
        Workshop(UnorderedCards),
        DestroyDrawn(UnorderedCards),
        MixAll { mixes: [(Color, Color); 2], mix_count: usize },
        SelectBuyer(u32),
        GainSecondary(Color),
        GainPrimary(Color),
        SwapTertiary(Color, Color),
    }

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
            perform_mix(&mut sim_wheel, a, b);
        }
        (mixes, count)
    }

    let op = match &state.phase {
        GamePhase::Draft { draft_state } => {
            let hand = draft_state.hands[draft_state.current_player_index];
            let id = hand.pick_random(rng).unwrap();
            Op::DraftPick(id as u32)
        }
        GamePhase::Action { action_state } => {
            let player = &state.players[action_state.current_player_index];
            match &action_state.pending_choice {
                None => {
                    if !player.drafted_cards.is_empty() && rng.random::<f64>() < 0.8 {
                        let card_id = player.drafted_cards.pick_random(rng).unwrap();
                        let card = state.card_lookup[card_id as usize];
                        match card.ability() {
                            Ability::MixColors { count } => {
                                let (mixes, mix_count) =
                                    random_mix_seq_inline(&player.color_wheel, count, rng);
                                Op::DestroyAndMix { card_id: card_id as u32, mixes, mix_count }
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
                                    Op::DestroyAndSell { card_id: card_id as u32, buyer_id: affordable[buyer_idx] }
                                } else {
                                    Op::DestroyDrafted(card_id as u32)
                                }
                            }
                            _ => Op::DestroyDrafted(card_id as u32),
                        }
                    } else {
                        Op::EndTurn
                    }
                }
                Some(PendingChoice::ChooseCardsForWorkshop { count }) => {
                    let mut copy = player.workshop_cards;
                    let selected = copy.draw_up_to(*count as u8, rng);
                    if selected.is_empty() {
                        Op::SkipWorkshop
                    } else {
                        Op::Workshop(selected)
                    }
                }
                Some(PendingChoice::ChooseCardsToDestroy { count }) => {
                    let mut copy = player.workshop_cards;
                    let selected = copy.draw_up_to(*count as u8, rng);
                    Op::DestroyDrawn(selected)
                }
                Some(PendingChoice::ChooseMix { remaining }) => {
                    let (mixes, mix_count) =
                        random_mix_seq_inline(&player.color_wheel, *remaining, rng);
                    Op::MixAll { mixes, mix_count }
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
                    Op::SelectBuyer(affordable[idx])
                }
                Some(PendingChoice::ChooseSecondaryColor) => {
                    let idx = rng.random_range(0..SECONDARIES.len());
                    Op::GainSecondary(SECONDARIES[idx])
                }
                Some(PendingChoice::ChoosePrimaryColor) => {
                    let idx = rng.random_range(0..PRIMARIES.len());
                    Op::GainPrimary(PRIMARIES[idx])
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
                    let lose_idx = rng.random_range(0..own_count);
                    let lose = owned[lose_idx];
                    let mut options = [Color::Red; 6];
                    let mut opt_count = 0usize;
                    for &c in &TERTIARIES {
                        if c != lose {
                            options[opt_count] = c;
                            opt_count += 1;
                        }
                    }
                    let gain_idx = rng.random_range(0..opt_count);
                    Op::SwapTertiary(lose, options[gain_idx])
                }
            }
        }
        _ => panic!("Cannot apply rollout step for current state"),
    };

    match op {
        Op::DraftPick(id) => {
            player_pick(state, id);
            if let GamePhase::Draft { ref draft_state } = state.phase {
                if draft_state.waiting_for_pass {
                    confirm_pass(state);
                }
            }
        }
        Op::DestroyDrafted(id) => destroy_drafted_card(state, id, rng),
        Op::DestroyAndMix { card_id, mixes, mix_count } => {
            destroy_drafted_card(state, card_id, rng);
            for i in 0..mix_count {
                let (a, b) = mixes[i];
                resolve_mix_colors(state, a, b, rng);
            }
            if let GamePhase::Action { ref action_state } = state.phase {
                if matches!(action_state.pending_choice, Some(PendingChoice::ChooseMix { .. })) {
                    skip_mix(state, rng);
                }
            }
        }
        Op::DestroyAndSell { card_id, buyer_id } => {
            destroy_drafted_card(state, card_id, rng);
            resolve_select_buyer(state, buyer_id, rng);
        }
        Op::EndTurn => {
            end_player_turn(state, rng);
            if matches!(state.phase, GamePhase::Draw) {
                execute_draw_phase(state, rng);
            }
        }
        Op::SkipWorkshop => skip_workshop(state, rng),
        Op::Workshop(selected) => resolve_workshop_choice(state, selected, rng),
        Op::DestroyDrawn(selected) => resolve_destroy_cards(state, selected, rng),
        Op::MixAll { mixes, mix_count } => {
            for i in 0..mix_count {
                let (a, b) = mixes[i];
                resolve_mix_colors(state, a, b, rng);
            }
            if let GamePhase::Action { ref action_state } = state.phase {
                if matches!(action_state.pending_choice, Some(PendingChoice::ChooseMix { .. })) {
                    skip_mix(state, rng);
                }
            }
        }
        Op::SelectBuyer(id) => resolve_select_buyer(state, id, rng),
        Op::GainSecondary(c) => resolve_gain_secondary(state, c, rng),
        Op::GainPrimary(c) => resolve_gain_primary(state, c, rng),
        Op::SwapTertiary(lose, gain) => {
            resolve_choose_tertiary_to_lose(state, lose);
            resolve_choose_tertiary_to_gain(state, gain, rng);
        }
    }
}
