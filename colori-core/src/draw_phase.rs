use crate::buyers_phase::initialize_buyers_phase;
use crate::draw_log_helpers::is_replaying;
use crate::game_log::{DrawEvent, DrawLog};
use crate::types::{CardInstance, GameState, PlayerState};
use crate::unordered_cards::UnorderedCards;
use rand::Rng;

/// How many cards a player starts the action phase holding in the workshop.
pub const WORKSHOP_SIZE: u32 = 5;

/// How many cards the draw phase adds.
///
/// The workshop is topped up to [`WORKSHOP_SIZE`] rather than dealt a fixed
/// five, because cards kept in the draft pool now arrive in the workshop at
/// the end of the previous round. Keeping one therefore trades against a fresh
/// draw instead of adding to the hand.
fn top_up_count(player: &PlayerState) -> u32 {
    WORKSHOP_SIZE.saturating_sub(player.workshop_cards.len())
}

/// What every player collects at the start of a round.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RoundIncome {
    pub ducats: u32,
    pub boats: u32,
    pub wild_primaries: u32,
}

/// Income for `round`: a ducat in rounds 4 and 6, a boat in round 5, and a
/// ducat, a boat and a wild primary in round 7. A ducat is a point on its own,
/// but its real use is that it buys a workshop pick, a mix or a sale — see
/// `Choice::SpendDucat`.
pub fn round_income(round: u32) -> RoundIncome {
    match round {
        4 | 6 => RoundIncome { ducats: 1, ..RoundIncome::default() },
        5 => RoundIncome { boats: 1, ..RoundIncome::default() },
        7 => RoundIncome { ducats: 1, boats: 1, wild_primaries: 1 },
        _ => RoundIncome::default(),
    }
}

/// Hand out the round's income. Deterministic, so it needs no draw-log event
/// to replay.
///
/// The rollout's draw+draft shortcut stands in for this whole function and
/// calls straight into here, so the rule lives in one place.
pub fn collect_round_income(state: &mut GameState) {
    let income = round_income(state.round);
    for player in state.players.iter_mut() {
        player.ducats += income.ducats;
        player.cached_whole_points += income.ducats;
        player.boats += income.boats;
        player.wild_primaries += income.wild_primaries;
    }
}

pub fn execute_draw_phase<R: Rng>(state: &mut GameState, rng: &mut R) {
    let num_players = state.players.len();

    collect_round_income(state);

    if is_replaying(state) {
        for i in 0..num_players {
            replay_player_deck_draw(state, i);
        }
    } else {
        for i in 0..num_players {
            let before = state.players[i].workshop_cards;
            let player = &mut state.players[i];
            let count = top_up_count(player);
            player.deck.draw_into(&mut player.workshop_cards, count, rng);
            record_player_deck_draw(state, i, before);
        }
    }

    initialize_buyers_phase(state, rng);
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

