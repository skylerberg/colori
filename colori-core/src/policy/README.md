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

- `colori-core/src/policy/features/` — the rule vocabulary, one table per file:
  `mod.rs` (the first table), `more.rs` (plans, completions, options, the clock)
  and `detail.rs` (one material, one tier of colour, one band of buyer value).
  Each rule carries its group and a one-line statement of which way a positive
  weight points.
- `colori-core/src/policy/value.rs` — rules that read a *position* rather than a
  choice, which is what a planned decision scores its leaves with.
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
   measure it. The vocabulary is meant to grow: a rule is one small, general
   reading of the board, and what the bot can notice is the union of them.

Two disciplines hold across the vocabulary:

- **Every rule is a delta.** A quantity that reads the same for every choice in
  a decision cancels out of the comparison; that kind of reading is a value rule.
- **Everything is in points.** A count of wanted colours and a colour's point
  value are different units, and mixing them is what made the first draft of the
  draft rules useless.
- **No rule may be dead.** A test walks reachable positions and fails on any rule
  that never returns anything but zero. A short, explicit `RARE` list names the
  rules that only fire in positions a random walk seldom reaches, so adding one
  is a decision somebody makes out loud.

Terms are evaluated in `FEATURES` order, never in a genome file's key order:
floating-point addition is not associative, so a genome that scored differently
after a re-save would not be deterministic. Ties go to the earliest choice in
enumeration order, which is itself deterministic.

## Two loops, with different jobs

Two things make the vocabulary non-linear:

- **Regimes.** A rule may carry its own weight for a stretch of the game
  (`early`, `late`), falling back to its base weight when it says nothing. This
  is how "keeping a card in round one is worth much more than keeping one in
  round six" gets expressed: the search found `ability_workshop` at 3.07 early
  against 5.61 late, which is the same statement.
- **Composed readings.** An interaction combines two operands with one operator
  (`multiply`, `min`, `max`, `add`), where an operand is any rule or a scalar the
  position supplies (`lateness`, `earliness`, `rounds_left`, `empty_slots`,
  `purchasable`, `affordable_buyers`). `multiply` scales one reading by another;
  `min`/`max` pick the binding constraint.

The dependence on the clock is written as compositions rather than as rules of
its own — `multiply(points_immediate, lateness)` and
`multiply(card_workshop_value, rounds_left)` replaced five hand-written time
rules at no cost in strength, which is the evidence that the mechanism can
substitute for hand-writing.

Structure is searched, not only weights: `policy-evolve --mutations N` mutates one
composed reading per attempt (its operator, an operand, or whether it exists) and
keeps the change only if a **paired** comparison beats its error bars — both
genomes played the same seeds, so a game that dealt well to one dealt well to the
other.

Three things had to be right before that search found anything at all, and the
first is the one that looked like the search being too weak:

1. **Fit a newcomer before judging it.** A composition introduced at a weight
   nobody chose is not that composition at its best. Each mutation now tries a
   grid of weights for the operand it changed, on a *separate* seed range so the
   verdict that follows is not a best-of-N dressed up as a single test.
2. **Lean towards accepting.** The two mistakes are not the same size: a
   worthless composition enters with a fitted weight near zero and the weight
   search drives it to zero, which costs compute; a good one that is rejected is
   gone, because nothing will propose it again. The bar is therefore one standard
   error, not two.
3. **Parallel universes.** The comparison is paired game by game.

With all three, a template with *no* composed readings ends a run holding eight,
discovered rather than written.

The third loop is a planned decision: over the *current turn* the policy can
follow its best few choices a couple of decisions deeper and score the position
that leaves, which is where sequences like "trash the dye for its Sell, then
choose the buyer" become visible. It is bounded, deterministic and cheap, and it
composes with the tuned one-ply score rather than replacing it, so bad value
weights cost nothing.

What a planned decision is worth depends almost entirely on the leaf, and the
first leaf had two faults.

It could not see an ability about to resolve, so a sale about to happen read as a
card already spent. Pricing that (a `value_pending_ability` term) took planning
from +0.05 to +0.30.

But pricing is not the same as fixing. A leaf that stops wherever the search ran
out of depth is not a position: it is a position *plus* however many actions
happen to be left, and two lines that stopped at different points are not
comparable. The tell was that deeper search kept paying — depth 1 < depth 2 <
depth 3, monotonically, which is what a biased leaf looks like. So the leaf now
**finishes the turn**: the rest of the turn is played out with the one-ply policy
and the position that is scored is where the turn ended, storage settled and
stack empty. The depth ladder flattened immediately (depth 1 = depth 2 = depth 3,
26.51 / 26.52 / 26.52), planning went to **+0.61**, and four value rules became
provably constant and were deleted.

The lesson generalises: when a term you added to fix a bias keeps growing in
importance as you add depth, the term is a proxy for the bias, not a fix.

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
whose other three seats are filled from a **frozen ladder** of genomes. That
gives a number that means the same thing in every generation and every A/B test.
Measuring a candidate against copies of itself, or against a pool the search can
also overfit, produces a bot that is champion of its own harness and mediocre
elsewhere.

Which rung sits where is decided by the game's index rather than sampled, so
every candidate measured on the same seed range faces exactly the same tables.
A random draw would leave a candidate's fitness at the mercy of whose turn it was
to face the strong rung.

The candidate's seat rotates over the games, so a favourable seat position
averages out instead of accumulating.

**Two numbers, one batch.** The report gives the candidate's **absolute score**
(what "thirty points" means) and its **delta against the rest of the table in
the same game**. The delta strips out the table's luck, because it is measured
within a game rather than across games, so it is the number a search should
compare. The absolute score stays the yardstick.

**Why not a round-robin instead.** Playing candidates against each other measures
more genomes per game — but a game costs about a millisecond, so throughput is
not the constraint; comparability is. Worse, these bots are deterministic, so a
population of them has no equilibrium to converge to: any exploit found against a
deterministic opponent is real, which is exactly what makes a fitness defined
relative to the current cohort cycle rather than climb. The ladder is what stops
that: it never moves, so the target does not.

The ladder is also where coevolution happens, deliberately: freeze the champion
into it when it beats every existing rung, and the class of opponents grows over
time while the expected fitness stays put. A `policy-arena` round-robin over the
champion and its predecessors is the right instrument for *checking* that
ranking, and for spotting cycles — not for fitness.

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
     --genome candidate-genome.json \
     --anchors policy-runs/best-genome.json \
     --games 2000 --seed-base 500000
   ```

5. Accept only if it clears the error bars. Then make the new genome the next
   run's `--template`.

`policy-play --trace-seed N` writes one game's full decision trace and prints
the ten decisions the rules **came closest to getting wrong**, with each side's
top contributing terms. That is the signal worth reasoning from — a decision
diff, not a score. If a rule is wrong, a narrow-margin decision will show it.

## Condensing a genome

`policy-prune` measures what a genome is holding up. Its honest use is the
*cumulative* curve rather than the per-gene table: cut the N smallest weights,
measure, and repeat. On a 171-gene genome at 26.71 self-play:

| cut | 20 | 40 | 60 | 80 | 90 | 100 | 120 |
|---|---|---|---|---|---|---|---|
| score | 26.65 | 26.53 | 26.51 | 26.40 | 25.33 | 24.25 | 18.45 |

Forty-seven per cent of the genome costs 0.31 points, the knee is at eighty, and
the cliff is between eighty and ninety. A random sixty is the control: it costs
2.39, so the ranking is doing real work. Retuning after the cut recovers most of
the rest — a 93-gene genome refitted lands at 26.58, within 0.12 of the parent.

The per-gene numbers are not to be trusted on their own, and knowing why matters
for anyone reading them: every single gene came out *negative*, meaning removing
it improved the table's score. The likeliest reading is that the tuned genome is
over-specialised for mirror play — four identical bots contend for the same
buyers and colours, so deviating from the optimum pays a little — while removing
many genes at once breaks the policy outright. Both effects are in every
measurement; the tier curve is where the second one shows.

## What is not here

- **Feature interactions** beyond `shape`: no `a · b` products. A rule that only
  matters in combination needs a feature that already encodes the context.
- **Draft denial**: a draft hand is the player's own; what an opponent is
  collecting is unmodelled. The `draft_redundancy` rule covers the related
  "don't draft a third of the same ability" question only.
- **Card quality** reads `HeuristicParams`, which default to
  `genetic-algorithm/batch-alobyo-gen-41.json` — the newest trained table, and
  the one baked into the binary so that a missing file cannot silently strip
  those rules of meaning. `--heuristic-params-file` overrides it.
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
