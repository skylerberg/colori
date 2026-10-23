# Colori

Colori is a strategic deck-building card game for 2-5 players about dyeing materials and selling to buyers. Complete game rules are in `RULES.md`.

## Three Peaks Hub

The physical game lives in the Three Peaks Hub project **Colori** (`threepeaks --project "Colori"`). The starter decks and the draft deck in `colori-core` mirror its **Main Deck**, and the buyers its **Sell Cards** deck (the printed name for the buyer cards): copy counts come from `threepeaks deck show`, and what a card does is read off its image. `public/cards/` holds those same card faces; the six character starter decks are under `public/cards/characters/<1-6>/`, numbered in deck order, and the buyers under `public/cards/sell/`.

The game screen is built on the printed boards (Three Peaks components in the **Boards** section). `public/boards/<1-6>.webp` are the player boards for characters 1-6 (components OG, Fun, Lady, Suave, Merchant, In Charge; "Sauve" is an outdated draft) and `public/boards/buyers.webp` is the Buyer's Board, each exported 2000px wide. The Buyer's Board is cropped to the right of its draft deck and trash, which only the physical game needs. `src/data/boardLayout.ts` records where every slot, counter and color-wheel region sits on that art, so re-measure it when the art's layout changes.

## Tech Stack

- Rust (game engine), Svelte 5 + TypeScript (frontend), Vite (build tool)
- trystero (P2P WebRTC multiplayer)

## Project Structure

Rust workspace plus a Svelte frontend:

- `colori-core/` — Core game engine library: game logic, phases, cards, colors, bots (MCTS)
- `colori-wasm/` — WASM bindings exposing Rust engine to the browser via wasm-bindgen (JSON serialization for state passing)
- `colori-runner/` — CLI tool for batch game simulation and benchmarking
- `colori-gui/` — Desktop GUI for game log analysis (egui/eframe)
- `src/` — Svelte 5 + TypeScript frontend
  - `src/components/` — UI components (game screen, draft/action views, card display, multiplayer screens)
  - `src/data/` — TypeScript type definitions and game data (mirrors Rust types)
  - `src/engine/` — WASM engine loader and game state management
  - `src/bots/` — Bot controller and Web Worker for running MCTS in background threads
  - `src/network/` — P2P multiplayer via trystero (host/guest controllers, state sanitization)
  - `src/wasm-pkg/` — Generated WASM output (gitignored)
- `game-logs/` — JSON game replay logs (gitignored)

## Build & Run Commands

- `pnpm install` — Install frontend dependencies
- `pnpm run dev` — Start Vite dev server
- `pnpm run build` — Production build
- `pnpm run build:wasm` — Rebuild WASM bindings (requires wasm-pack, outputs to `src/wasm-pkg/`)
- `pnpm run test` — Run frontend tests (Vitest)
- `pnpm run check` — Type-check the frontend (svelte-check + tsc)
- `pnpm run bench` — Run frontend benchmarks
- `cargo build` — Build all Rust crates
- `cargo test` — Run Rust tests

## Architecture Notes

- Game state flows through phases: Buyers → Draft → Action, then Game Over after round 7. There is no draw phase: setup deals the opening workshop and each player tops theirs up as their own turn ends, so a round opens inside whoever ended the last turn of the one before.
- All game logic lives in Rust (`colori-core`); other crates and the frontend consume it through bindings
- A bot searches determinized states with the `mcts` crate it depends on — opponents' hands are re-drawn every iteration, so a search never sees them — except on the game's last turn: once the last draft is over nothing is hidden or random, so `ColoriSearcher::search` hands that turn to `colori-core/src/final_turn.rs`, which solves it exactly for the highest ranking (points, then buyers, then colors). The pruning there takes some facts about the cards on trust; `the_cards_do_what_the_pruning_assumes` checks them, so a card change that breaks one fails that test. The frontend runs those searches in Web Workers.
- The action phase is simultaneous: each seat has its own `PlayerTurn` (ability stack and `ended` flag), and every choice is applied for a named player. The search tree holds only the searching player's action-phase moves; opponents' turns are played outside it by `colori-core/src/opponent_turns.rs`, so neither side reacts to the other. Bot drivers search an `ActionPhaseViews` view (the phase as it began plus that player's own moves), so no bot sees what others did that phase. Re-rooting a search tree is only sound for the same player's next decision in a turn; `TurnSearcher` (`colori-core/src/mcts_impl.rs`) makes that call for the runner and, as `BotSearcher`, for each seat's browser worker.
- Online multiplayer is peer-to-peer via trystero (WebRTC); opponent hands are sanitized before sending
- Cross-language FFI: Rust ↔ WASM (JSON) ↔ TypeScript
- Call the engine through `engineCall` (`src/engine/engineCall.ts`). A panic or failed allocation in wasm is otherwise a bare `RuntimeError: unreachable`; `colori-wasm/src/failure.rs` records the reason before the trap and `engineCall` puts it, with the call's input, in the error. Failures players should see go to `reportError` (`src/stores/errorReport.svelte.ts`), which shows a report they can copy or share.
- Card instance IDs (`cardInstanceId`) and buyer instance IDs (`buyerInstanceId`) use separate counters both starting from 0. In game logs, look up card IDs in `draftDeck` and player decks; look up buyer IDs in `buyerDeck` and `buyerDisplay`.
- Starter cards take the first card IDs, seven per seat in seat order. The frontend uses that to pick which character's portrait a starter card shows (`src/data/cardArt.ts`).
- `src/data/cards.ts` is generated by `colori-wasm/build.rs` from the Rust card data; change cards in `colori-core/src/types.rs` and `cards.rs`, then rebuild (`pnpm run build:wasm`) and commit the regenerated file.
- Scores can end in .5: bowls (wild primaries) and boats (wild materials) are worth half a point each unspent. `PlayerState::cached_whole_points` holds only the whole-point part (Completed Orders points plus gold coins); `scoring::score_in_half_points` is the full score.
- Use the board game's words. A card's *top actions* are what happens when it is used, and using one *trashes* the card (never "destroyed") into the *trash*; a player's drafted cards are their *top cards*, and a workshop card lifted up joins them. A *pigment* card stores colors, a *special* card its workshop ability. A buyer is worth *points*; *gold coins* are only the coins, each a point that can also be spent on a Workshop pick, a Mix or a sale. *Bowls* are the wild primaries and *boats* the wild materials, and the printed game calls a card's used state *used*, not rotated. Old names survive as serde aliases so saved games, logs and trained parameters still load.

## Testing

- **Rust**: Inline tests using `#[cfg(test)]` modules within source files. Run with `cargo test`.
- **TypeScript**: Vitest for tests (`.test.ts`) and benchmarks (`.bench.ts`). Run with `pnpm run test` or `pnpm run bench`.
- **Rust benchmarks**: Criterion-based, configured in `colori-core/Cargo.toml`.
- **Search fingerprint**: `cargo run --release -p colori-core --example search_fingerprint` prints the search's visits and rewards at fixed positions and seeds. To check a change that should leave search behaviour alone, diff its output against one taken from the code before the change.

## Code Conventions

- No explicit formatter/linter configs — use default `rustfmt` for Rust and default formatting for TypeScript
- Rust types use `#[derive(Debug, Clone, Serialize, Deserialize)]` extensively; add `Copy, PartialEq, Eq, Hash` where appropriate
- Serde JSON is the serialization format for game state across all boundaries (WASM, game logs)
- TypeScript uses discriminated unions for game phases (e.g., `{ type: 'draft'; draftState: ... }`)
- TypeScript interfaces for sanitized/public state use `Sanitized` prefix (e.g., `SanitizedGameState`)

## CI/CD

- GitHub Actions runs the Rust tests and the web build with its Vitest suite on pull requests (`.github/workflows/ci.yml`); the Rust job excludes `colori-gui`, so run `cargo test -p colori-gui` yourself after changing `colori-core`
- GitHub Actions deploys to S3 + CloudFront on push to `main` (`.github/workflows/deploy-to-s3.yml`)
- Steps: install deps → build WASM → build web → deploy to S3
