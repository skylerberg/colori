# Deterministic players

A rules-based Colori bot: deterministic, cheap enough to run in the tens of
thousands, and structured so that an LLM can improve the *rule system* while a
search tunes the numbers.

The starting point was an ISMCTS searcher whose rollout heuristics are tuned by
a genetic algorithm. That works, but it is slow and opaque: most of the compute
goes into the search, so the thing actually being tuned — the policy — gets very
few samples, and a 48-float genome says nothing about why a bot plays badly.

## The model

A decision offers a set of legal `Choice`s. The policy scores each and takes the
highest. No search, no sampling, no randomness: a game is reproducible from its
seed.

A choice's score is a weighted sum of **features**, each of which answers one
question about the choice **in points**. One currency is what lets the policy
compare a trash against a ducat spend against ending the turn. Without it,
per-choice-type constants drift apart and the rules stop being comparable as
soon as they interact — which is exactly where hand-written rules rot.

```
score(choice) = Σ  weight_f · shape_f( feature_f(state, choice) )
```

- `colori-core/src/policy/features.rs` — every feature, with its group and a
  one-line statement of which way a positive weight points.
- `colori-core/src/policy/effects.rs` — what a choice *moves* (colours,
  materials, boats, ducats, points, ability grants), read off the choice's
  payload. A mix sequence is replayed against a copy of the colour wheel and
  diffed, because a mix both spends inputs and produces an output.
- `colori-core/src/policy/context.rs` — the per-decision aggregates every
  feature shares: what the sitting buyers still want, and what a unit of each
  colour or material is worth in points.
- `colori-core/src/policy/mod.rs` — genomes, shapes, the compiled policy, and
  decision traces.

Writing the policy as a weighted sum rather than as nested conditionals buys
three things:

1. A genome is a flat table of numbers, so a tuner can search it without
   understanding it.
2. A decision's score is the sum of its parts, so a **trace is also an
   explanation**: the largest contributions say what the policy was thinking.
3. A new rule is a new `FeatureDef` and nothing else. It cannot change how
   existing rules behave, so it is safe to propose one from outside the loop and
   measure it.

Terms are evaluated in `FEATURES` order, never in a genome file's key order:
floating-point addition is not associative, so a genome that scored differently
after a re-save would not be deterministic. Ties go to the earliest choice in
enumeration order, which is itself deterministic.

## Two loops, with different jobs

| Loop | Question | Method | Cost |
| --- | --- | --- | --- |
| Inner | how much should this rule matter? | cross-entropy method over weights | millions of games |
| Outer | what rules should exist? | a human (or an LLM) proposes, the evaluator decides | tens of games of thinking |

A genetic algorithm is the wrong tool for the inner loop. Fifty continuous
parameters is not a genetic algorithm's problem: crossover on weights mixes
unrelated magnitudes and destroys the fine structure the search found, and
mutation alone is a slow random walk. The cross-entropy method — sample a
Gaussian, keep the elite, refit the Gaussian to them — needs a fraction of the
games, and its state (a mean and a spread per gene) is readable: a collapsed
spread means a gene is settled, a wide one means it is still being argued about.

The outer loop is the wrong place for a search. Rule composition is a
variable-length, structurally coupled space where most variants never fire, so a
search over it sees a plateau. That is where reasoning is worth spending, and
where the LLM earns its place — under one constraint, below.

## Measurement

This is the part that decides whether any of it works.

**The yardstick.** A candidate is measured on one seat of a four-player table
whose other three seats are a **frozen anchor** genome. That gives a number that
means the same thing in every generation and every A/B test. Measuring a
candidate against copies of itself, or against a pool the search can also
overfit, produces a bot that is champion of its own harness and mediocre
elsewhere.

The candidate's seat rotates over the games, so a favourable seat position
averages out instead of accumulating.

**The score, not the win rate.** Thirty is a good score, thirty-five a great
one, forty is above a human. The report gives the mean with its standard error,
the median, p10/p25/p75/p90, and the share of games at or above 30, 35 and 40.

**Noise.** Game `i` uses seed `seed_base + i`, and a genome consumes no
randomness, so every genome measured on the same range sees the same opening
hands. Streams do diverge later — how many cards a draw pulls depends on
choices made earlier — so this is variance reduction rather than a truly paired
comparison, but it removes the largest single source of noise.

**A proposal is only accepted if it clears its own error bars.** Two means that
differ by less than about `2 · sqrt(se_a² + se_b²)` are one number, not two.

**A holdout.** `--holdout-games` measures the best genome each generation on a
seed range that is never tuned against. A genome gaining on the training seeds
while losing on the holdout is overfitting the seeds, and the report says so
before a run wastes another hour.

**Inert rules are reported.** Per-feature mean absolute contribution and how
often it fired. A rule that never fires is either switched off, inert on this
position, or shadowed by another; a search cannot tune a gene that never fires,
and the answer is usually to delete it.

## The protocol for improving the rules

The evaluator is the judge; the LLM is a mutation operator. Not the other way
round: reading twenty games and rewriting the rule system from an anecdote is
how a confident, wrong rule gets adopted.

1. Run `policy-evolve` for a batch of generations. It writes one
   `generation-NNN.json` per generation plus `best-genome.json`.
2. Read the report, not the games. In order:
   - `meanGenome.summary` against `baseline`: is the mean still improving?
   - `holdout` against the training mean: is the gain real?
   - `activity`: which rules never fire? Delete them.
   - `weights[].sigma`: which rules has the search settled? Which is still
     arguing?
3. Form **one** hypothesis and make **one** change. Three kinds are available:
   - *prune*: set a weight to 0 in a new template genome (no code);
   - *reshape*: change a rule's `shape` (`linear`/`sqrt`/`square`/`log1p`/`unit`)
     to model diminishing or amplifying returns (no code);
   - *add a rule*: a new `FeatureDef` in `features.rs` (code, plus a test, plus a
     starter weight).
4. A/B it against the incumbent on enough games to resolve the difference:

   ```sh
   cargo run --release -p colori-runner -- policy-play \
     --genome policy-runs/best-genome.json \
     --challenger candidate-genome.json \
     --games 2000 --seed-base 500000
   ```

5. Accept only if it clears the error bars. Then make the new genome the next
   run's `--template`.

`policy-play --trace-seed N` writes one game's full decision trace and prints
the ten decisions the rules **came closest to getting wrong**, with each side's
top contributing terms. That is the signal worth reasoning from — a decision
diff, not a score. If a rule is wrong, a narrow-margin decision will show it.

## What is not here

- **Feature interactions** beyond `shape`: no `a · b` products. A rule that only
  matters in combination needs a feature that already encodes the context.
- **Draft denial**: a draft hand is the player's own; what an opponent is
  collecting is unmodelled. The `draft_redundancy` rule covers the related
  "don't draft a third of the same ability" question only.
- **Card quality** reads `HeuristicParams`. Without
  `--heuristic-params-file` the neutral table gives every card the same quality,
  so those rules are inert — visible in the activity report, but worth knowing
  before reading anything into them.
- **Search of any kind.** The next gains here are probably not better rules but
  a cheap exact solve where branching is tiny (mix sequences, workshop subsets,
  sell selection given a hand). That stays deterministic and fast.
- **Self-play imitation.** A stronger weight update than CEM is available once
  the policy is decent: play a batch, then reweight decisions towards those the
  high-scoring seat took, and fit the weights to that. Fewer games, better
  credit assignment.

## The upgrade path

A rules policy is also a value function. Once it is good, it can (a) be the
rollout policy inside the existing ISMCTS — cheaper and better than the current
hand-written rollout, (b) give a leaf estimate without a rollout, and (c) play
opponent turns in `opponent_turns.rs` more plausibly than a rollout does. None
of that requires the policy to be the final bot; it is useful at every strength.
