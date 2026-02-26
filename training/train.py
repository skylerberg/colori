"""Main training loop for Colori AlphaZero."""

import os
import sys
import time
import numpy as np
import torch
import torch.nn.functional as F
from pathlib import Path

from config import (
    STATE_ENCODING_SIZE, ACTION_ENCODING_SIZE, NUM_PLAYERS,
    LEARNING_RATE, BATCH_SIZE, EPOCHS_PER_ITERATION, NUM_ITERATIONS,
    WEIGHT_DECAY, GAMES_PER_ITERATION, MCTS_ITERATIONS, C_PUCT,
    NUM_THREADS, BUFFER_MIN_SIZE, MODEL_DIR, INITIAL_MODEL,
)
from model import ColoriNet, export_onnx, create_initial_model
from replay_buffer import ReplayBuffer, pad_to_max_actions


def train_epoch(model, optimizer, replay_buffer, batch_size):
    """Train for one epoch on sampled data from replay buffer.

    Returns dict with loss components.
    """
    model.train()

    batch = replay_buffer.sample(batch_size)

    states = torch.tensor(batch['states'], dtype=torch.float32)
    action_features = torch.tensor(batch['action_features'], dtype=torch.float32)
    action_masks = torch.tensor(batch['action_masks'], dtype=torch.bool)
    target_policies = torch.tensor(batch['policies'], dtype=torch.float32)
    target_values = torch.tensor(batch['values'], dtype=torch.float32)

    # Forward pass
    policy_logits, value_pred = model(states, action_features, action_masks)

    # Policy loss: cross-entropy between MCTS visit distribution and NN policy
    # Use log_softmax on logits, then nll_loss with soft targets
    log_policy = F.log_softmax(policy_logits, dim=-1)
    # Mask out invalid actions in target too
    target_policies = target_policies * action_masks.float()
    # Renormalize target
    target_sum = target_policies.sum(dim=-1, keepdim=True).clamp(min=1e-8)
    target_policies = target_policies / target_sum
    policy_loss = -(target_policies * log_policy).sum(dim=-1).mean()

    # Value loss: MSE between predicted value[0] (perspective player's win prob) and actual outcome
    # target_values is (batch,) float - 1.0 for winner, 0.0 for loser
    # value_pred is (batch, 3) softmax - we use index 0 (perspective player)
    value_loss = F.mse_loss(value_pred[:, 0], target_values)

    # Total loss
    loss = policy_loss + value_loss

    optimizer.zero_grad()
    loss.backward()
    optimizer.step()

    return {
        'total': loss.item(),
        'policy': policy_loss.item(),
        'value': value_loss.item(),
    }


def run_training():
    """Main training loop."""
    os.makedirs(MODEL_DIR, exist_ok=True)

    # Try to import the Rust self-play module
    try:
        import colori_python
        has_rust = True
    except ImportError:
        print("WARNING: colori_python not found. Build it with: cd colori-python && maturin develop --release")
        print("Proceeding without self-play (for testing training loop only).")
        has_rust = False

    # Initialize model
    model = ColoriNet()
    optimizer = torch.optim.Adam(model.parameters(), lr=LEARNING_RATE, weight_decay=WEIGHT_DECAY)
    replay_buffer = ReplayBuffer()

    # Export initial random model
    initial_path = INITIAL_MODEL
    export_onnx(model, initial_path)
    current_model_path = initial_path

    for iteration in range(NUM_ITERATIONS):
        iter_start = time.time()
        print(f"\n{'='*60}")
        print(f"Iteration {iteration + 1}/{NUM_ITERATIONS}")
        print(f"{'='*60}")

        # Self-play phase
        if has_rust:
            print(f"Running {GAMES_PER_ITERATION} self-play games...")
            sp_start = time.time()
            samples = colori_python.run_self_play_games(
                num_games=GAMES_PER_ITERATION,
                model_path=current_model_path,
                mcts_iterations=MCTS_ITERATIONS,
                c_puct=C_PUCT,
                num_threads=NUM_THREADS,
            )
            sp_time = time.time() - sp_start

            n_samples = len(samples['states'])
            print(f"Generated {n_samples} training samples in {sp_time:.1f}s")

            replay_buffer.add_batch(
                samples['states'],
                samples['action_features'],
                samples['action_masks'],
                samples['policies'],
                samples['values'],
            )
        else:
            print("Skipping self-play (no colori_python module)")
            if len(replay_buffer) == 0:
                print("No data in replay buffer. Generating dummy data for testing...")
                # Generate dummy data for testing the training loop
                n_dummy = BUFFER_MIN_SIZE
                max_actions = 50
                dummy_states = np.random.randn(n_dummy, STATE_ENCODING_SIZE).astype(np.float32)
                dummy_actions = np.random.randn(n_dummy, max_actions, ACTION_ENCODING_SIZE).astype(np.float32)
                dummy_masks = np.ones((n_dummy, max_actions), dtype=bool)
                dummy_masks[:, 30:] = False  # Mask out some actions
                dummy_policies = np.random.dirichlet(np.ones(max_actions), size=n_dummy).astype(np.float32)
                dummy_policies *= dummy_masks
                dummy_policies /= dummy_policies.sum(axis=-1, keepdims=True)
                dummy_values = np.random.choice([0.0, 1.0], size=n_dummy).astype(np.float32)
                replay_buffer.add_batch(
                    dummy_states, dummy_actions, dummy_masks, dummy_policies, dummy_values
                )

        # Training phase
        if len(replay_buffer) < BUFFER_MIN_SIZE:
            print(f"Not enough samples ({len(replay_buffer)}/{BUFFER_MIN_SIZE}). Skipping training.")
            continue

        print(f"Training for {EPOCHS_PER_ITERATION} epochs (buffer size: {len(replay_buffer)})...")
        train_start = time.time()

        for epoch in range(EPOCHS_PER_ITERATION):
            losses = train_epoch(model, optimizer, replay_buffer, BATCH_SIZE)
            if epoch % 5 == 0 or epoch == EPOCHS_PER_ITERATION - 1:
                print(f"  Epoch {epoch+1}/{EPOCHS_PER_ITERATION}: "
                      f"loss={losses['total']:.4f} "
                      f"(policy={losses['policy']:.4f}, value={losses['value']:.4f})")

        train_time = time.time() - train_start

        # Export updated model
        new_model_path = os.path.join(MODEL_DIR, f"model_iter{iteration+1:04d}.onnx")
        export_onnx(model, new_model_path)
        current_model_path = new_model_path

        # Save checkpoint
        checkpoint_path = os.path.join(MODEL_DIR, f"checkpoint_iter{iteration+1:04d}.pt")
        torch.save({
            'iteration': iteration + 1,
            'model_state_dict': model.state_dict(),
            'optimizer_state_dict': optimizer.state_dict(),
            'buffer_size': len(replay_buffer),
        }, checkpoint_path)

        iter_time = time.time() - iter_start
        print(f"Iteration complete in {iter_time:.1f}s (train: {train_time:.1f}s)")

    print(f"\nTraining complete! Final model: {current_model_path}")


if __name__ == '__main__':
    run_training()
