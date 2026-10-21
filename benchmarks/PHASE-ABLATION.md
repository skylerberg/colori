# Which phase needs the iterations

Both phases search with the same budget, which is not obviously fair: a draft
pick offers at most four cards against the twenty to sixty of an action choice.
This ablates each phase's budget separately, by head-to-head play, to see which
one the iterations are doing more work in.

## Design

Three-player games seating **one arm and two identical references**. Every arm
faces the same yardstick, so the arms' margins are directly comparable: the arm
that falls further below zero is the arm whose downgraded phase was doing more.

| run | arm | references |
|---|---|---|
| `variants-phase-action-weak.json` | draft 4 000, action 1 000 | 4 000 / 4 000 |
| `variants-phase-draft-weak.json` | draft 1 000, action 4 000 | 4 000 / 4 000 |

1 000 games per arm, 16 threads on `liquid` (16-core x86-64), heuristic params
`genetic-algorithm/batch-alobyo-gen-45.json` — the params the difficulty ladder
prices Easy (1 000) and Medium (4 000) with.

```sh
colori-runner --threads 16 --output phase-ablation/action-weak tournament \
    --games 1000 --players 3 --variants-file variants-phase-action-weak.json
# likewise draft-weak
python3 scripts/phase_ablation.py --logs phase-ablation/action-weak phase-ablation/draft-weak
```

## Result

Margin is the arm's score minus the mean of the two references', in points.

| arm | margin | 95% CI | win share | visits: draft / action (refs) |
|---|---:|---|---:|---|
| action 1 000 | **−1.760** | −2.025 to −1.494 | 0.220 | 82 320 / **25 553** (82 330 / 86 724) |
| draft 1 000 | **−0.353** | −0.544 to −0.161 | 0.300 | **20 559** / 87 764 (82 290 / 87 404) |

An evenly matched seat wins 0.333 of the share. **Draft-weak minus action-weak:
+1.407 points, 95% CI +1.080 to +1.734.**

So cutting the action budget to a quarter costs about **five times** what cutting
the draft budget to a quarter costs. At these budgets the draft phase's decisions
are nearly saturated; the action phase's are not.

Sanity checks from the same logs: the two references are identical players, and
their head-to-head margins are −0.024 ± 0.19 and +0.200 ± 0.20 — consistent with
zero, so no label or seat bias, and about ±0.2 points of wobble per 1 000 games.

## Reading it

The visits column is not the nominal budget. A draft pick is searched from a
fresh tree and spends it exactly (20 559 ≈ 21 picks × 1 000). An action choice
re-roots into the previous decision's tree, so the nominal 1 000 also inherits
thousands of visits: the action arm's realised ratio is 3.4x, not 4x. The
measured gap is therefore *smaller* than the configured one, which cuts against
missing an action effect — it is the conservative direction.

Decisions per game are about 21 draft and 43 action, so per decision the draft
cost is −0.017 points and the action cost −0.041. The phases are not independent
(a drafted card is only worth what the action phase makes of it), so the two
costs need not add up to the cost of weakening both, which this design does not
measure.

## What this does not say

**"Draft search is not iteration-limited" is not "draft decisions don't
matter."** A four-choice decision can be near-best at 1 000 iterations and still
decide the game. The test for that is a *structural* draft arm — draft with the
full budget but pick at random — which this design leaves for a follow-up.
Likewise the reference pair at 4 000 is a yardstick, not the strongest play
available; the ladder's Hard level is 25 000.

The two arms also ran as separate tournaments, so the comparison between them
rests on the references being the same in both, which they are by construction,
and on independent samples.
