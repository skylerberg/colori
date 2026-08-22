# Cost of ordering the personal deck

The rules changed so that a round's leftover workshop is shuffled onto the
**bottom** of the player's deck, kept draft-pool cards become the next round's
workshop, and the discard pile is gone. That breaks the assumption the fast
deck representation rested on — that a deck is one uniformly shuffled bag — so
this records what the replacement costs.

Machine: Apple M4, `aarch64-apple-darwin`. Toolchain: `rustc 1.93.1`.
Build: workspace `lto = true`, `codegen-units = 1`, plus `target-cpu=native`
from `.cargo/config.toml`.

## How this was measured

**Not wall clock.** `cargo bench` reports elapsed time, which on a desktop
running other work measures the machine's mood as much as the engine. macOS
exposes a per-process retired-instruction counter through
`proc_pid_rusage(RUSAGE_INFO_V4)`, needs no privileges, and is unaffected by
other processes competing for cores.

```sh
cargo run --release -p colori-core --example instruction_count
```

Baseline is that harness run at commit `bc10d72a`, which added it and changed
nothing else; the "after" column is the same harness on the finished change.

Two things about it are load-bearing:

**Positions are chosen by round and width, not by step index.**
`benches/ismcts_bench.rs` walks a seeded game forward a fixed number of steps.
That is reproducible while the game is fixed and worthless across a rules
change, because the same index lands somewhere else entirely and a slower
engine is indistinguishable from a harder position. The harness instead stops
at the first action node in a given round offering at least 20 legal choices,
and prints the branching it actually found, so the remaining drift is visible
rather than assumed away.

**Instructions are trustworthy; cycles are not.** Repeats of an identical
workload agree to within 0.05–0.36% on instructions. Cycles for the same
workload moved 6% between two runs of the *unchanged* baseline, because they
absorb frequency scaling and cache pressure from whatever else the machine is
doing. IPC below is indicative only; every claim rests on instruction counts.

## Result

| | before | after | change |
|---|---:|---:|---:|
| **instructions per game** | 12.107 G | 12.988 G | **+7.3%** |
| instructions per iteration | 95 368 | 98 742 | +3.5% |
| iterations per game | 127 035 | 131 530 | +3.5% |
| decisions per game | 165 | 162 | −1.8% |
| mean branching | 8.95 | 9.56 | +6.8% |
| `GameState` | 2 336 B | 3 040 B | +30.1% |
| `PlayerState` | 352 B | 528 B | +50.0% |

12 seeded 3-player games, 1 000 iterations per move, production config.

Per fixed position, 2 000 iterations, early termination off:

| position | branching before → after | instr/iter before | after | change |
|---|---:|---:|---:|---:|
| `early` (round 1) | 26 → 26 | 140 203 | 146 906 | +4.8% |
| `middle` (round 3) | 49 → 64 | 133 445 | 140 582 | +5.3% |
| `late` (round 5) | 34 → 35 | 69 971 | 82 778 | +18.3% |

**+7.3% per game.** That is the number that matters, and it is not too bad: at
the browser's iteration budget it is well inside the latency the AI already
spends thinking.

## Where the cost went, and where it did not

The +7.3% is not one thing. It decomposes into +3.5% per iteration and +3.5%
more iterations per game, and almost none of either is the deck data structure.

**Not the bigger state.** `GameState` grew 30%, which looks alarming until it
is priced. `BASELINE.md` measured the per-iteration state copy at 1.3% of an
iteration, so 30% more of it is under half a percent. That was confirmed
directly rather than assumed: the segment cap was tightened from 8 to 7 partway
through, cutting `GameState` by 128 bytes, and instructions per game moved
0.07% — inside the 0.22% noise floor. **Deck memory is not where the time is.**

**Mostly bigger decks and wider positions.** Mean deck size went from 4.90 to
5.78 cards (+18%), because cards kept in the draft pool now survive into the
workshop and thence into the deck instead of being spent. Mean branching went
from 8.95 to 9.56 (+6.8%). Both are the *rules* costing more, not the
representation. The `late` position's +18.3% is the clearest case: rollouts
from round 5 are short, so its cost is dominated by leaf evaluation, which
loops over every card a player owns — and that population grew by the same 18%.

**More iterations, not more decisions.** Decisions per game actually *fell*
(165 → 162), so the +3.5% in iterations per game is early termination firing
less often: wider, more balanced positions take longer to prove a winner.

## What the deck is now

`colori-core/src/deck.rs`. A deck is an ordered queue of independently shuffled
segments, each an `UnorderedCards` bitset. A whole workshop joins as one
segment at the bottom; draws take the front segment and move on when it is
exhausted.

This is not a compromise between speed and correctness — it is the exact
information state. The order *within* a segment is unknown to everyone,
including its owner, so:

* a draw is still a uniform sample from a bitset, exactly as when the deck was
  one bag;
* determinization still has nothing to do for player decks, because no hidden
  order is stored to reshuffle.

Storing a concrete card order instead would have been slower *and* unsound for
ISMCTS: the search would see its own future draws unless every determinization
reshuffled the deck.

`MAX_DECK_SEGMENTS` is 7, the exact structural bound for a six-round game — the
only way to add a segment is to end a round. Measured over 8 000 random games
across 1–4 players, no deck exceeded five segments and 99.4% held three or
fewer. A longer game (`--max-rounds` in solo simulation) can outrun the cap, and
a push onto a full deck then merges into the deepest segment, losing the order
between the last two pushes at the bottom of a deck already deeper than the
remaining rounds can draw.

## Caveats

**The AI is playing new rules with old weights.** `batch-lki08w-gen-32.json`
was trained by the GA against the discard-pile game. Weights like
`deck_thinning_value` and the card-quality table are now tuned for a game that
no longer exists, and retraining was explicitly out of scope. Mean score rose
from 13.89 to 14.25, so nothing collapsed, but strength against a retrained
opponent is unmeasured and the per-game cost above will move once the GA has
run.

**Positions are similar, not identical.** The `middle` position widened from 49
to 64 legal choices between the two builds — same seed, same selection rule,
different game. Some of its +5.3% is that. The per-game figures do not have this
problem, which is why they carry the conclusion.

**Archived game logs no longer replay.** A player's `deck` serialises as an
array of segments rather than a flat list, so logs recorded before this change
fail to deserialise. Replaying them was already meaningless — they record a
game with different rules.
