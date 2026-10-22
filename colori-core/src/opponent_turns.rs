//! How the search plays opponents' action turns.
//!
//! Everyone takes their action turn at once, unseen by the others until the
//! phase is over. A tree node is conditioned on every move above it, so an
//! opponent's turn in the tree would either answer the searching player's
//! moves or be answered by them. Opponents' turns are therefore played outside
//! the tree, through `Game::advance`, before the searching player's own.
//!
//! Each opponent gets a tree of their own per round, keyed by nothing but
//! their own moves and searched for their own reward: decoupled UCT, as the
//! draft does for single picks, over the run of decisions a turn takes. Past
//! the edge of that tree, the rest of the turn is played by the rollout
//! policy. So opponents improve as the search does, and none of it depends on
//! what the searching player chose.

use crate::colori_game::{apply_choice_to_state, enumerate_choices_into, is_taking_turn};
use crate::rollout::play_out_action_turn;
use crate::scoring::HeuristicParams;
use crate::types::{Choice, GamePhase, GameState, MAX_PLAYERS};
use rand::{Rng, RngExt};
use rustc_hash::FxHashMap;

const ROOT: u32 = 0;
const UNTRIED: u32 = u32::MAX;

#[derive(Default)]
struct TurnNode {
    /// In a tree whose turn starts from the same state in every iteration,
    /// a node offers the same moves on every visit, so they are enumerated
    /// once and kept here, each with the child it leads to or `UNTRIED`.
    moves: Vec<(Choice, u32)>,
    /// Otherwise the moves vary with the determinization, and children are
    /// looked up by move.
    children: FxHashMap<Choice, u32>,
    visits: u32,
    /// Visits to the parent in which this move was legal, which stands in
    /// for the parent's visit count where what is legal varies.
    availability: u32,
    cumulative_reward: f64,
}

/// One opponent's turn in one round.
struct TurnTree {
    round: u32,
    player: usize,
    /// Whether the turn starts from the same state in every iteration.
    fixed: bool,
    nodes: Vec<TurnNode>,
}

/// How to finish a turn past the edge of an opponent's tree, matching the
/// search's rollouts.
pub(crate) struct TurnPolicy<'a> {
    pub heuristic: bool,
    pub params: &'a HeuristicParams,
}

#[derive(Default)]
pub(crate) struct OpponentTurns {
    /// The round whose turns every iteration starts identically: the root's,
    /// when the search starts in the action phase. All of an opponent's turn
    /// is public then, and nothing in it is random.
    fixed_round: Option<u32>,
    trees: Vec<TurnTree>,
    /// Each node this iteration passed through, by tree, to be credited with
    /// that tree's player's reward once the iteration's payoffs are known.
    visited: Vec<(u32, u32)>,
    choices: Vec<Choice>,
    moves: Vec<(Choice, u32)>,
}

impl OpponentTurns {
    pub(crate) fn for_root(root: &GameState) -> Self {
        Self {
            fixed_round: matches!(root.phase, GamePhase::Action { .. }).then_some(root.round),
            ..Self::default()
        }
    }

    /// Play out the turn of every opponent of `perspective` still taking one.
    pub(crate) fn play<R: Rng>(
        &mut self,
        state: &mut GameState,
        perspective: usize,
        exploration: f64,
        policy: &TurnPolicy,
        rng: &mut R,
    ) {
        for player in 0..state.players.len() {
            if player != perspective && is_taking_turn(state, player) {
                self.play_turn(state, player, exploration, policy, rng);
            }
        }
    }

    fn play_turn<R: Rng>(
        &mut self,
        state: &mut GameState,
        player: usize,
        exploration: f64,
        policy: &TurnPolicy,
        rng: &mut R,
    ) {
        let tree_index = self.tree_index(state.round, player);
        let tree = &mut self.trees[tree_index as usize];
        let mut node = ROOT;
        self.visited.push((tree_index, ROOT));

        while is_taking_turn(state, player) {
            // Taken out of their homes for the visit, and put back after.
            let mut moves = if tree.fixed {
                std::mem::take(&mut tree.nodes[node as usize].moves)
            } else {
                std::mem::take(&mut self.moves)
            };
            if !tree.fixed || moves.is_empty() {
                moves.clear();
                enumerate_choices_into(state, player, &mut self.choices);
                let children = &tree.nodes[node as usize].children;
                moves.extend(self.choices.drain(..).map(|choice| {
                    let child = children.get(&choice).copied().unwrap_or(UNTRIED);
                    (choice, child)
                }));
            }

            let mut untried: Option<usize> = None;
            let mut untried_seen = 0u32;
            let mut best: Option<usize> = None;
            let mut best_value = f64::NEG_INFINITY;
            for (i, &(_, child)) in moves.iter().enumerate() {
                if child == UNTRIED {
                    // Uniform over the untried moves, in one pass.
                    untried_seen += 1;
                    if rng.random_range(0..untried_seen) == 0 {
                        untried = Some(i);
                    }
                    continue;
                }
                let child_node = &mut tree.nodes[child as usize];
                child_node.availability += 1;
                let value = if child_node.visits == 0 {
                    f64::INFINITY
                } else {
                    let visits = child_node.visits as f64;
                    child_node.cumulative_reward / visits
                        + exploration * ((child_node.availability as f64).ln() / visits).sqrt()
                };
                if value > best_value {
                    best_value = value;
                    best = Some(i);
                }
            }

            let expanding = untried.is_some();
            let i = untried
                .or(best)
                .expect("A player taking their turn can always end it");
            if expanding {
                let child = tree.nodes.len() as u32;
                tree.nodes.push(TurnNode { availability: 1, ..TurnNode::default() });
                moves[i].1 = child;
                if !tree.fixed {
                    tree.nodes[node as usize].children.insert(moves[i].0.clone(), child);
                }
            }
            let child = moves[i].1;
            self.visited.push((tree_index, child));
            apply_choice_to_state(state, player, &moves[i].0, rng);

            if tree.fixed {
                tree.nodes[node as usize].moves = moves;
            } else {
                self.moves = moves;
            }

            if expanding {
                play_out_action_turn(state, player, policy.heuristic, policy.params, rng);
                return;
            }
            node = child;
        }
    }

    fn tree_index(&mut self, round: u32, player: usize) -> u32 {
        if let Some(i) = self
            .trees
            .iter()
            .position(|tree| tree.round == round && tree.player == player)
        {
            return i as u32;
        }
        self.trees.push(TurnTree {
            round,
            player,
            fixed: self.fixed_round == Some(round),
            nodes: vec![TurnNode::default()],
        });
        (self.trees.len() - 1) as u32
    }

    pub(crate) fn begin_iteration(&mut self) {
        self.visited.clear();
    }

    pub(crate) fn credit(&mut self, rewards: &[f64; MAX_PLAYERS]) {
        for &(tree_index, node) in &self.visited {
            let tree = &mut self.trees[tree_index as usize];
            let reward = rewards[tree.player];
            let node = &mut tree.nodes[node as usize];
            node.visits += 1;
            node.cumulative_reward += reward;
        }
        self.visited.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw_phase::execute_draw_phase;
    use crate::setup::create_initial_game_state;
    use crate::types::ActionState;
    use rand::SeedableRng;
    use wyrand::WyRand;

    fn params() -> HeuristicParams {
        const PARAMS_JSON: &str = include_str!("../../genetic-algorithm/batch-lki08w-gen-32.json");
        serde_json::from_str(PARAMS_JSON).expect("frozen heuristic params should parse")
    }

    /// The first action phase of a three-player game, reached by random play.
    fn action_phase(seed: u64) -> GameState {
        let mut rng = WyRand::seed_from_u64(seed);
        let mut state = create_initial_game_state(3, &[true, true, true], &mut rng);
        execute_draw_phase(&mut state, &mut rng);
        while !matches!(state.phase, GamePhase::Action { .. }) {
            let player = match &state.phase {
                GamePhase::Buyers { buyers_state } => buyers_state.current_player_index,
                GamePhase::Draft { draft_state } => draft_state.current_player_index,
                _ => unreachable!("round 1 reaches its action phase"),
            };
            let choices = crate::choices::enumerate_choices(&state, player);
            let choice = choices[rng.random_range(0..choices.len())].clone();
            apply_choice_to_state(&mut state, player, &choice, &mut rng);
        }
        state
    }

    fn turns(state: &GameState) -> &ActionState {
        match &state.phase {
            GamePhase::Action { action_state } => action_state,
            _ => panic!("Expected action phase"),
        }
    }

    #[test]
    fn plays_every_opponent_turn_and_leaves_the_searcher_to_act() {
        let params = params();
        let policy = TurnPolicy { heuristic: true, params: &params };
        let mut model = OpponentTurns::for_root(&action_phase(17));
        let mut rng = WyRand::seed_from_u64(5);

        for _ in 0..50 {
            let mut state = action_phase(17);
            model.begin_iteration();
            model.play(&mut state, 1, 0.75, &policy, &mut rng);

            let turns = turns(&state);
            assert!(turns.turns[0].ended && turns.turns[2].ended);
            assert!(!turns.turns[1].ended, "the searcher's turn is theirs to play");
            model.credit(&[1.0, 0.0, 0.0, 0.0]);
        }
    }

    /// The point of the model: what an opponent does cannot depend on the
    /// searching player's moves, because those are not made yet when the
    /// opponents play, and the opponent's tree holds no trace of them.
    #[test]
    fn opponent_turns_do_not_see_the_searchers_moves() {
        let params = params();
        let policy = TurnPolicy { heuristic: true, params: &params };
        let base = action_phase(29);

        // The searcher has already made a move in one of the two states.
        let mut moved = base.clone();
        let mut rng = WyRand::seed_from_u64(1);
        let first = crate::choices::enumerate_choices(&moved, 1)
            .into_iter()
            .find(|choice| !matches!(choice, Choice::EndTurn { .. }))
            .expect("a first-round turn has something to do");
        apply_choice_to_state(&mut moved, 1, &first, &mut rng);

        let play = |mut state: GameState, fixed: bool| {
            let mut model = if fixed {
                OpponentTurns::for_root(&state)
            } else {
                OpponentTurns::default()
            };
            let mut rng = WyRand::seed_from_u64(99);
            for _ in 0..30 {
                let mut copy = state.clone();
                model.begin_iteration();
                model.play(&mut copy, 1, 0.75, &policy, &mut rng);
                model.credit(&[0.5, 0.0, 0.5, 0.0]);
            }
            model.begin_iteration();
            model.play(&mut state, 1, 0.75, &policy, &mut rng);
            [state.players[0].clone(), state.players[2].clone()]
        };

        for fixed in [false, true] {
            let [a0, a2] = play(base.clone(), fixed);
            let [b0, b2] = play(moved.clone(), fixed);
            assert_eq!(format!("{a0:?}"), format!("{b0:?}"));
            assert_eq!(format!("{a2:?}"), format!("{b2:?}"));
        }
    }
}
