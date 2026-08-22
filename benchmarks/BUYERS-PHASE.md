# Cost of the buyers phase

Sell cards are no longer completed out of the shared display. A new phase runs
before each draft in which players claim cards into a personal buyers section,
and only those can be sold to. That adds a phase, adds decisions per round, and
changes what evaluation looks at — so this records what it costs.

Method and machine are the same as `DECK-ORDERING.md`: retired instructions via
`proc_pid_rusage(RUSAGE_INFO_V4)` on an Apple M4, never wall clock.

```sh
cargo run --release -p colori-core --example instruction_count
```

Baseline is that harness at `23bb30b8`, the commit before this change.

## Result

| | before | after | change |
|---|---:|---:|---:|
| **instructions per game** | 13.016 G | 12.841 G | **−1.3%** |
| instructions per iteration | 98 606 | 84 931 | −13.9% |
| iterations per game | 132 001 | 151 188 | +14.5% |
| decisions per game | 162 | 184 | +13.6% |
| mean branching | 9.38 | 10.02 | +6.8% |
| `GameState` | 3 040 B | 3 152 B | +3.7% |
| `PlayerState` | 528 B | 560 B | +6.1% |

12 seeded 3-player games, 1 000 iterations per move, production config.

**Roughly free.** A whole extra phase, 13.6% more decisions per game and 14.5%
more search iterations, and the game still costs slightly *less* to play out
than before, because each iteration got 13.9% cheaper.

## Why the iterations got cheaper

Two things, both consequences of selling moving off the shared display.

**Evaluation scans less.** `heuristic_score` finds the best-aligned sell card by
scanning them one at a time. It used to scan the six-card display; it now scans
a player's at most three buyers. That runs at every leaf.

**So does the rollout.** `SellCardCache` is rebuilt at every action node of a
heuristic rollout and was sized to the display; it is now sized to the buyers.

**And the new phase is cheap per node.** The rollout resolves an entire buyers
phase in one step, the way it already did for the draft, so the extra decisions
cost tree nodes rather than rollout work. Claiming is at most six choices —
five face-up cards deduplicated by type, plus the deck — against 20 to 60 at a
typical action node.

The `+14.5%` in iterations per game is not extra work per decision: it is
`early_termination` having 22 more decisions per game to run at.

## What is not comparable here

**The per-position figures.** The harness picks the first action node in a given
round with branching of at least 20. Adding a phase moves the seeded walk so far
that the three positions found are not the same kind of node as before —
branching went 26 → 37, 49 → 23 and 34 → 59 — and the numbers should not be read
as a per-position regression or improvement. The `early` position also came in
at 1.32% spread, well above this harness's usual 0.05–0.3%. The per-game figures
carry the conclusion.

## A balance effect worth naming

Mean score fell from 13.72 to 13.19 (−3.9%). This is not a strength measurement
— every player in these games runs the same weights — but it does say players
complete fewer sell cards, which is what the rules change should do. Selling now
needs a commitment made a phase in advance, to at most three cards, instead of
opportunistic buying from six.

The one-slot first round is the tightest constraint: a player can complete at
most one sell card in round 1, whatever they draw.

## Caveats

**The AI is playing new rules with old weights, again.**
`batch-lki08w-gen-32.json` predates both this change and the deck rework.
`sell_card_material_alignment` and `sell_card_color_alignment` now score a
player's own buyers rather than a shared display — the same terms measuring a
different thing — and nothing has been retrained against it. The buyers-phase
rollout policy uses those same weights to decide what to claim, so a retrained
set would move both the play and the cost figures above.

**Archived game logs still do not replay**, for the same reason as the deck
change plus a new one: a player's `buyers` are part of the state, and claiming
off the deck is a hidden draw recorded as a new `buyerDraw` event.
