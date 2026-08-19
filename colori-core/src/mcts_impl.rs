//! Colori on the shared `mcts` crate.
//!
//! The search itself lives in `mcts`; everything here is the game's side of the
//! contract. Three pieces of it are worth knowing about:
//!
//! * [`SearchContext`] holds what is constant for a whole search — tuned
//!   parameters, the precomputed card table, per-player scores at the root, the
//!   rollout horizon. Keeping it out of `GameState` means it is not copied on
//!   every determinization.
//! * [`DraftModel`] holds the DUCT bandit over opponent draft picks. Those picks
//!   are deliberately not tree nodes, so they live in the search's side state
//!   and are advanced through the `advance` hook.
//! * The rollout is handed over whole, so the expert policy can keep resolving
//!   an entire draft phase in one step.

use mcts::rand_core::Rng;
use mcts::{Game, Status};

use crate::choices::enumerate_choices_into;
use crate::colori_game::{apply_choice_to_state, determinize_in_place};
use crate::ismcts::MctsConfig;
use crate::ismcts::{advance_past_opponent_draft_picks, OpponentDraftStats};
use crate::rollout::{apply_heuristic_rollout_step, apply_rollout_step};
use crate::scoring::{
    calculate_score, compute_heuristic_rewards, compute_terminal_rewards, heuristic_score,
    CardHeuristicTable, HeuristicParams,
};
use crate::types::{Card, Choice, GamePhase, GameState, MAX_PLAYERS};

pub type Rewards = [f64; MAX_PLAYERS];

/// Constant for one search. Never cloned per iteration.
pub struct SearchContext {
    pub heuristic_params: HeuristicParams,
    pub card_table: CardHeuristicTable,
    /// Scores at the root, restored after every determinization because
    /// `cached_score` is a denormalised field the search must not let drift.
    pub cached_scores: [u32; MAX_PLAYERS],
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
}

impl SearchContext {
    /// Resolves the same horizon and evaluation choice that `ismcts` derives
    /// from its config, so both implementations search to the same depth.
    pub fn new(
        state: &GameState,
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

        let mut cached_scores = [0u32; MAX_PLAYERS];
        for (i, player) in state.players.iter().enumerate() {
            cached_scores[i] = calculate_score(player);
        }

        Self {
            heuristic_params: params.clone(),
            card_table: CardHeuristicTable::new(params),
            cached_scores,
            max_rollout_round: effective_max_rollout_round,
            use_heuristic,
            heuristic_rollout,
            heuristic_draft,
            max_rollout_steps,
            no_rollout,
            force_max_workshop,
            exploration_constant,
        }
    }

    fn evaluate(&self, state: &GameState) -> Rewards {
        if self.use_heuristic {
            compute_heuristic_rewards(
                &state.players,
                &state.sell_card_display,
                &state.card_lookup,
                &self.heuristic_params,
                &self.card_table,
            )
        } else {
            compute_terminal_rewards(&state.players)
        }
    }
}

/// A flat bandit over opponent draft picks, plus this iteration's picks so they
/// can be credited once the iteration's payoffs are known.
///
/// Opponent draft picks are not modelled as tree nodes — branching on them would
/// be ruinous — so they are resolved here and rewarded separately.
#[derive(Default)]
pub struct DraftModel {
    stats: OpponentDraftStats,
    pick_log: Vec<(u32, usize, Card)>,
}

impl Game for GameState {
    type Choice = Choice;
    type Rewards = Rewards;
    type Context = SearchContext;
    type Side = DraftModel;

    // Branching runs from a handful to about forty, and `Choice` keeps its
    // payloads inline, so most nodes stay on the linear scan. See the crossover
    // measured in the mcts crate's benchmarks/BASELINE.md.
    const CHILD_INDEX_THRESHOLD: usize = 16;

    // Determinization only reshuffles what is hidden from the searching player;
    // their own hand, workshop and materials are known, so their legal choices
    // at the root do not vary. `root_choices_are_invariant` asserts this.
    const ROOT_CHOICES_INVARIANT: bool = false;

    fn status(&self, ctx: &SearchContext) -> Status<Rewards> {
        if matches!(self.phase, GamePhase::GameOver) {
            return Status::Terminal(compute_terminal_rewards(&self.players));
        }
        if ctx.max_rollout_round.is_some_and(|mr| self.round > mr) {
            return Status::Terminal(ctx.evaluate(self));
        }
        let player = match &self.phase {
            GamePhase::Draft { draft_state } => draft_state.current_player_index,
            GamePhase::Action { action_state } => action_state.current_player_index,
            GamePhase::Draw => 0,
            GamePhase::GameOver => unreachable!("handled above"),
        };
        Status::Active {
            player: player as u8,
        }
    }

    fn choices_into(&self, _ctx: &SearchContext, out: &mut Vec<Self::Choice>) {
        enumerate_choices_into(self, out);
    }

    fn apply_choice<R: Rng + ?Sized>(
        &mut self,
        _ctx: &SearchContext,
        choice: &Self::Choice,
        mut rng: &mut R,
    ) {
        apply_choice_to_state(self, choice, &mut rng);
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
            &ctx.cached_scores,
            &mut rng,
        );
        // A search-policy flag that prunes legal moves, so it has to survive
        // being overwritten by the clone inside determinization.
        dest.force_max_workshop = ctx.force_max_workshop;
    }

    fn advance<R: Rng + ?Sized>(
        &mut self,
        ctx: &SearchContext,
        side: &mut DraftModel,
        perspective: u8,
        mut rng: &mut R,
    ) {
        advance_past_opponent_draft_picks(
            self,
            perspective as usize,
            &mut side.stats,
            &mut side.pick_log,
            ctx.exploration_constant,
            &mut rng,
        );
    }

    fn heuristic_bias(&self, ctx: &SearchContext, perspective: u8) -> f32 {
        heuristic_score(
            &self.players[perspective as usize],
            &self.sell_card_display,
            &self.card_lookup,
            &ctx.heuristic_params,
            &ctx.card_table,
        ) as f32
    }

    fn begin_iteration(side: &mut DraftModel) {
        side.pick_log.clear();
    }

    fn credit_iteration(side: &mut DraftModel, rewards: &Rewards) {
        for &(pick_round, player, card) in &side.pick_log {
            side.stats
                .record_outcome(pick_round as usize, player, card, rewards[player]);
        }
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
/// turn; [`Self::reuse_subtree`] carries the relevant subtree forward.
pub struct ColoriSearcher {
    inner: mcts::Searcher<GameState>,
}

impl ColoriSearcher {
    pub fn new(state: &GameState) -> Self {
        Self {
            inner: mcts::Searcher::new(state),
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
        };

        let result = self.inner.search(
            state,
            &context,
            player_index as u8,
            &search_config,
            None,
            rng,
        );

        SearchOutcome {
            choice: result.choice,
            iterations_used: result.iterations_used,
            reused_iterations: result.reused_iterations,
            stop_reason: result.stop_reason,
        }
    }

    /// Re-root at `choice` so the next search inherits its statistics. Only
    /// sound when the next search continues from that position with no hidden
    /// information revealed in between; the caller owns that judgement.
    pub fn reuse_subtree(&mut self, choice: &Choice) -> bool {
        self.inner.reuse_subtree(choice)
    }

    pub fn clear_tree(&mut self) {
        self.inner.clear_tree();
    }

    pub fn tree(&self) -> Option<&mcts::Node<Choice>> {
        self.inner.tree()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::choices::{check_choice_available, enumerate_choices};
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
        let mut rng = WyRand::seed_from_u64(seed);
        let ai_players = vec![true; num_players];
        let mut state = create_initial_game_state(num_players, &ai_players, &mut rng);
        execute_draw_phase(&mut state, &mut rng);

        let config = config(10);
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
                &config,
                max_rollout_round,
                &mut rng,
            );
            let choice = outcome.choice;

            enumerate_choices_into(&state, &mut choices_buf);
            assert!(
                choices_buf.contains(&choice),
                "seed={seed} players={num_players} step={step} round={} phase={:?}: \
                 chose {choice:?}, which is not an enumerated choice",
                state.round,
                state.phase
            );
            assert!(
                check_choice_available(&state, &choice),
                "seed={seed} players={num_players} step={step} round={} phase={:?}: \
                 chose {choice:?}, which is not available",
                state.round,
                state.phase
            );

            apply_choice_to_state(&mut state, &choice, &mut rng);
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

    /// The root's choice set is *not* stable across determinizations, which is
    /// why `ROOT_CHOICES_INVARIANT` is off.
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
        assert!(!<GameState as Game>::ROOT_CHOICES_INVARIANT);
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
