#!/usr/bin/env python3
"""Attribute strength to the draft and the action phase by ablation.

Reads game logs written by `colori-runner tournament` where every game seats one
*arm* and two identical *reference* seats (three players), and reports how much
points an arm gives up by searching one phase with a smaller budget. Because
every arm faces the same references, the arms' margins are directly comparable:
the arm that loses more is missing more in that phase.

Expected variants files: `variants-phase-action-weak.json` and
`variants-phase-draft-weak.json`, run as

    colori-runner --threads 16 --output game-logs/phase-ablation/action-weak \
        tournament --games 1000 --players 3 --variants-file variants-phase-action-weak.json

Seats are told apart by name: the arm's name starts with "arm", the references'
with "ref". A game that does not seat exactly one arm and two references is
skipped and counted.
"""

import argparse
import glob
import json
import math
import statistics
import sys

Z95 = 1.96


def phase_budgets(variant):
    """The visits a seat spends on (draft, action), from the log's own record."""
    base = variant.get("iterations", 0)
    return (variant.get("draftIterations", base), variant.get("actionIterations", base))


def read_game(path):
    with open(path) as f:
        log = json.load(f)
    variants = log.get("playerVariants")
    scores = log.get("finalScores")
    if not variants or not scores or len(variants) != len(scores) or len(scores) != 3:
        return None

    arms = [i for i, v in enumerate(variants) if (v.get("name") or "").startswith("arm")]
    refs = [i for i, v in enumerate(variants) if (v.get("name") or "").startswith("ref")]
    if len(arms) != 1 or len(refs) != 2:
        return None
    arm = arms[0]

    points = [s["score"] for s in scores]
    best = max(points)
    top = points.count(best)

    decisions = {"draft": [0] * 3, "action": [0] * 3}
    for entry in log.get("entries") or []:
        phase = entry.get("phase")
        seat = entry.get("playerIndex")
        if phase in decisions and seat is not None:
            decisions[phase][seat] += 1

    draft_iters = log.get("playerDraftIterations") or [0] * 3
    action_iters = log.get("playerActionIterations") or [0] * 3

    return {
        "arm_budget": phase_budgets(variants[arm]),
        "ref_budget": phase_budgets(variants[refs[0]]),
        "points": points,
        "arm_points": points[arm],
        "ref_points": [points[i] for i in refs],
        "arm_margin": points[arm] - statistics.mean(points[i] for i in refs),
        "arm_credit": (1.0 / top) if points[arm] == best else 0.0,
        "arm_draft_iters": draft_iters[arm],
        "arm_action_iters": action_iters[arm],
        "ref_draft_iters": statistics.mean(draft_iters[i] for i in refs),
        "ref_action_iters": statistics.mean(action_iters[i] for i in refs),
        "arm_draft_decisions": decisions["draft"][arm],
        "arm_action_decisions": decisions["action"][arm],
    }


def summarize(values):
    n = len(values)
    if n == 0:
        return {"n": 0}
    mean = statistics.mean(values)
    sd = statistics.pstdev(values) if n > 1 else 0.0
    se = sd / math.sqrt(n) if n > 1 else float("nan")
    return {
        "n": n,
        "mean": mean,
        "sd": sd,
        "se": se,
        "ci": (mean - Z95 * se, mean + Z95 * se),
    }


def describe(label, games):
    margins = [g["arm_margin"] for g in games]
    credits = [g["arm_credit"] for g in games]
    stats = summarize(margins)
    if stats["n"] == 0:
        print(f"{label}: no games")
        return stats

    lo, hi = stats["ci"]
    budget = games[0]["arm_budget"]
    ref = games[0]["ref_budget"]
    draft_decisions = statistics.median(g["arm_draft_decisions"] for g in games)
    action_decisions = statistics.median(g["arm_action_decisions"] for g in games)

    print(f"\n{label}")
    print(f"  arm draft {budget[0]} / action {budget[1]}, references {ref[0]} / {ref[1]}")
    print(f"  games                 {stats['n']}")
    print(
        f"  margin vs references  {stats['mean']:+.3f} pts "
        f"(95% CI {lo:+.3f} to {hi:+.3f}, sd {stats['sd']:.2f})"
    )
    print(f"  win share             {statistics.mean(credits):.3f} (parity 0.333)")
    print(
        f"  visits spent          draft {statistics.mean(g['arm_draft_iters'] for g in games):.0f} "
        f"/ action {statistics.mean(g['arm_action_iters'] for g in games):.0f} "
        f"(references draft {statistics.mean(g['ref_draft_iters'] for g in games):.0f} "
        f"/ action {statistics.mean(g['ref_action_iters'] for g in games):.0f})"
    )
    print(
        f"  decisions per game    draft {draft_decisions:.0f} / action {action_decisions:.0f} "
        f"-> {stats['mean'] / max(draft_decisions, 1):+.4f} pts per draft decision, "
        f"{stats['mean'] / max(action_decisions, 1):+.4f} per action decision"
    )
    return stats


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--logs", nargs="+", required=True, help="log directories or globs")
    parser.add_argument("--json", help="write the per-arm summary here")
    args = parser.parse_args()

    summaries = {}
    for pattern in args.logs:
        paths = []
        for root in glob.glob(pattern):
            paths += glob.glob(root) if root.endswith(".json") else glob.glob(f"{root}/*.json")
        games, skipped = [], 0
        for path in paths:
            game = read_game(path)
            if game is None:
                skipped += 1
            else:
                games.append(game)

        label = pattern.rstrip("/").split("/")[-1]
        stats = describe(label, games)
        if skipped:
            print(f"  skipped               {skipped} games without one arm and two references")
        summaries[label] = {
            "dir": pattern,
            "skipped": skipped,
            "margin": stats,
            "win_share": statistics.mean(g["arm_credit"] for g in games) if games else None,
            "arm_budget": games[0]["arm_budget"] if games else None,
        }

    # Both arms face the same references, so the arm that is further below zero
    # is the one whose downgraded phase was doing more work.
    labels = list(summaries)
    if len(labels) == 2:
        a, b = labels
        sa, sb = summaries[a]["margin"], summaries[b]["margin"]
        if sa["n"] > 1 and sb["n"] > 1:
            diff = sb["mean"] - sa["mean"]
            se = math.sqrt(sa["se"] ** 2 + sb["se"] ** 2)
            print(
                f"\n{b} minus {a}: {diff:+.3f} pts "
                f"(95% CI {diff - Z95 * se:+.3f} to {diff + Z95 * se:+.3f})"
            )
            print(
                "  a phase whose downgrade costs more is the phase where the 4x "
                "budget was doing more work"
            )

    if args.json:
        with open(args.json, "w") as f:
            json.dump(summaries, f, indent=2)
        print(f"\nwrote {args.json}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
