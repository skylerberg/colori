# Fixtures

Tracked inputs for reproducible measurement. The default `variants.json` is
gitignored, so anything used to justify a number has to live here instead.

* `variants-migration.json` — two identical variants at 4 000 iterations, both
  pinned to the same frozen heuristic parameters. Identical on purpose: running
  it should produce a win rate indistinguishable from 50%, which calibrates how
  much spread comes from the harness alone before any real comparison is read.
  It was also the basis of the old-vs-new A/B during the migration; that
  comparison is finished (8 000 games, recorded in `../BASELINE.md`) and the
  legacy search is gone, so only the calibration use remains.

* Heuristic parameters are pinned to `genetic-algorithm/batch-lki08w-gen-32.json`,
  the generation the tests already use. It is tracked and immutable, so it is
  referenced directly rather than copied here — an ongoing GA run cannot move it
  underneath a measurement.

4 000 iterations is deliberate: it is what the GA already uses for evaluation,
and a powered comparison needs thousands of games, which is not affordable at
25 000. Confirm at production iteration counts with a smaller sample afterwards.
