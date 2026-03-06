# Colori

A strategic deck-building card game for 2-5 players about dyeing materials and selling to buyers. Complete game rules are in `RULES.md`.

## Quick Start

```bash
npm install
npm run build:wasm
npm run dev
```

## AlphaZero Training Pipeline

The `training/` directory contains an AlphaZero-style training pipeline that learns to play Colori through self-play. It uses a neural network (policy + value heads) guided by MCTS.

### Prerequisites

The training pipeline requires the `colori-python` PyO3 crate, which exposes the Rust game engine to Python.

```bash
# Create a virtual environment
python -m venv training/venv
source training/venv/bin/activate

# Install Python dependencies
pip install -r training/requirements.txt

# Build and install the colori-python module
pip install maturin
cd colori-python && maturin develop --release && cd ..
```

### Training

Each iteration generates self-play games using the current model, then trains on the collected data. Checkpoints are saved after every iteration.

```bash
cd training

# Run training with defaults (100 iterations, 100 games/iter, 1000 sims/move)
python run_training.py

# Customize training parameters
python run_training.py --iterations 50 --games 200 --simulations 800 --epochs 10

# Resume from a checkpoint
python run_training.py --resume checkpoints/checkpoint_0010.pt
```

Checkpoints are saved to `training/checkpoints/` as `checkpoint_NNNN.pt`. A final `model.onnx` is exported automatically when training completes.

### Exporting Checkpoints to ONNX

To benchmark a specific training iteration, export its checkpoint to ONNX:

```bash
python training/export_checkpoint.py training/checkpoints/checkpoint_0010.pt -o model_iter10.onnx
```

If `-o` is omitted, the output path defaults to the checkpoint path with an `.onnx` extension.

### Benchmarking with colori-runner

The runner supports NN-MCTS variants alongside traditional ISMCTS variants via `variants.json`. Build with the `nn-ai` feature to enable NN-MCTS support:

```bash
cargo build --release -p colori-runner --features nn-ai
```

Create a `variants.json` file:

```json
[
  {
    "name": "nn iter 10",
    "algorithm": "nn-mcts",
    "modelPath": "model_iter10.onnx",
    "simulations": 200,
    "cPuct": 1.5
  },
  {
    "name": "ismcts 50k",
    "iterations": 50000
  }
]
```

Run games:

```bash
cargo run --release -p colori-runner --features nn-ai -- \
  --games 100 --threads 4 --variants-file variants.json
```

### Variant Configuration Reference

**ISMCTS variants** (default):

| Field                | Type   | Default    | Description                       |
|----------------------|--------|------------|-----------------------------------|
| `name`               | string | auto       | Display name for this variant     |
| `iterations`         | number | 100        | MCTS iterations per move          |
| `explorationConstant`| number | sqrt(2)    | UCB exploration constant          |
| `maxRolloutSteps`    | number | 200        | Max steps per rollout simulation  |

**NN-MCTS variants** (requires `nn-ai` feature):

| Field         | Type   | Default | Description                                    |
|---------------|--------|---------|------------------------------------------------|
| `name`        | string | auto    | Display name for this variant                  |
| `algorithm`   | string | —       | Must be `"nn-mcts"`                            |
| `modelPath`   | string | —       | Path to ONNX model file (required)             |
| `simulations` | number | 200     | MCTS simulations per move                      |
| `cPuct`       | number | 1.5     | PUCT exploration constant (higher = more exploration) |
