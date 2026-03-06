#!/usr/bin/env python3
"""AlphaZero-style training for Colori."""

import os
import sys
import argparse
import torch
from tqdm import tqdm

from config import (
    NUM_ITERATIONS,
    GAMES_PER_ITERATION,
    NUM_SIMULATIONS,
    NUM_EPOCHS,
    CHECKPOINT_DIR,
)
from model import ColoriNet
from replay_buffer import ReplayBuffer
from self_play import generate_games
from train import train_epoch, create_optimizer


def main():
    parser = argparse.ArgumentParser(description="Train AlphaZero for Colori")
    parser.add_argument("--iterations", type=int, default=NUM_ITERATIONS)
    parser.add_argument("--games", type=int, default=GAMES_PER_ITERATION)
    parser.add_argument("--simulations", type=int, default=NUM_SIMULATIONS)
    parser.add_argument("--epochs", type=int, default=NUM_EPOCHS)
    parser.add_argument("--checkpoint-dir", type=str, default=CHECKPOINT_DIR)
    parser.add_argument("--resume", type=str, default=None, help="Path to checkpoint to resume from")
    args = parser.parse_args()

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    print(f"Using device: {device}")

    model = ColoriNet().to(device)
    optimizer = create_optimizer(model)
    replay_buffer = ReplayBuffer()
    start_iteration = 0

    if args.resume:
        checkpoint = torch.load(args.resume, map_location=device, weights_only=False)
        model.load_state_dict(checkpoint["model"])
        optimizer.load_state_dict(checkpoint["optimizer"])
        start_iteration = checkpoint.get("iteration", 0) + 1
        print(f"Resumed from iteration {start_iteration - 1}")

    os.makedirs(args.checkpoint_dir, exist_ok=True)

    for iteration in range(start_iteration, args.iterations):
        print(f"\n=== Iteration {iteration} ===")

        # Self-play
        print(f"Generating {args.games} self-play games ({args.simulations} sims/move)...")
        samples = generate_games(model, device, args.games, args.simulations)
        replay_buffer.add_batch(samples)
        print(f"  Generated {len(samples)} samples, buffer size: {len(replay_buffer)}")

        # Training
        print(f"Training for {args.epochs} epochs...")
        for epoch in range(args.epochs):
            policy_loss, value_loss = train_epoch(model, optimizer, replay_buffer, device)
            if epoch == 0 or epoch == args.epochs - 1:
                print(f"  Epoch {epoch}: policy_loss={policy_loss:.4f}, value_loss={value_loss:.4f}")

        # Save checkpoint
        checkpoint_path = os.path.join(args.checkpoint_dir, f"checkpoint_{iteration:04d}.pt")
        torch.save(
            {
                "iteration": iteration,
                "model": model.state_dict(),
                "optimizer": optimizer.state_dict(),
            },
            checkpoint_path,
        )
        print(f"  Saved checkpoint: {checkpoint_path}")

    # Export to ONNX
    export_onnx(model, device, os.path.join(args.checkpoint_dir, "model.onnx"))


def export_onnx(model, device, path):
    """Export trained model to ONNX format."""
    from config import OBS_SIZE, NUM_ACTIONS

    model.eval()
    dummy_obs = torch.randn(1, OBS_SIZE, device=device)
    dummy_mask = torch.ones(1, NUM_ACTIONS, device=device)

    torch.onnx.export(
        model,
        (dummy_obs, dummy_mask),
        path,
        input_names=["observation", "legal_mask"],
        output_names=["log_policy", "value"],
        dynamic_axes={
            "observation": {0: "batch"},
            "legal_mask": {0: "batch"},
            "log_policy": {0: "batch"},
            "value": {0: "batch"},
        },
        opset_version=17,
        dynamo=False,
    )
    print(f"Exported ONNX model to {path}")


if __name__ == "__main__":
    main()
