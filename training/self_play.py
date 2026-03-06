import numpy as np
import colori_python as cp
from config import NUM_ACTIONS, NUM_PLAYERS, TEMPERATURE_THRESHOLD
from mcts import MCTS


def play_game(model, mcts_handler, seed, temperature=1.0):
    """Play one self-play game, returning training samples.

    Returns list of (observation, legal_mask, policy_target, value_target) tuples.
    """
    game = cp.PyGameState(NUM_PLAYERS, seed)
    game.advance_draw_phase()

    trajectory = []  # (obs, legal_mask, policy, player_index)
    step = 0

    while not game.is_terminal():
        current_player = game.get_current_player()
        obs = game.get_observation(current_player).copy()
        legal_mask = game.get_legal_mask().copy()

        # Run MCTS
        visit_distribution = mcts_handler.search(game, current_player, add_noise=True)

        # Store training sample
        trajectory.append((obs, legal_mask, visit_distribution.copy(), current_player))

        # Select action with temperature
        if step < TEMPERATURE_THRESHOLD:
            # Sample proportionally to visit counts
            probs = visit_distribution ** (1.0 / temperature)
            probs = probs / probs.sum()
            action = np.random.choice(NUM_ACTIONS, p=probs)
        else:
            # Greedy
            action = np.argmax(visit_distribution)

        game.apply_action(action)
        step += 1

    # Get terminal rewards
    rewards = game.get_rewards()

    # Convert to training samples with value targets
    samples = []
    for obs, legal_mask, policy, player_idx in trajectory:
        # Map reward [0, 1] to value [-1, 1]
        value = rewards[player_idx] * 2 - 1
        samples.append((obs, legal_mask, policy, value))

    return samples


def generate_games(model, device, num_games, num_simulations):
    """Generate multiple self-play games and return all training samples."""
    mcts_handler = MCTS(model, num_simulations, device)
    all_samples = []

    for i in range(num_games):
        seed = np.random.randint(0, 2**32)
        samples = play_game(model, mcts_handler, seed)
        all_samples.extend(samples)

    return all_samples
