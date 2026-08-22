# Cost of the ducat economy

Players collect a ducat at the start of rounds 3, 5 and 6, and can spend a
ducat during their turn for a Workshop pick, a Mix, or a Sell. That adds a
class of turn action that did not exist, so this records what it costs.

Method and machine as in `DECK-ORDERING.md`: retired instructions via
`proc_pid_rusage(RUSAGE_INFO_V4)` on an Apple M4, never wall clock.

```sh
cargo run --release -p colori-core --example instruction_count
```

Baseline is that harness at `46ac2336`, the commit before this change.

## Result

| | before | after | change |
|---|---:|---:|---:|
| **instructions per game** | 12.540 G | 13.916 G | **+11.0%** |
| instructions per iteration | 84 022 | 86 860 | +3.4% |
| iterations per game | 149 243 | 160 214 | +7.3% |
| decisions per game | 182 | 190 | +4.4% |
| mean branching | 9.35 | 9.55 | +2.1% |
| `GameState` | 3 152 B | 3 152 B | — |

12 seeded 3-player games, 1 000 iterations per move, production config.

**+11.0% per game**, and no state growth at all: `ducats` was already a field,
so the whole feature is behaviour rather than data.

## Where it goes

Roughly two thirds of the cost is having more to decide, not deciding more
slowly.

**More decisions (+4.4%).** A purchase is two nodes, not one: `SpendDucat`,
then the bought ability's own choice. That is deliberate. Fusing them — a
`SpendDucatAndWorkshop { card_types }` in the style of the existing
`DestroyAnd*` choices — would halve the node count per purchase but multiply
the top-level menu by everything each ability could then do, at the most
visited nodes in the tree. Pushing the ability instead keeps the top-level
menu at most three choices wider and reuses every existing resolution path.

**More iterations (+7.3%).** `early_termination` running at 8 more decisions
per game. Not extra work per decision.

**Slightly dearer iterations (+3.4%).** The extra top-level choices, and
`can_buy_with_ducat` being evaluated three times per top-level enumeration.

## What the ducats are actually worth

Mean score went from 13.97 to 14.14 — **+1.2%, against +3 ducats of income per
player**. That looks wrong until you see what buying a Sell does: it spends the
same material and the same colors a drafted Sell card would have spent. It does
not add a sale, it moves one. The ducat buys the *opportunity* to sell in a
round where nothing drafted allows it, and the rest of the income goes back out
as spent points.

This bit us in the rollout. The first version of the heuristic policy bought a
Sell whenever one was affordable, on the reasoning that paying 1 for a card
worth 2–4 pays for itself. It does not, if you were holding a drafted Sell card
that would have done it free. Restricting the purchase to when no drafted card
offers Sell moved mean score from 13.94 to 14.14 and cut instructions per game
by 3%. The plausible-sounding version of the rule was simply wrong, and the
measurement is what said so.

## Caveats

**The GA weights predate this and the two changes before it.** Nothing values a
ducat as anything but the point it is worth at scoring: `heuristic_score` reads
`cached_score`, which counts ducats at face value and knows nothing about what
one can buy. A retrained set would be the first thing to change these numbers,
and the rollout's purchase policy above is hand-written for the same reason.

**The per-position figures are not a before/after.** As with the buyers phase,
the seeded walk lands on different positions once the game changes; branching
at the three probes went 37 → 37, 23 → 24 and 59 → 51. The per-game figures
carry the conclusion.
