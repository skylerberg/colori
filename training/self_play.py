import numpy as np
import colori_python as cp
from config import NUM_ACTIONS, NUM_PLAYERS, TEMPERATURE_THRESHOLD, CONCURRENT_GAMES
from mcts import BatchedMCTS


def generate_games(model, device, num_games, num_simulations):
    """Generate self-play games with batched GPU inference.

    Runs CONCURRENT_GAMES games simultaneously so leaf evaluations can be
    batched into single GPU forward passes, making GPU inference efficient.
    """
    import torch
    from tqdm import tqdm

    model.eval()
    batched_mcts = BatchedMCTS(model, num_simulations, device)
    all_samples = []
    games_completed = 0

    with tqdm(total=num_games, desc="Self-play", unit="game") as pbar:
        while games_completed < num_games:
            batch_size = min(CONCURRENT_GAMES, num_games - games_completed)
            batch_samples = _play_game_batch(batched_mcts, batch_size)
            all_samples.extend(batch_samples)
            games_completed += batch_size
            pbar.update(batch_size)

    return all_samples


def _play_game_batch(batched_mcts, batch_size, temperature=1.0):
    """Play a batch of games simultaneously, using batched MCTS.

    Returns all training samples from all games in the batch.
    """
    seeds = [int(np.random.randint(0, 2**32)) for _ in range(batch_size)]
    games = [cp.PyGameState(NUM_PLAYERS, s) for s in seeds]
    for g in games:
        g.advance_draw_phase()

    # Per-game state
    trajectories = [[] for _ in range(batch_size)]  # (obs, legal_mask, policy, player_idx)
    steps = [0] * batch_size
    active = list(range(batch_size))  # indices of games still in progress

    while active:
        # Gather current players for active games
        active_games = [games[i] for i in active]
        active_players = [games[i].get_current_player() for i in active]

        # Batched MCTS search
        visit_dists = batched_mcts.search_batch(active_games, active_players, add_noise=True)

        # Process results for each active game
        newly_finished = []
        for idx_in_active, game_idx in enumerate(active):
            game = games[game_idx]
            player = active_players[idx_in_active]
            visit_dist = visit_dists[idx_in_active]
            step = steps[game_idx]

            # Record training sample
            obs = game.get_observation(player).copy()
            legal_mask = game.get_legal_mask().copy()
            trajectories[game_idx].append((obs, legal_mask, visit_dist.copy(), player))

            # Select action
            if step < TEMPERATURE_THRESHOLD:
                probs = visit_dist ** (1.0 / temperature)
                total = probs.sum()
                if total > 0:
                    probs = probs / total
                    action = np.random.choice(NUM_ACTIONS, p=probs)
                else:
                    action = np.random.choice(game.get_legal_actions())
            else:
                action = int(np.argmax(visit_dist))

            game.apply_action(action)
            steps[game_idx] = step + 1

            # Advance draw phase if needed
            if game.is_draw_phase():
                game.advance_draw_phase()

            if game.is_terminal():
                newly_finished.append(game_idx)

        # Remove finished games from active list
        for gi in newly_finished:
            active.remove(gi)

    # Convert trajectories to training samples
    all_samples = []
    for game_idx in range(batch_size):
        rewards = games[game_idx].get_rewards()
        for obs, legal_mask, policy, player_idx in trajectories[game_idx]:
            value = rewards[player_idx] * 2 - 1
            all_samples.append((obs, legal_mask, policy, value))

    return all_samples
