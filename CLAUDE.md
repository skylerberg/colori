# Colori

Colori is a strategic deck-building card game for 2-5 players about dyeing materials and selling to buyers. Complete game rules are in `RULES.md`.

## Tech Stack

- Rust (game engine), Svelte 5 + TypeScript (frontend), Vite (build tool)
- trystero (P2P WebRTC multiplayer), PyTorch + PyO3/maturin (training), ONNX Runtime (NN inference)

## Project Structure

Rust workspace with 4 crates plus a Svelte frontend and Python training pipeline:

- `colori-core/` — Core game engine library: game logic, phases, cards, colors, AI (MCTS + NN-MCTS), state encoding
- `colori-wasm/` — WASM bindings exposing Rust engine to the browser via wasm-bindgen (JSON serialization for state passing)
- `colori-runner/` — CLI tool for batch game simulation and benchmarking
- `colori-python/` — PyO3 Python bindings for the AlphaZero training pipeline
- `src/` — Svelte 5 + TypeScript frontend
  - `src/components/` — UI components (game screen, draft/action views, card display, multiplayer screens)
  - `src/data/` — TypeScript type definitions and game data (mirrors Rust types)
  - `src/engine/` — WASM engine loader and game state management
  - `src/ai/` — AI controller and Web Worker for running MCTS in background threads
  - `src/network/` — P2P multiplayer via trystero (host/guest controllers, state sanitization)
  - `src/analysis/` — Game log analysis dashboard
  - `src/wasm-pkg/` — Generated WASM output (gitignored)
- `training/` — AlphaZero-style neural network training (Python/PyTorch)
  - `train.py` — Main training loop with checkpoint resume
  - `model.py` — ColoriNet neural network (768-dim state → policy + value)
  - `replay_buffer.py` — Experience replay buffer
  - `config.py` — Training hyperparameters
- `game-logs/` — JSON game replay logs (gitignored)

## Build & Run Commands

- `npm install` — Install frontend dependencies
- `npm run dev` — Start Vite dev server
- `npm run build` — Production build
- `npm run build:wasm` — Rebuild WASM bindings (requires wasm-pack, outputs to `src/wasm-pkg/`)
- `npm run test` — Run frontend tests (Vitest)
- `npm run bench` — Run frontend benchmarks
- `cargo build --release` — Build all Rust crates
- `cargo test` — Run Rust tests
- `npm run run-games -- --games N --iterations I --threads T` — Batch game simulation via colori-runner
- Training: `cd colori-python && maturin develop --release`, then `cd training && python train.py`

## Architecture Notes

- Game state flows through phases: Draw → Draft → Action → Cleanup (repeats up to 10 rounds or until a player reaches 15+ points)
- All game logic lives in Rust (`colori-core`); other crates and the frontend consume it through bindings
- AI uses Information Set MCTS (for imperfect information); NN-MCTS adds AlphaZero-style neural network policy/value guidance
- Frontend runs AI in Web Workers; draft picks are precomputed for responsiveness
- Online multiplayer is peer-to-peer via trystero (WebRTC); opponent hands are sanitized before sending
- State encoding for NN: 768-dim state vector, 86-dim action features
- Cross-language FFI: Rust ↔ WASM (JSON) ↔ TypeScript, Rust ↔ Python (NumPy/PyO3)

## Testing

- **Rust**: Inline tests using `#[cfg(test)]` modules within source files. Run with `cargo test`.
- **TypeScript**: Vitest for tests (`.test.ts`) and benchmarks (`.bench.ts`). Currently only benchmarks exist. Run with `npm run test` or `npm run bench`.
- **Rust benchmarks**: Criterion-based, configured in `colori-core/Cargo.toml`.

## Code Conventions

- No explicit formatter/linter configs — use default `rustfmt` for Rust and default formatting for TypeScript
- Rust types use `#[derive(Debug, Clone, Serialize, Deserialize)]` extensively; add `Copy, PartialEq, Eq, Hash` where appropriate
- Serde JSON is the serialization format for game state across all boundaries (WASM, Python, game logs)
- TypeScript uses discriminated unions for game phases (e.g., `{ type: 'draft'; draftState: ... }`)
- TypeScript interfaces for sanitized/public state use `Sanitized` prefix (e.g., `SanitizedGameState`)

## CI/CD

- GitHub Actions deploys to S3 + CloudFront on push to `main` (`.github/workflows/deploy-to-s3.yml`)
- Steps: install deps → build WASM → build web → deploy to S3
