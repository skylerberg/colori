import colori_python as cp

# Game constants (from Rust engine)
NUM_ACTIONS = cp.NUM_ACTIONS  # 300
OBS_SIZE = cp.OBS_SIZE  # 596

# Network architecture
HIDDEN_SIZE = 256
NUM_RES_BLOCKS = 8

# MCTS
NUM_SIMULATIONS = 200
C_PUCT = 1.5
TEMPERATURE_THRESHOLD = 30  # steps after which temperature drops to near-zero
DIRICHLET_ALPHA = 0.3
DIRICHLET_EPSILON = 0.25

# Training
NUM_PLAYERS = 2
BATCH_SIZE = 256
LEARNING_RATE = 0.001
WEIGHT_DECAY = 1e-4
NUM_EPOCHS = 10
REPLAY_BUFFER_SIZE = 100_000

# Self-play
GAMES_PER_ITERATION = 100
NUM_ITERATIONS = 100
CHECKPOINT_DIR = "checkpoints"
