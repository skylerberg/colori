# AlphaZero Training for Colori

This document explains how to train a neural network to play Colori using
AlphaZero-style self-play, and how to use the trained model.

## Overview

The training pipeline replaces the random rollouts in MCTS with a neural
network that learns to evaluate positions and suggest moves through self-play.

```
Training cycle:
  Python (PyTorch) trains NN  →  exports ONNX model  →  Rust loads model
       ↑                                                       ↓
  training samples (numpy)  ←  self-play games via NN-MCTS  ←─┘
```

- Game simulation, MCTS, and NN inference during self-play all run in Rust
- Training runs in Python with PyTorch
- PyO3 bindings connect the two
- ONNX model files transfer weights from Python to Rust

## Prerequisites

- Rust toolchain (stable)
- Python 3.8+
- [maturin](https://www.maturin.rs/) (`pip install maturin`)

## Setup

### 1. Install Python dependencies

```bash
cd training
pip install -r requirements.txt
```

### 2. Build the Python extension module

```bash
cd colori-python
maturin develop --release
```

This compiles the Rust game engine and NN-MCTS code into a Python module
called `colori_python`. The `--release` flag is important for performance
since self-play is CPU-intensive.

## Training

### Run the training loop

```bash
cd training
python train.py
```

Each iteration:
1. Runs 200 self-play games using the current model (8 threads)
2. Stores training samples in a replay buffer
3. Trains the neural network for 10 epochs
4. Exports the updated model as ONNX

Output files are written to `training/models/`:
- `model_initial.onnx` — randomly initialized starting model
- `model_iterNNNN.onnx` — model after iteration NNNN
- `checkpoint_iterNNNN.pt` — PyTorch checkpoint (for resuming training)

### Configuration

Edit `training/config.py` to adjust hyperparameters:

| Parameter | Default | Description |
|---|---|---|
| `GAMES_PER_ITERATION` | 200 | Self-play games per iteration |
| `MCTS_ITERATIONS` | 200 | MCTS simulations per move |
| `C_PUCT` | 1.5 | PUCT exploration constant |
| `NUM_THREADS` | 8 | Parallel threads for self-play |
| `LEARNING_RATE` | 1e-3 | Adam optimizer learning rate |
| `BATCH_SIZE` | 256 | Training batch size |
| `EPOCHS_PER_ITERATION` | 10 | Training epochs per iteration |
| `NUM_ITERATIONS` | 100 | Total training iterations |
| `BUFFER_MAX_SIZE` | 500,000 | Max samples in replay buffer |
| `TEMP_THRESHOLD` | 30 | Move count where temperature drops from 1.0 to 0.1 |

### What to expect

- The first few iterations use a random model, so play quality is poor
- After ~10 iterations, the model should beat random play
- After ~50+ iterations, it should compete with baseline ISMCTS at low
  iteration counts
- Training time depends on hardware; each iteration takes minutes on a
  modern CPU

## Using a Trained Model

### CLI runner (head-to-head evaluation)

Compare NN-MCTS against baseline ISMCTS:

```bash
cargo run --release --manifest-path colori-runner/Cargo.toml --features nn -- \
  --games 100 \
  --model training/models/model_iter0100.onnx \
  --iterations 200 \
  --threads 8
```

The `--model` flag tells the runner to use NN-MCTS for all players.

To pit NN-MCTS against regular ISMCTS, use a variants file with a `model`
field on the variants that should use NN-MCTS. Variants without a `model`
field use regular ISMCTS:

```json
[
  {"name": "NN-200", "iterations": 200, "model": "training/models/model_iter0100.onnx"},
  {"name": "ISMCTS-1000", "iterations": 1000},
  {"name": "ISMCTS-100", "iterations": 100}
]
```

```bash
cargo run --release --manifest-path colori-runner/Cargo.toml --features nn -- \
  --games 100 \
  --variants-file variants.json \
  --threads 8
```

The `--model` CLI flag can also be used as a default for variants that don't
specify their own model. For example, `--model path/to/model.onnx` applies
NN-MCTS to any variant without an explicit `model` field in the variants file.

Note: the `nn` feature flag is required. Without it, `--model` will panic at
runtime.

### Browser (WASM)

The WASM build exposes NN-MCTS through a JavaScript callback pattern. The
frontend AI worker supports both regular ISMCTS and NN-MCTS.

To enable NN-MCTS in the browser:

```typescript
aiController.setNnMctsMode(true, 1.5);
```

The AI worker calls `wasm_run_nn_mcts` which invokes a JavaScript callback for
each neural network evaluation. The callback receives state and action
encodings as `Float32Array` values and must return `{ priors: Float32Array,
value: number }`.

The current implementation uses a uniform evaluator as a placeholder. To use a
real model, load it with ONNX Runtime Web and implement the evaluation
callback:

```typescript
import * as ort from 'onnxruntime-web';

const session = await ort.InferenceSession.create('model.onnx');

function nnEvaluate(stateEncoding, actionEncodings) {
  // Build ONNX Runtime tensors from stateEncoding and actionEncodings
  // Run session.run() with state, action_features, action_mask inputs
  // Return { priors: Float32Array, value: number }
}
```

### WASM encoding utilities

The WASM build also exposes encoding functions for custom integrations:

- `wasm_encode_state(gameStateJson, perspectivePlayer)` — returns 768
  floats
- `wasm_encode_legal_actions(gameStateJson)` — returns JSON array of
  86-float action encodings
- `wasm_get_legal_actions(gameStateJson)` — returns legal moves as JSON

## Architecture Details

### Neural network

- **State encoder**: Linear(768 → 256 → 256 → 128) with ReLU
- **Value head**: Linear(128 → 64 → 3) with softmax (win probability per
  player)
- **Action encoder**: Linear(86 → 64 → 128) with ReLU
- **Policy**: dot product between state embedding and each action embedding,
  softmax over legal actions

### State encoding (768 floats)

Per player (3 slots, rotated so the acting player is slot 0):
- Color wheel (12), materials (3), ducats (1), score (1)
- Completed buyers (54), workshop/drafted/used cards (42 each)
- Deck size (1), discard size (1)

Global features:
- Buyer display (54), round (1), phase (4), num players (3)
- Draft hand (42), draft pick/direction (2)
- Pending choice type (7), ability stack (9)

### Action encoding (86 floats)

- Choice type one-hot (14)
- Card type involved (42)
- Buyer category involved (18)
- Colors involved (12)

### Self-play data format

The `run_self_play_games` function returns a dict of numpy arrays:

| Key | Shape | Description |
|---|---|---|
| `states` | (N, 768) | State encodings |
| `action_features` | (N, max_actions, 86) | Action encodings, zero-padded |
| `action_masks` | (N, max_actions) | Boolean mask for valid actions |
| `policies` | (N, max_actions) | MCTS visit distribution |
| `values` | (N,) | Game outcome (1.0 = win, 0.0 = loss) |
