import numpy as np
from config import OBS_SIZE, NUM_ACTIONS, REPLAY_BUFFER_SIZE


class ReplayBuffer:
    def __init__(self, max_size=REPLAY_BUFFER_SIZE):
        self.max_size = max_size
        self.observations = np.zeros((max_size, OBS_SIZE), dtype=np.float32)
        self.legal_masks = np.zeros((max_size, NUM_ACTIONS), dtype=np.float32)
        self.policies = np.zeros((max_size, NUM_ACTIONS), dtype=np.float32)
        self.values = np.zeros(max_size, dtype=np.float32)
        self.size = 0
        self.index = 0

    def add(self, obs, legal_mask, policy, value):
        self.observations[self.index] = obs
        self.legal_masks[self.index] = legal_mask
        self.policies[self.index] = policy
        self.values[self.index] = value
        self.index = (self.index + 1) % self.max_size
        self.size = min(self.size + 1, self.max_size)

    def add_batch(self, samples):
        """Add a list of (obs, legal_mask, policy, value) tuples."""
        for obs, legal_mask, policy, value in samples:
            self.add(obs, legal_mask, policy, value)

    def sample(self, batch_size):
        indices = np.random.choice(self.size, size=min(batch_size, self.size), replace=False)
        return (
            self.observations[indices],
            self.legal_masks[indices],
            self.policies[indices],
            self.values[indices],
        )

    def __len__(self):
        return self.size
