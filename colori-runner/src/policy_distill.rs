//! Distilling the policy into a decision tree: what a tree would have to decide.
//!
//! The policy is an argmax of a weighted sum over ~50 readings. A decision tree
//! over the same readings would have to reproduce an **oblique** boundary with
//! **axis-aligned** splits, which is the worst case for a tree: it needs a
//! staircase of splits to approximate every slanted line, so the tree either
//! grows enormous or loses the fine trade-offs.
//!
//! A tree over a dozen *interpretable* quantities is a different proposition. It
//! cannot separate "trash this card for its Sell" from "trash it for its
//! Workshop", because that difference lives in the card, not the position. What
//! it can do is decide **what kind of action to take** — sell, store, mix, spend,
//! end — and hand the choice of card or buyer to a greedy rule inside that kind.
//! That is also how a good hand-written deterministic bot is shaped.
//!
//! So this measures the one thing that decides whether the approach is worth
//! building: how well a shallow tree can predict the *kind* of action the tuned
//! policy takes, from position quantities alone. High agreement means a tree is a
//! viable representation of the policy's judgement; low agreement means the
//! judgement lives in the per-choice trade-offs and a tree will not carry it.
//!
//! The rows are written as CSV so the tree can be fitted and inspected outside
//! the program that collects them.

use colori_core::colori_game::{apply_choice_to_state, enumerate_choices, ActionPhaseViews};
use colori_core::draw_phase::execute_draw_phase;
use colori_core::policy::{ChoiceVariant, CompiledPolicy, Ctx, Plan};
use colori_core::setup::create_initial_game_state;
use colori_core::types::*;
use rand::SeedableRng;
use wyrand::WyRand;

use crate::cli::PolicyDistillArgs;
use crate::policy_play;

/// The columns of the dump. Kept short and interpretable on purpose: a tree over
/// these is a tree a person can read.
pub const COLUMNS: [&str; 19] = [
    "round",
    "points_banked",
    "standing",
    "pending_points",
    "buyers_affordable",
    "best_affordable_points",
    "shortfall_units",
    "best_buyer_points",
    "actions_left",
    "workshop_cards",
    "workshop_storage",
    "mixable_pairs",
    "ducats",
    "wilds",
    "boats",
    "empty_slots",
    "purchasable",
    "pending_sell",
    "kind",
];

fn is_taking_turn(state: &GameState, seat: usize) -> bool {
    matches!(
        &state.phase,
        GamePhase::Action { action_state } if !action_state.turns[seat].ended
    )
}

/// The kind of action a choice is, at the granularity a tree could decide.
pub fn kind_of(choice: &Choice) -> &'static str {
    match ChoiceVariant::of(choice) {
        ChoiceVariant::DraftPick => "draft",
        ChoiceVariant::TakeBuyer | ChoiceVariant::DrawBuyer => "claim",
        ChoiceVariant::EndTurn => "end",
        ChoiceVariant::SpendDucat { .. } => "spend",
        ChoiceVariant::SelectSellCard { .. } | ChoiceVariant::TrashAndSell { .. } => "sell",
        ChoiceVariant::Workshop { .. }
        | ChoiceVariant::SkipWorkshop
        | ChoiceVariant::TrashAndWorkshop { .. } => "store",
        ChoiceVariant::MixAll { .. } | ChoiceVariant::TrashAndMix { .. } => "mix",
        ChoiceVariant::TrashAndMoveToDraftPool { .. } | ChoiceVariant::MoveToDraftPool { .. } => {
            "keep"
        }
        ChoiceVariant::GainSecondary { .. } => "gain",
        ChoiceVariant::TrashDraftedCard { .. } => "use",
    }
}

fn row(ctx: &Ctx) -> Vec<String> {
    vec![
        ctx.round.to_string(),
        format!("{:.1}", colori_core::scoring::score_in_half_points(ctx.player) as f64 / 2.0),
        ctx.standing.to_string(),
        ctx.pending_points.to_string(),
        ctx.buyers_affordable.to_string(),
        ctx.best_affordable_points.to_string(),
        (ctx.demanded_color_units() + ctx.demanded_material_units()).to_string(),
        ctx.best_buyer_points.to_string(),
        ctx.drafted.to_string(),
        ctx.player.workshop_cards.len().to_string(),
        format!("{:.2}", ctx.workshop_storage),
        ctx.mixable_pairs.to_string(),
        ctx.player.ducats.to_string(),
        ctx.player.wild_primaries.to_string(),
        ctx.player.boats.to_string(),
        ctx.empty_slots.to_string(),
        ctx.purchasable.to_string(),
        match ctx.pending_ability {
            Some(Ability::Sell) => "1".to_string(),
            _ => "0".to_string(),
        },
    ]
}

pub fn run_policy_distill(args: &PolicyDistillArgs, _threads: usize, _output: &str) {
    let params = policy_play::load_params(args.heuristic_params_file.as_deref());
    let genome = policy_play::load_genome(&args.genome);
    let policy = policy_play::compile(&genome, &params, "the genome");
    let plan = Plan::new(args.lookahead_width, args.lookahead);

    let mut lines = vec![COLUMNS.join(",")];
    let mut decisions = 0usize;
    for game in 0..args.games {
        let seed = args.seed_base + game as u64;
        collect_game(&policy, seed, args.players, plan, &mut lines, &mut decisions);
    }

    std::fs::write(&args.write, lines.join("\n")).expect("failed to write the dump");
    eprintln!(
        "{} decisions from {} games written to {}",
        decisions, args.games, args.write
    );
}

fn collect_game(
    policy: &CompiledPolicy,
    seed: u64,
    players: usize,
    plan: Plan,
    lines: &mut Vec<String>,
    decisions: &mut usize,
) {
    let mut rng = WyRand::seed_from_u64(seed);
    let ai = vec![true; players];
    let mut state = create_initial_game_state(players, &ai, &mut rng);
    state.force_max_workshop = true;
    execute_draw_phase(&mut state, &mut rng);

    let mut guard = 0u32;
    while !matches!(state.phase, GamePhase::GameOver) {
        guard += 1;
        if guard > 20_000 {
            return;
        }
        match &state.phase {
            GamePhase::Buyers { buyers_state } => {
                let seat = buyers_state.current_player_index;
                record(policy, &state, seat, plan, lines, decisions);
                step(policy, &mut state, &mut rng, seat, plan);
            }
            GamePhase::Draft { draft_state } => {
                let seat = draft_state.current_player_index;
                record(policy, &state, seat, plan, lines, decisions);
                step(policy, &mut state, &mut rng, seat, plan);
            }
            GamePhase::Action { .. } => {
                let mut views = ActionPhaseViews::new(&state);
                for seat in 0..players {
                    while is_taking_turn(views.view(seat), seat) {
                        let view = views.view(seat);
                        record(policy, view, seat, plan, lines, decisions);
                        let choices = enumerate_choices(view, seat);
                        let decision = policy
                            .decide_planned(view, seat, &choices, plan)
                            .expect("choices were offered");
                        apply_choice_to_state(&mut state, seat, &decision.choice, &mut rng);
                        views.apply(seat, &decision.choice);
                    }
                }
            }
            GamePhase::Draw | GamePhase::GameOver => break,
        }
    }
}

fn record(
    policy: &CompiledPolicy,
    state: &GameState,
    seat: usize,
    plan: Plan,
    lines: &mut Vec<String>,
    decisions: &mut usize,
) {
    let choices = enumerate_choices(state, seat);
    if choices.is_empty() {
        return;
    }
    let Some(decision) = policy.decide_planned(state, seat, &choices, plan) else {
        return;
    };
    let ctx = Ctx::new(state, seat, policy.card_table());
    let mut fields = row(&ctx);
    fields.push(kind_of(&decision.choice).to_string());
    lines.push(fields.join(","));
    *decisions += 1;
}

fn step(
    policy: &CompiledPolicy,
    state: &mut GameState,
    rng: &mut WyRand,
    seat: usize,
    plan: Plan,
) {
    let choices = enumerate_choices(state, seat);
    let Some(decision) = policy.decide_planned(state, seat, &choices, plan) else {
        return;
    };
    apply_choice_to_state(state, seat, &decision.choice, rng);
}
