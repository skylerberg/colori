# Search performance baseline

Captured before migrating onto the shared `mcts` crate, so the migration can be
judged against something rather than asserted. This implementation is the
performance bar: it is the faster and more advanced of the two the shared crate
has to replace, so a regression here is the thing most likely to sink the work.

Machine: Apple M4, 4 P-cores + 6 E-cores, 32 GB, `aarch64-apple-darwin`.
Toolchain: `rustc 1.93.1 (01f6ddf75 2026-02-11)`.
Baseline commit: `c1a08da4`.
Build: workspace `lto = true`, `codegen-units = 1`, plus `target-cpu=native`
from `.cargo/config.toml` — all already in place.

## How to reproduce

```sh
cargo bench -p colori-core --bench ismcts_bench -- --save-baseline pre-migration
# later, to compare:
cargo bench -p colori-core --bench ismcts_bench -- --baseline pre-migration
```

The bench walks a seeded game forward by picking uniformly among legal choices,
then times one search from where it stops. Positions are a pure function of
`(seed, steps)`, so nothing is checked in and a clean checkout reproduces them.

| name | steps | round | branching |
|---|---:|---:|---:|
| `early` | 12 | 1 | 26 |
| `middle` | 50 | 2 | 41 |
| `late` | 110 | 5 | 5 |

Two things about the bench are load-bearing rather than incidental:

**`early_termination` is off in every timed configuration.** With it on,
`iterations_used` varies per run, so wall-clock at a nominal iteration count
stops meaning anything and two implementations cannot be compared. It gets its
own bench instead.

**A position with one legal choice is rejected.** The search returns immediately
without iterating, so such a position would time nothing at all. The bench
asserts branching is at least 2 rather than silently reporting a very fast
search.

## Baseline, as of `c1a08da4`

| position | branching | 1 000 iterations | 10 000 iterations |
|---|---:|---:|---:|
| `early` | 26 | 9.91 ms — 100 891/s | 107.06 ms — 93 408/s |
| `middle` | 41 | 8.79 ms — 113 771/s | 95.74 ms — 104 452/s |
| `late` | 5 | 3.52 ms — 283 964/s | 51.61 ms — 193 757/s |

Throughput falls by 8–32% going from 1 000 to 10 000 iterations, which is the
tree outgrowing cache — worth remembering when reading any single-budget number.

The `late` position is roughly three times faster per iteration than the others.
Branching is only part of it: rollouts from round 5 are short, because the
`max(8, round + 2)` horizon is close by.

## Early termination

| | median |
|---|---:|
| off | 88.22 ms |
| on | **53.23 ms** |

**1.66x**, at 10 000 iterations from the `middle` position, and it cannot change
the chosen move. This is the single largest feature-level effect measured here,
and it is worth stating plainly because it is easy to lose in a migration: if
the ported implementation drops or weakens early termination, wall-clock per
game regresses by up to this much at identical per-iteration cost, and it will
look like a library problem when it is a missing feature.

Any comparison against the new implementation must therefore check average
`iterations_used` per game as well as time, or a lost feature will be
misdiagnosed.

## Fixtures

`variants.json`, the default for `simulate` and `tournament`, is gitignored, so
until now no measurement here could be reproduced from a clean checkout. Tracked
inputs now live in `benchmarks/fixtures/` — see the README there.

## Known gap: the search is not reproducible across runs

`ismcts` threads a seeded `&mut R` throughout, so a search *is* reproducible for
a fixed seed — better than the other consumer, which calls `thread_rng()` inside
its hot loop. But the seed is drawn per thread from entropy at each entry point
(`WyRand::from_rng(&mut rand::rng())`), so production runs are not repeatable.

Deriving per-thread seeds from one master seed is a prerequisite for golden
fixture tests, and is cheap. Until then, strength comparisons need thousands of
games to see through the variance that seeding would remove for free.

## What a migration has to beat

Not just these numbers. Also:

* **Subtree reuse.** Assert `iterations_used < config.iterations` on a reused
  search. Silently losing it looks like unchanged per-iteration speed with more
  iterations needed per game.
* **Allocation behaviour.** One `GameState` is allocated per search and reused
  via `clone_from`; the descent and rollout mutate it in place. Any port that
  reintroduces a clone per iteration will show up in wall-clock long before
  anyone thinks to look for it.
* **The wasm target.** `npm run bench` must not regress, and
  `src/wasm-pkg/colori_wasm_bg.wasm` must not grow much — generic
  monomorphisation bloats wasm, and there is a real latency budget behind it.
