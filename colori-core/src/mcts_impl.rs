//! Colori on the shared `mcts` crate.
//!
//! The search itself lives in `mcts`; everything here is the game's side of the
//! contract. Three pieces of it are worth knowing about:
//!
//! * [`SearchContext`] holds what is constant for a whole search — tuned
//!   parameters, the precomputed card table, per-player scores at the root, the
//!   rollout horizon. Keeping it out of `GameState` means it is not copied on
//!   every determinization.
//! * [`OpponentModel`] plays the opponents' simultaneous moves: the DUCT
//!   bandit over their draft picks, and a tree per opponent over their own
//!   action turn (`crate::opponent_turns`). Neither is part of the searching
//!   player's tree, where every move is conditioned on the moves above it, so
//!   they live in the search's side state and are advanced through the
//!   `advance` hook.
//! * The rollout is handed over whole, so the expert policy can keep resolving
//!   an entire draft phase in one step.

use mcts::rand_core::Rng;
use mcts::{ChoicesOnRevisit, Game, Status};

use crate::choices::{enumerate_choices, enumerate_choices_into};
use crate::colori_game::{apply_choice_to_state, determinize_in_place};
use crate::ismcts::MctsConfig;
use crate::ismcts::{advance_past_opponent_draft_picks, OpponentDraftStats};
use crate::opponent_turns::{OpponentTurns, TurnPolicy};
use crate::rollout::{apply_heuristic_rollout_step, apply_rollout_step};
use crate::scoring::{
    calculate_whole_points, compute_heuristic_rewards, compute_terminal_rewards, heuristic_score,
    CardHeuristicTable, HeuristicParams,
};
use crate::types::{Card, Choice, GamePhase, GameState, MAX_PLAYERS};

pub type Rewards = [f64; MAX_PLAYERS];

/// A node of the search tree, re-exported so consumers can inspect a tree
/// without depending on `mcts` directly.
pub type SearchNode = mcts::Node<Choice>;

/// Why a search stopped, re-exported so consumers can name the outcome without
/// depending on `mcts` directly.
pub use mcts::StopReason;

/// The graph a transposition search leaves behind, keyed as
/// `crate::transposition` keys positions.
pub type SearchGraph = mcts::Graph<Choice, u128>;

/// Constant for one search. Never cloned per iteration.
pub struct SearchContext {
    /// The player searching. In the action phase everyone acts at once, and
    /// this is who the tree is for.
    pub perspective: usize,
    pub heuristic_params: HeuristicParams,
    pub card_table: CardHeuristicTable,
    /// Scores at the root, restored after every determinization because
    /// `cached_whole_points` is a denormalised field the search must not let drift.
    pub cached_whole_points: [u32; MAX_PLAYERS],
    /// Rollouts past this round stop and are evaluated instead of played out.
    pub max_rollout_round: Option<u32>,
    /// Whether that evaluation is heuristic or the real terminal scoring.
    pub use_heuristic: bool,
    pub heuristic_rollout: bool,
    pub heuristic_draft: bool,
    pub max_rollout_steps: u32,
    /// Skip the rollout and evaluate the leaf directly.
    pub no_rollout: bool,
    pub force_max_workshop: bool,
    /// Used by the DUCT bandit, which runs its own UCB outside the tree.
    pub exploration_constant: f64,
    /// The round whose action phase the search began in, if it began in one:
    /// the only stretch of play in which the searcher's turn starts the same
    /// way on every iteration, and so the only states a transposition search
    /// may merge.
    pub merge_round: Option<u32>,
}

impl SearchContext {
    /// Resolves the same horizon and evaluation choice that `ismcts` derives
    /// from its config, so both implementations search to the same depth.
    pub fn new(
        state: &GameState,
        perspective: usize,
        params: &HeuristicParams,
        use_heuristic_eval: bool,
        max_rollout_round: Option<u32>,
        heuristic_rollout: bool,
        heuristic_draft: bool,
        max_rollout_steps: u32,
        no_rollout: bool,
        force_max_workshop: bool,
        exploration_constant: f64,
    ) -> Self {
        let (effective_max_rollout_round, use_heuristic) = if use_heuristic_eval {
            if state.round <= params.heuristic_round_threshold {
                let heuristic_round = state.round + params.heuristic_lookahead;
                let effective =
                    max_rollout_round.map_or(heuristic_round, |mr| mr.min(heuristic_round));
                (Some(effective), true)
            } else {
                (max_rollout_round, false)
            }
        } else {
            (max_rollout_round, false)
        };

        let mut cached_whole_points = [0u32; MAX_PLAYERS];
        for (i, player) in state.players.iter().enumerate() {
            cached_whole_points[i] = calculate_whole_points(player);
        }

        Self {
            perspective,
            heuristic_params: params.clone(),
            card_table: CardHeuristicTable::new(params),
            cached_whole_points,
            max_rollout_round: effective_max_rollout_round,
            use_heuristic,
            heuristic_rollout,
            heuristic_draft,
            max_rollout_steps,
            no_rollout,
            force_max_workshop,
            exploration_constant,
            merge_round: matches!(state.phase, GamePhase::Action { .. }).then_some(state.round),
        }
    }

    fn evaluate(&self, state: &GameState) -> Rewards {
        if self.use_heuristic {
            compute_heuristic_rewards(
                &state.players,
                &state.card_lookup,
                &self.heuristic_params,
                &self.card_table,
            )
        } else {
            compute_terminal_rewards(&state.players)
        }
    }
}

/// The opponents' simultaneous moves, played outside the tree and rewarded
/// separately once each iteration's payoffs are known.
///
/// Draft picks get a flat bandit, plus this iteration's picks so they can be
/// credited; action turns get a tree per opponent.
#[derive(Default)]
pub struct OpponentModel {
    draft_stats: OpponentDraftStats,
    pick_log: Vec<(u32, usize, Card)>,
    turns: OpponentTurns,
}

/// Why a search for `player_index` cannot run on `state`, if it cannot.
///
/// The shared search panics when a position offers the searching player no
/// choices, and a panic compiled to wasm reaches JS only as
/// `RuntimeError: unreachable` — no message, no location. The web AI is asked
/// from an async view that can describe a turn that has already ended, so it
/// checks here and reports a real error instead of trapping.
///
/// Returns `None` when a search is safe to run.
pub fn search_refusal(state: &GameState, player_index: usize) -> Option<String> {
    if player_index >= state.players.len() {
        return Some(format!("player {player_index} is not seated at this table"));
    }
    if matches!(state.phase, GamePhase::GameOver) {
        return Some("the game is over".to_string());
    }
    if enumerate_choices(state, player_index).is_empty() {
        let phase = match &state.phase {
            GamePhase::Draw => "draw",
            GamePhase::Buyers { .. } => "buyers",
            GamePhase::Draft { .. } => "draft",
            GamePhase::Action { .. } => "action",
            GamePhase::GameOver => "game over",
        };
        return Some(format!(
            "player {player_index} has no legal choices in the {phase} phase"
        ));
    }
    None
}

impl GameState {
    /// Whose choice the search makes next. In the action phase that is always
    /// the searching player: `advance` has already played every opponent's
    /// turn, and the round ends with the searcher's.
    fn search_actor(&self, ctx: &SearchContext) -> usize {
        match &self.phase {
            GamePhase::Buyers { buyers_state } => buyers_state.current_player_index,
            GamePhase::Draft { draft_state } => draft_state.current_player_index,
            GamePhase::Action { action_state } => {
                debug_assert!(
                    !action_state.turns[ctx.perspective].ended,
                    "The searcher's turn is over but the round is not, so an opponent \
                     was left unplayed"
                );
                ctx.perspective
            }
            GamePhase::Draw | GamePhase::GameOver => 0,
        }
    }
}

impl Game for GameState {
    type Choice = Choice;
    type Rewards = Rewards;
    type Context = SearchContext;
    type Side = OpponentModel;

    // Branching runs from a handful to about forty. Comparing two choices
    // walks their payloads, so the hash index pays from nine children up:
    // measured against 16 on GA-shaped games, 1.1% fewer cycles.
    const CHILD_INDEX_THRESHOLD: usize = 8;

    // What the searching player may do at the root depends on opponents' draft
    // picks, which vary by iteration. See
    // `root_choice_set_varies_so_the_fast_path_stays_off`.
    const CHOICES_ON_REVISIT: ChoicesOnRevisit = ChoicesOnRevisit::MayChange;

    fn status(&self, ctx: &SearchContext) -> Status<Rewards> {
        if matches!(self.phase, GamePhase::GameOver) {
            return Status::Terminal(compute_terminal_rewards(&self.players));
        }
        if ctx.max_rollout_round.is_some_and(|mr| self.round > mr) {
            return Status::Terminal(ctx.evaluate(self));
        }
        Status::Active {
            player: self.search_actor(ctx) as u8,
        }
    }

    fn choices_into(&self, ctx: &SearchContext, out: &mut Vec<Self::Choice>) {
        enumerate_choices_into(self, self.search_actor(ctx), out);
    }

    // In the action phase of the round a search began in, every iteration
    // starts the searcher's turn the same way and nothing in a turn is random,
    // so a position's choices follow from the moves above it. Elsewhere a
    // hidden draft pick or a draw can change them. `transposition_key` merges
    // over the same stretch for the same reason.
    fn choices_fixed(&self, ctx: &SearchContext, _at_root: bool) -> bool {
        matches!(self.phase, GamePhase::Action { .. }) && ctx.merge_round == Some(self.round)
    }

    fn apply_choice<R: Rng + ?Sized>(
        &mut self,
        ctx: &SearchContext,
        choice: &Self::Choice,
        mut rng: &mut R,
    ) {
        let actor = self.search_actor(ctx);
        apply_choice_to_state(self, actor, choice, &mut rng);
    }

    fn rollout<R: Rng + ?Sized>(&mut self, ctx: &SearchContext, mut rng: &mut R) -> Rewards {
        if ctx.no_rollout {
            return ctx.evaluate(self);
        }
        for _ in 0..ctx.max_rollout_steps {
            if matches!(self.phase, GamePhase::GameOver) {
                return compute_terminal_rewards(&self.players);
            }
            if ctx.max_rollout_round.is_some_and(|mr| self.round > mr) {
                return ctx.evaluate(self);
            }
            if ctx.heuristic_rollout {
                apply_heuristic_rollout_step(
                    self,
                    ctx.heuristic_draft,
                    &ctx.heuristic_params,
                    &mut rng,
                );
            } else {
                apply_rollout_step(self, ctx.heuristic_draft, &ctx.heuristic_params, &mut rng);
            }
        }
        ctx.evaluate(self)
    }

    fn new_buffer(&self) -> Self {
        self.clone()
    }

    fn determinize_into<R: Rng + ?Sized>(
        &self,
        dest: &mut Self,
        ctx: &SearchContext,
        perspective: u8,
        mut rng: &mut R,
    ) {
        determinize_in_place(
            dest,
            self,
            perspective as usize,
            &ctx.cached_whole_points,
            &mut rng,
        );
        // A search-policy flag that prunes legal moves, so it has to survive
        // being overwritten by the clone inside determinization.
        dest.force_max_workshop = ctx.force_max_workshop;
    }

    fn advance<R: Rng + ?Sized>(
        &mut self,
        ctx: &SearchContext,
        side: &mut OpponentModel,
        perspective: u8,
        mut rng: &mut R,
    ) {
        advance_past_opponent_draft_picks(
            self,
            perspective as usize,
            &mut side.draft_stats,
            &mut side.pick_log,
            ctx.exploration_constant,
            &mut rng,
        );
        // The draft may just have ended, so this runs after it.
        if matches!(self.phase, GamePhase::Action { .. }) {
            let policy = TurnPolicy {
                heuristic: ctx.heuristic_rollout,
                params: &ctx.heuristic_params,
            };
            side.turns.play(
                self,
                perspective as usize,
                ctx.exploration_constant,
                &policy,
                &mut rng,
            );
        }
    }

    fn init_side(&self, _ctx: &SearchContext, side: &mut OpponentModel) {
        *side = OpponentModel {
            turns: OpponentTurns::for_root(self),
            ..OpponentModel::default()
        };
    }

    fn heuristic_bias(&self, ctx: &SearchContext, perspective: u8) -> f32 {
        heuristic_score(
            &self.players[perspective as usize],
            &self.card_lookup,
            &ctx.heuristic_params,
            &ctx.card_table,
        ) as f32
    }

    fn begin_iteration(side: &mut OpponentModel) {
        side.pick_log.clear();
        side.turns.begin_iteration();
    }

    fn credit_iteration(side: &mut OpponentModel, rewards: &Rewards) {
        for &(pick_round, player, card) in &side.pick_log {
            side.draft_stats
                .record_outcome(pick_round as usize, player, card, rewards[player]);
        }
        side.turns.credit(rewards);
    }
}

/// Upper bound on any one player's reward, for the early-termination proof.
///
/// Multiplayer rewards are a winner's share of 1.0, so the bound is exact. Solo
/// scoring adds `score / 100` on top of the win bonus and can exceed 1.0, so it
/// gets a looser bound — too high only makes the proof more conservative, while
/// too low would make it unsound.
fn reward_ceiling(state: &GameState) -> f64 {
    if state.players.len() == 1 {
        2.0
    } else {
        1.0
    }
}

/// Shape of a finished tree, for display.
#[derive(Clone, Copy, Debug)]
pub struct TreeStats {
    pub total_nodes: usize,
    pub max_depth: usize,
    pub avg_branching_factor: f64,
}

/// Summarise a tree for display. Counts only expanded nodes, so the branching
/// factor is what the search actually explored rather than what was legal.
pub fn tree_stats(node: &SearchNode) -> TreeStats {
    let mut total_nodes = 0usize;
    let mut internal = 0usize;
    let mut children_total = 0usize;

    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        total_nodes += 1;
        let children = current.children();
        if !children.is_empty() {
            internal += 1;
            children_total += children.len();
        }
        stack.extend(children.iter());
    }

    TreeStats {
        total_nodes,
        max_depth: node.max_depth(),
        avg_branching_factor: if internal == 0 {
            0.0
        } else {
            children_total as f64 / internal as f64
        },
    }
}

/// [`tree_stats`] for a transposition search's graph: every position counted
/// once however many routes reach it, and the branching factor over the links
/// the search opened.
pub fn graph_stats(graph: &SearchGraph) -> TreeStats {
    let mut internal = 0usize;
    let mut links = 0usize;
    for id in 0..graph.node_count() {
        let node = graph.node(id).expect("every id below the count names a node");
        let opened = node.edges().len();
        if opened != 0 {
            internal += 1;
            links += opened;
        }
    }
    TreeStats {
        total_nodes: graph.node_count(),
        max_depth: graph.max_depth(),
        avg_branching_factor: if internal == 0 {
            0.0
        } else {
            links as f64 / internal as f64
        },
    }
}

/// What a search decided.
pub struct SearchOutcome {
    pub choice: Choice,
    /// Iterations run by this call, excluding any inherited from a reused tree.
    pub iterations_used: u32,
    pub reused_iterations: u32,
    pub stop_reason: mcts::StopReason,
}

/// Holds the reusable buffers and the retained tree across a player's moves.
///
/// Keep one per player for the length of a game and call [`Self::search`] each
/// turn; [`Self::reuse_subtree`] carries the relevant subtree forward. Searches
/// as a tree, or as a transposition graph when [`MctsConfig::transpositions`] is
/// on; switching between the two drops whatever was retained.
pub struct ColoriSearcher {
    inner: Inner,
}

enum Inner {
    Tree(mcts::Searcher<GameState>),
    Graph(mcts::GraphSearcher<GameState>),
}

impl ColoriSearcher {
    pub fn new(state: &GameState) -> Self {
        Self {
            inner: Inner::Tree(mcts::Searcher::new(state)),
        }
    }

    pub fn search<R: Rng>(
        &mut self,
        state: &GameState,
        player_index: usize,
        config: &MctsConfig,
        max_rollout_round: Option<u32>,
        rng: &mut R,
    ) -> SearchOutcome {
        let context = SearchContext::new(
            state,
            player_index,
            &config.heuristic_params,
            config.use_heuristic_eval,
            max_rollout_round,
            config.heuristic_rollout,
            config.heuristic_draft,
            config.max_rollout_steps,
            config.no_rollout,
            config.force_max_workshop,
            config.exploration_constant,
        );

        let search_config = mcts::Config {
            iterations: config.iterations,
            time_limit_ms: config.time_limit_ms,
            exploration_constant: config.exploration_constant,
            progressive_bias_weight: config.progressive_bias_weight,
            early_termination: config.early_termination,
            min_reward: 0.0,
            max_reward: reward_ceiling(state),
            // Knobs that apply only at a simultaneous node. The tree never
            // holds one: opponents' simultaneous moves are played outside it,
            // by `OpponentModel`.
            simultaneous: mcts::SimultaneousConfig::default(),
        };

        match (&self.inner, config.transpositions) {
            (Inner::Tree(_), true) => self.inner = Inner::Graph(mcts::GraphSearcher::new(state)),
            (Inner::Graph(_), false) => self.inner = Inner::Tree(mcts::Searcher::new(state)),
            _ => {}
        }
        let perspective = player_index as u8;
        let result = match &mut self.inner {
            Inner::Tree(searcher) => {
                searcher.search(state, &context, perspective, &search_config, None, rng)
            }
            Inner::Graph(searcher) => {
                searcher.search(state, &context, perspective, &search_config, None, rng)
            }
        };

        SearchOutcome {
            choice: result.choice,
            iterations_used: result.iterations_used,
            reused_iterations: result.reused_iterations,
            stop_reason: result.stop_reason,
        }
    }

    /// Re-root at `choice` so the next search inherits its statistics. Only
    /// sound when the same player's next search continues from that position
    /// with no hidden information revealed in between; the caller owns that
    /// judgement. The tree is that player's: in an action phase it holds none
    /// of anyone else's moves.
    pub fn reuse_subtree(&mut self, choice: &Choice) -> bool {
        match &mut self.inner {
            Inner::Tree(searcher) => searcher.reuse_subtree(choice),
            Inner::Graph(searcher) => searcher.reuse_subtree(choice),
        }
    }

    pub fn clear_tree(&mut self) {
        match &mut self.inner {
            Inner::Tree(searcher) => searcher.clear_tree(),
            Inner::Graph(searcher) => searcher.clear_graph(),
        }
    }

    /// The retained tree, when searching as a tree.
    pub fn tree(&self) -> Option<&SearchNode> {
        match &self.inner {
            Inner::Tree(searcher) => searcher.tree(),
            Inner::Graph(_) => None,
        }
    }

    /// The retained graph, when searching for transpositions.
    pub fn graph(&self) -> Option<&SearchGraph> {
        match &self.inner {
            Inner::Graph(searcher) => searcher.graph(),
            Inner::Tree(_) => None,
        }
    }

    /// The shape of whatever is retained, tree or graph.
    pub fn stats(&self) -> Option<TreeStats> {
        match &self.inner {
            Inner::Tree(searcher) => searcher.tree().map(tree_stats),
            Inner::Graph(searcher) => searcher.graph().map(graph_stats),
        }
    }

    /// Take whatever is retained, for handing to something that outlives this
    /// searcher — the GUI renders it on another thread.
    pub fn take_retained(&mut self) -> Option<Retained> {
        match &mut self.inner {
            Inner::Tree(searcher) => searcher.take_tree().map(Retained::Tree),
            Inner::Graph(searcher) => searcher.take_graph().map(Retained::Graph),
        }
    }
}

/// A finished search's tree or graph, owned.
pub enum Retained {
    Tree(SearchNode),
    Graph(SearchGraph),
}

impl Retained {
    /// Iterations the root has seen.
    pub fn visits(&self) -> u32 {
        match self {
            Retained::Tree(root) => root.visits(),
            Retained::Graph(graph) => graph.root().visits(),
        }
    }

    pub fn stats(&self) -> TreeStats {
        match self {
            Retained::Tree(root) => tree_stats(root),
            Retained::Graph(graph) => graph_stats(graph),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::choices::check_choice_available;
    use crate::colori_game::{get_game_status, GameStatus};
    use crate::draw_phase::execute_draw_phase;
    use crate::setup::create_initial_game_state;
    use rand::SeedableRng;
    use wyrand::WyRand;

    fn params() -> HeuristicParams {
        const PARAMS_JSON: &str = include_str!("../../genetic-algorithm/batch-lki08w-gen-32.json");
        serde_json::from_str(PARAMS_JSON).expect("frozen heuristic params should parse")
    }

    fn config(iterations: u32) -> MctsConfig {
        MctsConfig {
            iterations,
            ..MctsConfig::new(params())
        }
    }

    /// Every move the search returns must be one the game would actually allow.
    fn play_full_game_checking_legality(num_players: usize, seed: u64) {
        play_full_game(num_players, seed, &config(10), false);
    }

    /// [`play_full_game_checking_legality`] under `config`, re-rooting the
    /// searcher whenever the same player is still acting if `reuse` is set —
    /// the runner's policy, and the path where a retained graph meets a new
    /// search.
    fn play_full_game(num_players: usize, seed: u64, config: &MctsConfig, reuse: bool) {
        let mut rng = WyRand::seed_from_u64(seed);
        let ai_players = vec![true; num_players];
        let mut state = create_initial_game_state(num_players, &ai_players, &mut rng);
        execute_draw_phase(&mut state, &mut rng);

        let mut searchers: Vec<ColoriSearcher> = (0..num_players)
            .map(|_| ColoriSearcher::new(&state))
            .collect();
        let mut choices_buf: Vec<Choice> = Vec::new();

        for step in 0..5_000 {
            match &state.phase {
                GamePhase::GameOver => return,
                GamePhase::Draw => {
                    execute_draw_phase(&mut state, &mut rng);
                    continue;
                }
                _ => {}
            }

            let player_index = match get_game_status(&state, None) {
                GameStatus::AwaitingAction { player_index } => player_index,
                GameStatus::Terminated { .. } => return,
            };

            let max_rollout_round = Some(std::cmp::max(8, state.round + 2));
            // Deliberately no clear_tree: searchers are held across moves, and
            // the library must not carry a stale tree into a new position.
            let outcome = searchers[player_index].search(
                &state,
                player_index,
                config,
                max_rollout_round,
                &mut rng,
            );
            let choice = outcome.choice;

            enumerate_choices_into(&state, player_index, &mut choices_buf);
            assert!(
                choices_buf.contains(&choice),
                "seed={seed} players={num_players} step={step} round={} phase={:?}: \
                 chose {choice:?}, which is not an enumerated choice",
                state.round,
                state.phase
            );
            assert!(
                check_choice_available(&state, player_index, &choice),
                "seed={seed} players={num_players} step={step} round={} phase={:?}: \
                 chose {choice:?}, which is not available",
                state.round,
                state.phase
            );

            apply_choice_to_state(&mut state, player_index, &choice, &mut rng);
            if reuse {
                let same_player = matches!(
                    get_game_status(&state, None),
                    GameStatus::AwaitingAction { player_index: next } if next == player_index
                );
                if same_player {
                    searchers[player_index].reuse_subtree(&choice);
                } else {
                    searchers[player_index].clear_tree();
                }
            }
        }

        panic!("seed={seed} players={num_players}: game did not finish in 5000 steps");
    }

    #[test]
    fn valid_moves_2_players() {
        for seed in 0..5 {
            play_full_game_checking_legality(2, seed);
        }
    }

    #[test]
    fn valid_moves_3_players() {
        for seed in 0..5 {
            play_full_game_checking_legality(3, seed);
        }
    }

    #[test]
    fn valid_moves_4_players() {
        for seed in 0..5 {
            play_full_game_checking_legality(4, seed);
        }
    }

    /// A seat whose turn has ended, or a finished game, is refused before the
    /// search rather than trapping inside it.
    #[test]
    fn search_refuses_a_position_with_no_choices() {
        let mut rng = WyRand::seed_from_u64(3);
        let mut state = create_initial_game_state(3, &[true; 3], &mut rng);
        execute_draw_phase(&mut state, &mut rng);
        while !matches!(state.phase, GamePhase::Action { .. }) {
            if matches!(state.phase, GamePhase::Draw) {
                execute_draw_phase(&mut state, &mut rng);
                continue;
            }
            let GameStatus::AwaitingAction { player_index } = get_game_status(&state, None) else {
                unreachable!("round 1 reaches its action phase");
            };
            let choices = enumerate_choices(&state, player_index);
            let pick = rng.next_u64() % choices.len() as u64;
            apply_choice_to_state(&mut state, player_index, &choices[pick as usize], &mut rng);
        }
        assert!(search_refusal(&state, 0).is_none());

        // Play seat 0's turn out, leaving the others still acting.
        while matches!(&state.phase, GamePhase::Action { action_state } if !action_state.turns[0].ended)
        {
            let choices = enumerate_choices(&state, 0);
            let pick = rng.next_u64() % choices.len() as u64;
            apply_choice_to_state(&mut state, 0, &choices[pick as usize], &mut rng);
        }
        assert!(matches!(state.phase, GamePhase::Action { .. }));
        let refusal = search_refusal(&state, 0).expect("an ended turn has no choices");
        assert!(refusal.contains("no legal choices"), "{refusal}");
    }

    /// Whole games through the transposition graph. Debug builds recompute every
    /// merged position's key on every visit and panic if it moved, so this is
    /// where a key that read an opponent's hidden picks or a round's random draw
    /// would show — every search during a draft reaches the action phase below
    /// its root.
    #[test]
    fn valid_moves_through_the_graph() {
        let config = MctsConfig {
            transpositions: true,
            ..config(150)
        };
        for players in [2, 3, 4] {
            play_full_game(players, 7, &config, true);
        }
    }

    #[test]
    fn a_graph_search_merges_a_turns_positions() {
        let mut rng = WyRand::seed_from_u64(19);
        let mut state = create_initial_game_state(3, &[true; 3], &mut rng);
        execute_draw_phase(&mut state, &mut rng);
        // Random play to the start of round 2's action phase: every player's
        // view of it, since nobody has moved.
        while !(state.round == 2 && matches!(state.phase, GamePhase::Action { .. })) {
            if matches!(state.phase, GamePhase::Draw) {
                execute_draw_phase(&mut state, &mut rng);
                continue;
            }
            let GameStatus::AwaitingAction { player_index } = get_game_status(&state, None) else {
                unreachable!("round 2 is reached");
            };
            let mut choices = Vec::new();
            enumerate_choices_into(&state, player_index, &mut choices);
            let choice = choices[(rng.next_u64() % choices.len() as u64) as usize].clone();
            apply_choice_to_state(&mut state, player_index, &choice, &mut rng);
        }
        let player = 1;
        let config = MctsConfig {
            transpositions: true,
            ..config(3_000)
        };
        let mut searcher = ColoriSearcher::new(&state);
        searcher.search(&state, player, &config, Some(4), &mut rng);
        let graph = searcher.graph().expect("a transposition search leaves a graph");

        let mut keyed = std::collections::HashSet::new();
        let mut entered_by_links = 0u64;
        for id in 0..graph.node_count() {
            let node = graph.node(id).expect("a node");
            if let Some(key) = node.key() {
                assert!(keyed.insert((*key, node.reward_player())), "a position has two vertices");
            }
            entered_by_links += node.edges().map(|e| e.visits() as u64).sum::<u64>();
        }
        assert!(!keyed.is_empty(), "nothing in the turn was merged under a key");
        let positions_entered: u64 = (0..graph.node_count())
            .filter_map(|id| graph.node(id))
            .filter(|node| !node.is_root())
            .map(|node| node.visits() as u64)
            .sum();
        assert_eq!(entered_by_links, positions_entered);
    }

    /// The root's choice set is *not* stable across determinizations, which is
    /// why `CHOICES_ON_REVISIT` claims nothing. `choices_fixed` claims only the
    /// action-phase turn a search begins in, where every iteration starts the
    /// same way.
    ///
    /// An earlier version of this test claimed the opposite and passed, because
    /// it determinized and then enumerated without running `advance` in
    /// between. The real search advances past opponent draft picks first, and
    /// those picks vary per iteration, so what the searching player may then do
    /// varies too. With the flag on, a 300-game tournament panicked applying an
    /// illegal move; with it off, the same 300 games are clean.
    ///
    /// There is no assertion to make here that would not be flaky — the guard
    /// is in the library, which re-enumerates the root every iteration under
    /// `debug_assertions` and panics if the set disagrees with the tree. Any
    /// attempt to turn the flag back on fails in the tests above.
    #[test]
    fn root_choice_set_varies_so_the_fast_path_stays_off() {
        assert_eq!(
            <GameState as Game>::CHOICES_ON_REVISIT,
            ChoicesOnRevisit::MayChange
        );
    }

    #[test]
    fn same_seed_gives_the_same_choice() {
        let mut setup = WyRand::seed_from_u64(41);
        let ai_players = vec![true; 3];
        let mut state = create_initial_game_state(3, &ai_players, &mut setup);
        execute_draw_phase(&mut state, &mut setup);

        let config = config(200);
        let run = || {
            let mut rng = WyRand::seed_from_u64(7);
            let mut searcher = ColoriSearcher::new(&state);
            let outcome = searcher.search(&state, 0, &config, Some(8), &mut rng);
            (outcome.choice, outcome.iterations_used)
        };
        assert_eq!(run(), run());
    }
}
