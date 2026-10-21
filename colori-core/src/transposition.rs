//! Which search states are the same position, for `mcts::GraphSearcher`.
//!
//! A turn is a run of trashes, workshops, mixes and sales whose order mostly
//! does not matter, so a tree holds each position of a turn once per order it
//! was reached in. The graph search merges states with equal keys into one
//! node, which is only sound where a node's key is the same on every iteration
//! that reaches it.
//!
//! In an action phase the searching player's tree holds only their own moves.
//! `advance` has already played every opponent's turn, differently on every
//! iteration (`crate::opponent_turns`), and none of it depends on the route the
//! searcher takes. No turn touches another player's state. The one shared pile
//! a turn adds to, the trashed cards, gains exactly what left the player's
//! own cards. So the key covers the searcher's own side of the phase and
//! nothing else, and two routes to the same own state face the same opponents.
//!
//! That needs the searcher's turn to start the same way on every iteration,
//! which holds only when the search began in this action phase. A search rooted
//! earlier reaches the action phase through hidden draft picks, and the next
//! round starts with a random draw, so those states are never merged.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use mcts::Transposition;

use crate::mcts_impl::SearchContext;
use crate::types::{Card, GamePhase, GameState, SellCard, NUM_CARDS, NUM_SELL_CARDS};
use crate::unordered_cards::UnorderedCards;

impl Transposition for GameState {
    type Key = u128;

    fn transposition_key(&self, ctx: &SearchContext) -> Option<u128> {
        if !matches!(self.phase, GamePhase::Action { .. }) || ctx.merge_round != Some(self.round) {
            return None;
        }
        Some(self.own_key(ctx.perspective))
    }
}

impl GameState {
    /// `player`'s own side of the game, from which two states play the same for
    /// them. Two independent 64-bit hashes, so that a collision merging two
    /// different positions is negligible.
    pub(crate) fn own_key(&self, player: usize) -> u128 {
        let low = self.own_hash(player, 0x243F_6A88_85A3_08D3);
        let high = self.own_hash(player, 0x1319_8A2E_0370_7344);
        (high as u128) << 64 | low as u128
    }

    /// Everything of `player`'s that decides how the rest of their turn and the
    /// game can go, from the abilities waiting to resolve on. Cards are counted
    /// by type, since two copies of a card are interchangeable, and so are the
    /// buyers, whose slot order plays no part.
    fn own_hash(&self, player: usize, seed: u64) -> u64 {
        let mut h = DefaultHasher::new();
        seed.hash(&mut h);
        self.round.hash(&mut h);
        if let GamePhase::Action { action_state } = &self.phase {
            action_state.turns[player]
                .ability_stack
                .as_slice()
                .hash(&mut h);
        }
        let own = &self.players[player];
        own.deck.segments().len().hash(&mut h);
        for segment in own.deck.segments() {
            self.card_types(*segment).hash(&mut h);
        }
        self.card_types(own.workshop_cards).hash(&mut h);
        self.card_types(own.workshopped_cards).hash(&mut h);
        self.card_types(own.drafted_cards).hash(&mut h);
        own.color_wheel.counts.hash(&mut h);
        own.materials.counts.hash(&mut h);
        sell_type_counts(own.buyers.iter().map(|b| b.sell_card)).hash(&mut h);
        sell_type_counts(own.completed_sell_cards.iter().map(|c| c.sell_card)).hash(&mut h);
        own.ducats.hash(&mut h);
        own.wild_primaries.hash(&mut h);
        own.boats.hash(&mut h);
        h.finish()
    }

    fn card_types(&self, cards: UnorderedCards) -> [u8; NUM_CARDS] {
        let mut counts = [0u8; NUM_CARDS];
        for id in cards.iter() {
            let card: Card = self.card_lookup[id as usize];
            counts[card as usize] += 1;
        }
        counts
    }
}

fn sell_type_counts(cards: impl Iterator<Item = SellCard>) -> [u8; NUM_SELL_CARDS] {
    let mut counts = [0u8; NUM_SELL_CARDS];
    for card in cards {
        counts[card as usize] += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use mcts::Transposition;
    use rand::{RngExt, SeedableRng};
    use wyrand::WyRand;

    use crate::choices::enumerate_choices;
    use crate::colori_game::{apply_choice_to_state, get_game_status, GameStatus};
    use crate::draw_phase::execute_draw_phase;
    use crate::mcts_impl::SearchContext;
    use crate::scoring::HeuristicParams;
    use crate::setup::create_initial_game_state;
    use crate::types::{Ability, Choice, GamePhase, GameState};

    fn params() -> HeuristicParams {
        const PARAMS_JSON: &str = include_str!("../../genetic-algorithm/batch-lki08w-gen-32.json");
        serde_json::from_str(PARAMS_JSON).expect("frozen heuristic params should parse")
    }

    /// The context a search for `perspective` rooted at `root` would run under.
    fn context(root: &GameState, perspective: usize) -> SearchContext {
        SearchContext::new(
            root,
            perspective,
            &params(),
            true,
            None,
            true,
            false,
            1000,
            false,
            0.75,
        )
    }

    fn taking_turn(state: &GameState, player: usize) -> bool {
        matches!(&state.phase, GamePhase::Action { action_state } if !action_state.turns[player].ended)
    }

    /// Random play from a fresh game to the start of `round`'s action phase.
    fn action_phase(players: usize, seed: u64, round: u32) -> GameState {
        let mut rng = WyRand::seed_from_u64(seed);
        let mut state = create_initial_game_state(players, &vec![true; players], &mut rng);
        execute_draw_phase(&mut state, &mut rng);
        while !(state.round == round && matches!(state.phase, GamePhase::Action { .. })) {
            if matches!(state.phase, GamePhase::Draw) {
                execute_draw_phase(&mut state, &mut rng);
                continue;
            }
            let GameStatus::AwaitingAction { player_index } = get_game_status(&state, None) else {
                panic!("seed {seed}: the game ended before round {round}");
            };
            let choices = enumerate_choices(&state, player_index);
            let choice = choices[rng.random_range(0..choices.len())].clone();
            apply_choice_to_state(&mut state, player_index, &choice, &mut rng);
        }
        state
    }

    /// Play every turn but `perspective`'s at random, as `advance` plays them.
    fn play_opponents(state: &mut GameState, perspective: usize, seed: u64) {
        let mut rng = WyRand::seed_from_u64(seed);
        for player in 0..state.players.len() {
            while player != perspective && taking_turn(state, player) {
                let choices = enumerate_choices(state, player);
                let choice = choices[rng.random_range(0..choices.len())].clone();
                apply_choice_to_state(state, player, &choice, &mut rng);
            }
        }
    }

    #[test]
    fn only_the_root_rounds_action_phase_is_keyed() {
        let action = action_phase(3, 1, 2);
        let ctx = context(&action, 1);
        assert!(action.transposition_key(&ctx).is_some());

        let mut rng = WyRand::seed_from_u64(2);
        let mut draft = create_initial_game_state(3, &[true; 3], &mut rng);
        execute_draw_phase(&mut draft, &mut rng);
        assert!(
            draft.transposition_key(&ctx).is_none(),
            "a state outside the action phase is never merged"
        );
        assert!(
            action.transposition_key(&context(&draft, 1)).is_none(),
            "a search rooted before the action phase reaches it through hidden picks"
        );

        let mut later = action.clone();
        later.round += 1;
        assert!(
            later.transposition_key(&ctx).is_none(),
            "a later round begins with a random draw"
        );
    }

    #[test]
    fn a_key_reads_the_searchers_own_side_alone() {
        let state = action_phase(4, 3, 3);
        let ctx = context(&state, 2);
        let key = state.transposition_key(&ctx);

        let mut pending = state.clone();
        if let GamePhase::Action { action_state } = &mut pending.phase {
            action_state.turns[2].ability_stack.push(Ability::Sell);
        }
        assert_ne!(key, pending.transposition_key(&ctx));

        let mut richer = state.clone();
        richer.players[2].ducats += 1;
        assert_ne!(key, richer.transposition_key(&ctx));

        let mut opponent = state.clone();
        opponent.players[0].ducats += 1;
        if let GamePhase::Action { action_state } = &mut opponent.phase {
            action_state.turns[1].ability_stack.push(Ability::Sell);
        }
        assert_eq!(key, opponent.transposition_key(&ctx));
    }

    /// What the searcher's tree sees: the same own moves after opponents' turns
    /// that went differently. A node's key has to come out the same on every
    /// iteration, so it must not see them.
    #[test]
    fn opponents_turns_do_not_move_the_key() {
        for (players, seed, round) in [(2usize, 5u64, 1u32), (3, 7, 2), (4, 11, 4)] {
            let root = action_phase(players, seed, round);
            let perspective = players - 1;
            let ctx = context(&root, perspective);
            let mut moves = Vec::new();
            let mut walk = root.clone();
            let mut rng = WyRand::seed_from_u64(seed);
            while moves.len() < 4 {
                let choices: Vec<Choice> = enumerate_choices(&walk, perspective)
                    .into_iter()
                    .filter(|choice| !matches!(choice, Choice::EndTurn))
                    .collect();
                if choices.is_empty() {
                    break;
                }
                let choice = choices[rng.random_range(0..choices.len())].clone();
                apply_choice_to_state(&mut walk, perspective, &choice, &mut rng);
                moves.push(choice);
            }

            let keys: Vec<_> = (0..8)
                .map(|opponents| {
                    let mut state = root.clone();
                    play_opponents(&mut state, perspective, opponents);
                    let mut rng = WyRand::seed_from_u64(0);
                    for choice in &moves {
                        apply_choice_to_state(&mut state, perspective, choice, &mut rng);
                    }
                    state
                        .transposition_key(&ctx)
                        .expect("still the root round's action phase")
                })
                .collect();
            assert!(
                keys.windows(2).all(|pair| pair[0] == pair[1]),
                "players={players}"
            );
        }
    }

    /// Walk every line of the searcher's turn a few choices deep. Any two states
    /// that share a key must play the same: the same choices, each leading to
    /// states that share a key again. That holds a key to everything that
    /// decides what happens next, whether or not the key's own field list
    /// thought to include it, and it requires that turns really do reach one
    /// position by more than one route, or there is nothing to merge.
    #[test]
    fn states_that_share_a_key_play_the_same() {
        let mut merged = 0;
        for (players, seed, round) in [(2usize, 5u64, 1u32), (3, 7, 3), (4, 11, 5)] {
            let mut root = action_phase(players, seed, round);
            let perspective = 0;
            play_opponents(&mut root, perspective, seed);
            let ctx = context(&root, perspective);
            let mut by_key: HashMap<u128, (GameState, Vec<Choice>)> = HashMap::new();
            let mut frontier = vec![(root.clone(), 0usize)];
            let mut rng = WyRand::seed_from_u64(0);
            while let Some((state, depth)) = frontier.pop() {
                let Some(key) = state.transposition_key(&ctx) else {
                    continue;
                };
                let mut choices = enumerate_choices(&state, perspective);
                choices.sort_by_key(|c| format!("{c:?}"));
                match by_key.get(&key) {
                    Some((other, other_choices)) => {
                        merged += 1;
                        assert_eq!(
                            &choices, other_choices,
                            "a shared key offered different choices"
                        );
                        for choice in &choices {
                            let (mut a, mut b) = (state.clone(), other.clone());
                            apply_choice_to_state(&mut a, perspective, choice, &mut rng);
                            apply_choice_to_state(&mut b, perspective, choice, &mut rng);
                            assert_eq!(
                                a.transposition_key(&ctx),
                                b.transposition_key(&ctx),
                                "{choice:?} took two states with one key to different positions"
                            );
                        }
                        continue;
                    }
                    None => {
                        by_key.insert(key, (state.clone(), choices.clone()));
                    }
                }
                if depth == 3 {
                    continue;
                }
                for choice in &choices {
                    let mut next = state.clone();
                    apply_choice_to_state(&mut next, perspective, choice, &mut rng);
                    frontier.push((next, depth + 1));
                }
            }
        }
        assert!(merged > 0, "no turn reached a position by two routes");
    }
}
