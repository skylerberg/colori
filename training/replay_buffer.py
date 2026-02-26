"""Replay buffer for storing self-play training data."""

import numpy as np
from config import BUFFER_MAX_SIZE


class ReplayBuffer:
    """Fixed-size replay buffer storing training samples from self-play games."""

    def __init__(self, max_size=BUFFER_MAX_SIZE):
        self.max_size = max_size
        self.states = []
        self.action_features = []
        self.action_masks = []
        self.policies = []
        self.values = []
        self._size = 0

    def add_batch(self, states, action_features, action_masks, policies, values):
        """Add a batch of samples from self-play.

        Args:
            states: np.ndarray (N, 768)
            action_features: np.ndarray (N, max_actions, 86)
            action_masks: np.ndarray (N, max_actions) bool
            policies: np.ndarray (N, max_actions) visit distribution
            values: np.ndarray (N,) game outcome from acting player's perspective
        """
        n = len(states)

        self.states.append(states)
        self.action_features.append(action_features)
        self.action_masks.append(action_masks)
        self.policies.append(policies)
        self.values.append(values)
        self._size += n

        # Trim if over max size
        if self._size > self.max_size:
            self._compact()

    def _compact(self):
        """Merge all chunks and trim to max_size (keep most recent)."""
        all_states = np.concatenate(self.states, axis=0)
        all_action_features, all_action_masks, all_policies = pad_to_max_actions(
            self.action_features, self.action_masks, self.policies
        )
        all_values = np.concatenate(self.values, axis=0)

        # Keep most recent samples
        if len(all_states) > self.max_size:
            all_states = all_states[-self.max_size:]
            all_action_features = all_action_features[-self.max_size:]
            all_action_masks = all_action_masks[-self.max_size:]
            all_policies = all_policies[-self.max_size:]
            all_values = all_values[-self.max_size:]

        self.states = [all_states]
        self.action_features = [all_action_features]
        self.action_masks = [all_action_masks]
        self.policies = [all_policies]
        self.values = [all_values]
        self._size = len(all_states)

    def sample(self, batch_size):
        """Sample a random batch of training data.

        Returns dict with keys: states, action_features, action_masks, policies, values
        """
        self._compact()

        indices = np.random.choice(self._size, size=min(batch_size, self._size), replace=False)

        states = self.states[0][indices]
        action_features = self.action_features[0][indices]
        action_masks = self.action_masks[0][indices]
        policies = self.policies[0][indices]
        values = self.values[0][indices]

        # Pad action features/masks/policies to consistent max_actions
        max_actions = action_masks.shape[1]

        return {
            'states': states,
            'action_features': action_features,
            'action_masks': action_masks,
            'policies': policies,
            'values': values,
        }

    def __len__(self):
        return self._size


def pad_to_max_actions(action_features_list, action_masks_list, policies_list):
    """Pad variable-length action arrays to the same max_actions dimension.

    Args:
        action_features_list: list of (N_i, max_actions_i, 86) arrays
        action_masks_list: list of (N_i, max_actions_i) arrays
        policies_list: list of (N_i, max_actions_i) arrays

    Returns:
        Tuple of padded (action_features, action_masks, policies) arrays
    """
    max_actions = max(af.shape[1] for af in action_features_list)

    padded_features = []
    padded_masks = []
    padded_policies = []

    for af, am, p in zip(action_features_list, action_masks_list, policies_list):
        n, cur_max, feat_dim = af.shape
        if cur_max < max_actions:
            pad_width = max_actions - cur_max
            af = np.pad(af, ((0, 0), (0, pad_width), (0, 0)))
            am = np.pad(am, ((0, 0), (0, pad_width)))
            p = np.pad(p, ((0, 0), (0, pad_width)))
        padded_features.append(af)
        padded_masks.append(am)
        padded_policies.append(p)

    return (
        np.concatenate(padded_features, axis=0),
        np.concatenate(padded_masks, axis=0),
        np.concatenate(padded_policies, axis=0),
    )
