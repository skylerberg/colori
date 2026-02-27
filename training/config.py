"""Hyperparameters for AlphaZero-style training."""

# State/action encoding dimensions (must match Rust)
STATE_ENCODING_SIZE = 768
ACTION_ENCODING_SIZE = 86

# Network architecture
STATE_HIDDEN_DIM = 256
STATE_EMBED_DIM = 128
ACTION_HIDDEN_DIM = 64
NUM_PLAYERS = 3  # Fixed 3-player games for training

# Training
LEARNING_RATE = 1e-3
BATCH_SIZE = 256
EPOCHS_PER_ITERATION = 10
NUM_ITERATIONS = 100
WEIGHT_DECAY = 1e-4

# Self-play
GAMES_PER_ITERATION = 200
MCTS_ITERATIONS = 200
C_PUCT = 1.5
NUM_THREADS = 8

# Temperature schedule
TEMP_HIGH = 1.0       # Temperature for first N moves
TEMP_LOW = 0.1        # Temperature after N moves
TEMP_THRESHOLD = 30   # Switch temperature after this many moves

# Replay buffer
BUFFER_MAX_SIZE = 500_000  # Max training samples in buffer
BUFFER_MIN_SIZE = 1_000    # Min samples before training starts

# Model paths
MODEL_DIR = "models"
INITIAL_MODEL = "models/model_initial.onnx"
