//! The round boundary: the workshop top-up draw, round income, and the move
//! from one round into the next.
//!
//! There is no draw phase. A player draws their workshop back up as the last
//! thing their own turn does, so a card shuffled to the bottom of the deck at
//! the end of a turn is drawn again by that same player at the end of their
//! next one, without anyone waiting on it. Setup deals the opening workshops
//! the same way, and the income a round brings is collected as the round opens.

use crate::buyers_phase::initialize_buyers_phase;
use crate::draw_log_helpers::is_replaying;
use crate::game_log::{DrawEvent, DrawLog};
use crate::types::{CardInstance, GamePhase, GameState, PlayerState};
use crate::unordered_cards::UnorderedCards;
use rand::Rng;

/// How many cards a player holds in the workshop during the action phase.
pub const WORKSHOP_SIZE: u32 = 5;

/// What every player collects at the start of a round.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RoundIncome {
    pub gold_coins: u32,
    pub boats: u32,
    pub bowls: u32,
}

/// Income for `round`: a gold coin in rounds 4 and 6, a boat in round 5, and a
/// gold coin, a boat and a bowl in round 7. A gold coin is a point on its own,
/// but its real use is that it buys a workshop pick, a mix or a sale — see
/// `Choice::SpendGoldCoin`.
pub fn round_income(round: u32) -> RoundIncome {
    match round {
        4 | 6 => RoundIncome { gold_coins: 1, ..RoundIncome::default() },
        5 => RoundIncome { boats: 1, ..RoundIncome::default() },
        7 => RoundIncome { gold_coins: 1, boats: 1, bowls: 1 },
        _ => RoundIncome::default(),
    }
}

/// Hand out the round's income. Deterministic, so it needs no draw-log event
/// to replay.
///
/// The rollout's round shortcut stands in for this whole function and calls
/// straight into here, so the rule lives in one place.
pub fn collect_round_income(state: &mut GameState) {
    let income = round_income(state.round);
    for player in state.players.iter_mut() {
        player.gold_coins += income.gold_coins;
        player.cached_whole_points += income.gold_coins;
        player.boats += income.boats;
        player.bowls += income.bowls;
    }
}

/// How many cards a player's workshop is short of [`WORKSHOP_SIZE`].
///
/// The workshop is topped up rather than dealt a fixed five, because a card
/// kept at the end of a turn went onto the top of the deck and comes back out
/// of it here. Keeping one therefore trades against a fresh draw instead of
/// adding to it.
fn top_up_count(player: &PlayerState) -> u32 {
    WORKSHOP_SIZE.saturating_sub(player.workshop_cards.len())
}

/// Draw `player_index`'s workshop back up to [`WORKSHOP_SIZE`].
pub fn draw_into_workshop<R: Rng>(state: &mut GameState, player_index: usize, rng: &mut R) {
    if is_replaying(state) {
        replay_player_deck_draw(state, player_index);
        return;
    }
    let before = state.players[player_index].workshop_cards;
    let player = &mut state.players[player_index];
    let count = top_up_count(player);
    player.deck.draw_into(&mut player.workshop_cards, count, rng);
    record_player_deck_draw(state, player_index, before);
}

/// Deal every player's opening workshop: the same top-up a turn's cleanup
/// draws, before the game's first buyers phase.
pub fn deal_opening_workshops<R: Rng>(state: &mut GameState, rng: &mut R) {
    for player_index in 0..state.players.len() {
        draw_into_workshop(state, player_index, rng);
    }
}

/// Open `round`: everyone collects its income and the buyers phase begins.
pub fn start_round<R: Rng>(state: &mut GameState, rng: &mut R) {
    collect_round_income(state);
    initialize_buyers_phase(state, rng);
}

/// Close the round: the next one opens, or the game ends.
pub fn end_round<R: Rng>(state: &mut GameState, rng: &mut R) {
    state.round += 1;
    if state.round > state.max_rounds {
        state.phase = GamePhase::GameOver;
    } else {
        start_round(state, rng);
    }
}

fn record_player_deck_draw(state: &mut GameState, player_index: usize, before: UnorderedCards) {
    if let Some(DrawLog::Recording(log)) = &mut state.draw_log {
        let drawn = state.players[player_index].workshop_cards.difference(before);
        let cards: Vec<CardInstance> = drawn
            .iter()
            .map(|id| CardInstance {
                instance_id: id as u32,
                card: state.card_lookup[id as usize],
            })
            .collect();
        if !cards.is_empty() {
            log.push(DrawEvent::PlayerDeckDraw {
                player_index,
                cards,
            });
        }
    }
}

/// In replay mode, pop the next PlayerDeckDraw event for this player and apply
/// it, moving the recorded cards from the deck to the workshop.
///
/// A draw that yielded nothing records no event — the deck can run dry, and
/// there is no discard pile to reshuffle — so the queue is matched against
/// before it is consumed. Popping blindly would hand this player the next
/// player's draw.
fn replay_player_deck_draw(state: &mut GameState, player_index: usize) {
    let matches_player = matches!(
        &state.draw_log,
        Some(DrawLog::Replaying(queue))
            if matches!(queue.front(), Some(DrawEvent::PlayerDeckDraw { player_index: pi, .. }) if *pi == player_index)
    );
    if !matches_player {
        return;
    }
    let event = match &mut state.draw_log {
        Some(DrawLog::Replaying(queue)) => queue.pop_front(),
        _ => return,
    };
    if let Some(DrawEvent::PlayerDeckDraw { cards, .. }) = event {
        let player = &mut state.players[player_index];
        for card in &cards {
            let id = card.instance_id as u8;
            player.deck.remove(id);
            player.workshop_cards.insert(id);
        }
    }
}
