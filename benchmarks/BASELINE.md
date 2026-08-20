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

## After the migration

Both implementations run at the same positions, same seed, same config, from
`--bench ismcts_bench`:

| position | branching | budget | legacy | shared crate | change |
|---|---:|---:|---:|---:|---:|
| `early` | 26 | 1 000 | 95 766/s | 102 036/s | +6.5% |
| `early` | 26 | 10 000 | 87 650/s | 95 912/s | +9.4% |
| `middle` | 41 | 1 000 | 100 644/s | 120 807/s | **+20.0%** |
| `middle` | 41 | 10 000 | 90 785/s | 113 434/s | **+24.9%** |
| `late` | 5 | 1 000 | 276 322/s | 282 270/s | +2.2% |
| `late` | 5 | 10 000 | 215 033/s | 227 173/s | +5.6% |

Faster everywhere, and the gain tracks branching factor: largest at `middle`
(41 children), smallest at `late` (5). That is the shape the shared crate's own
measurements predict, since it switches a node from a linear child scan to a
hash index above 16 children. The rest is not position-specific: selection
returns on the first unvisited child rather than scoring every child, expansion
reservoir-samples one unseen choice instead of building a list, and the descent
records a path of child indices instead of recursing.

> An earlier draft of this table claimed +14% to +32%. Those numbers were
> measured with `ROOT_CHOICES_INVARIANT` on, which is **unsound for this game**
> — see below. They were real measurements of a search that returns illegal
> moves, which makes them worthless.

### Why `ROOT_CHOICES_INVARIANT` is off

The shared crate can skip enumerating the root every iteration when a game's
root choice set cannot vary across determinizations. The argument that colori
qualifies is superficially strong: determinization only reshuffles what is
hidden from the searching player, so their own options should not change.

It is wrong, because the search advances past opponent draft picks *after*
determinizing. Those picks vary per iteration, and what the searching player may
then do varies with them.

The evidence, not the argument, settled it: with the flag on, a tournament
panicked applying an illegal move within a few games; with it off, 300 games are
clean. A test that asserted invariance and passed had omitted the advance step,
so it confirmed the assumption instead of testing it.

Turning it off costs something — it is why the figures above are smaller than
the first draft's — but a faster search that returns illegal moves is not a
faster search.

### Whole games, and strength

8000 games at 4 000 iterations, `benchmarks/fixtures/variants-old-vs-new.json`,
run in eight chunks of 1 000:

| variant | games | wins | draws | losses | score | avg time | avg iterations |
|---|---:|---:|---:|---:|---:|---:|---:|
| legacy | 8000 | 4014 | 42 | 3944 | 50.44% | 1.4–1.6 s | 147 870 |
| crate | 8000 | 3944 | 42 | 4014 | **49.56%** | 1.1–1.4 s | 148 957 |

* Standard error 0.56 pp; 95% CI on the crate's score is
  **[48.47%, 50.66%]**, which spans 50%, so no strength difference is
  detectable.
* One-sided 95% lower bound is **48.64%**, above the 48% needed for
  non-inferiority at δ = 2 pp. **The gate passes.**
* Read precisely: this rules out the crate being more than ~1.4 pp weaker. It
  does not prove exact equality, and could not — the point estimate is 0.44 pp
  below even, comfortably inside noise.
* **Zero panics in 8000 games**, which is the more valuable half of this run.
  The illegal-move defects found during the port each showed up within a few
  hundred games, so this is a real absence rather than a small sample.
* The crate is ~15% faster per game while running slightly *more* iterations.
  That the iteration counts stay level is the check that early termination and
  subtree reuse still fire; a port that quietly lost either would need many more
  iterations per game.

Caveat on reproducibility: `tournament` seeds each worker from entropy
(`WyRand::from_rng(&mut rand::rng())`), so this exact run cannot be replayed.
The sample size is what carries it, not the seed.

For scale on why 8000 games and not 300: the identical-variant control in
`variants-migration.json` came out 55.8% / 44.2% over 300 games. Two
configurations differing in nothing, 11.6 pp apart. At 300 games the resolution
is roughly ±6 pp; here it is ±1.1 pp.

## Retiring the legacy search

With the strength gate passed, `ismcts.rs` no longer holds a search: 982 lines
down to 274, keeping only `MctsConfig` and the DUCT opponent draft model, which
is deliberately outside the tree and so outside the generic crate too. Every
caller — the runner, the GA, the GUI, and the browser build — now goes through
`ColoriSearcher`.

The wasm binary grew from **552 847 to 581 334 bytes, +5.2%**, against a gate of
20%. Before this change the wasm build still called the legacy search, so the
generic code was largely eliminated; this is the real cost of monomorphising it
for the browser, and it buys the same ~15% speedup the native build measured.

### The lookup-table hoist, and why it was dropped

The plan called for moving `card_lookup` and `sell_card_lookup` out of
`GameState`, on the grounds that 512 bytes per iteration is ~51 MB of memcpy
over a 100 000-iteration search. The arithmetic is right; the conclusion was
not. Measured (`--bench ismcts_bench determinize`):

| | median |
|---|---:|
| `determinize_in_place` | 114.3 ns |
| the state copy alone | 111.9 ns |

An iteration at that position takes ~8.8 µs, so **the entire state copy is 1.3%
of an iteration**, and the two tables are about 22% of it. Hoisting them would
save roughly **0.3% of a search** in exchange for rewriting ~150 call sites.
Dropped, and recorded here so it is not proposed again.

The same measurement dispatches a related assumption: `GameState` derives
`Clone`, so `clone_from` is the default `*self = source.clone()` and reuses no
allocations. It hardly matters — the state is bitsets, inline `FixedVec`s and
two byte arrays, so a clone is essentially a 2.3 KB memcpy with nothing to
reuse.

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
