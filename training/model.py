"""Neural network model for Colori AlphaZero training."""

import torch
import torch.nn as nn
import torch.nn.functional as F
import numpy as np
from config import (
    STATE_ENCODING_SIZE, ACTION_ENCODING_SIZE,
    STATE_HIDDEN_DIM, STATE_EMBED_DIM, ACTION_HIDDEN_DIM, NUM_PLAYERS,
)


class ColoriNet(nn.Module):
    """
    AlphaZero-style network for Colori.

    State encoder produces a fixed-size embedding.
    Value head predicts win probability for each player.
    Policy head scores each legal action via dot product with action embeddings.
    """

    def __init__(self):
        super().__init__()

        # State encoder: 768 -> 256 -> 256 -> 128
        self.state_encoder = nn.Sequential(
            nn.Linear(STATE_ENCODING_SIZE, STATE_HIDDEN_DIM),
            nn.ReLU(),
            nn.Linear(STATE_HIDDEN_DIM, STATE_HIDDEN_DIM),
            nn.ReLU(),
            nn.Linear(STATE_HIDDEN_DIM, STATE_EMBED_DIM),
            nn.ReLU(),
        )

        # Value head: 128 -> 64 -> 3 (win prob per player, from perspective player's view)
        self.value_head = nn.Sequential(
            nn.Linear(STATE_EMBED_DIM, 64),
            nn.ReLU(),
            nn.Linear(64, NUM_PLAYERS),
        )

        # Action encoder: 86 -> 64 -> 128 (to match state embed dim for dot product)
        self.action_encoder = nn.Sequential(
            nn.Linear(ACTION_ENCODING_SIZE, ACTION_HIDDEN_DIM),
            nn.ReLU(),
            nn.Linear(ACTION_HIDDEN_DIM, STATE_EMBED_DIM),
        )

    def forward(self, state, action_features, action_mask):
        """
        Args:
            state: (batch, 768) state encoding
            action_features: (batch, max_actions, 86) action feature vectors
            action_mask: (batch, max_actions) bool mask (True = valid action)

        Returns:
            policy_logits: (batch, max_actions) unnormalized log-probs
            value: (batch, 3) win probabilities (softmax'd)
        """
        # Encode state
        state_embed = self.state_encoder(state)  # (batch, 128)

        # Value prediction
        value_logits = self.value_head(state_embed)  # (batch, 3)
        value = F.softmax(value_logits, dim=-1)

        # Encode actions
        batch_size, max_actions, _ = action_features.shape
        flat_actions = action_features.reshape(-1, ACTION_ENCODING_SIZE)  # (batch*max_actions, 86)
        action_embed = self.action_encoder(flat_actions)  # (batch*max_actions, 128)
        action_embed = action_embed.reshape(batch_size, max_actions, STATE_EMBED_DIM)  # (batch, max_actions, 128)

        # Policy: dot product between state embedding and each action embedding
        # state_embed: (batch, 128) -> (batch, 1, 128)
        policy_logits = torch.sum(
            state_embed.unsqueeze(1) * action_embed, dim=-1
        )  # (batch, max_actions)

        # Mask invalid actions with large negative value
        policy_logits = policy_logits.masked_fill(~action_mask, float('-inf'))

        return policy_logits, value

    def predict(self, state, action_features, action_mask):
        """Convenience method that returns softmax'd policy."""
        with torch.no_grad():
            policy_logits, value = self.forward(state, action_features, action_mask)
            policy = F.softmax(policy_logits, dim=-1)
            # Replace NaN (from all-masked rows) with 0
            policy = torch.nan_to_num(policy, 0.0)
            return policy, value


def export_onnx(model, path, max_actions=256):
    """Export model to ONNX format for Rust inference.

    The exported model takes:
      - state: (1, 768)
      - action_features: (1, N, 86) where N is dynamic
      - action_mask: (1, N) bool

    And returns:
      - policy_logits: (1, N)
      - value: (1, 3)
    """
    model.eval()

    dummy_state = torch.randn(1, STATE_ENCODING_SIZE)
    dummy_actions = torch.randn(1, max_actions, ACTION_ENCODING_SIZE)
    dummy_mask = torch.ones(1, max_actions, dtype=torch.bool)

    torch.onnx.export(
        model,
        (dummy_state, dummy_actions, dummy_mask),
        path,
        input_names=['state', 'action_features', 'action_mask'],
        output_names=['policy_logits', 'value'],
        dynamic_axes={
            'state': {0: 'batch'},
            'action_features': {0: 'batch', 1: 'num_actions'},
            'action_mask': {0: 'batch', 1: 'num_actions'},
            'policy_logits': {0: 'batch', 1: 'num_actions'},
            'value': {0: 'batch'},
        },
        opset_version=17,
        dynamo=False,
    )
    print(f"Model exported to {path}")


def create_initial_model(path):
    """Create and export a randomly initialized model."""
    model = ColoriNet()
    export_onnx(model, path)
    return model
