import torch
import torch.nn as nn
import torch.nn.functional as F
from config import OBS_SIZE, NUM_ACTIONS, HIDDEN_SIZE, NUM_RES_BLOCKS


class ResBlock(nn.Module):
    def __init__(self, hidden_size):
        super().__init__()
        self.fc1 = nn.Linear(hidden_size, hidden_size)
        self.bn1 = nn.BatchNorm1d(hidden_size)
        self.fc2 = nn.Linear(hidden_size, hidden_size)
        self.bn2 = nn.BatchNorm1d(hidden_size)

    def forward(self, x):
        residual = x
        out = F.relu(self.bn1(self.fc1(x)))
        out = self.bn2(self.fc2(out))
        out = F.relu(out + residual)
        return out


class ColoriNet(nn.Module):
    def __init__(
        self,
        obs_size=OBS_SIZE,
        num_actions=NUM_ACTIONS,
        hidden_size=HIDDEN_SIZE,
        num_res_blocks=NUM_RES_BLOCKS,
    ):
        super().__init__()
        self.stem = nn.Sequential(
            nn.Linear(obs_size, hidden_size),
            nn.BatchNorm1d(hidden_size),
            nn.ReLU(),
        )
        self.res_blocks = nn.Sequential(
            *[ResBlock(hidden_size) for _ in range(num_res_blocks)]
        )
        # Policy head
        self.policy_fc1 = nn.Linear(hidden_size, 128)
        self.policy_fc2 = nn.Linear(128, num_actions)
        # Value head
        self.value_fc1 = nn.Linear(hidden_size, 64)
        self.value_fc2 = nn.Linear(64, 1)

    def forward(self, obs, legal_mask):
        """
        Args:
            obs: [batch, OBS_SIZE]
            legal_mask: [batch, NUM_ACTIONS] with 1.0 for legal, 0.0 for illegal
        Returns:
            policy: [batch, NUM_ACTIONS] log probabilities (masked)
            value: [batch, 1] in range [-1, 1]
        """
        x = self.stem(obs)
        x = self.res_blocks(x)

        # Policy head
        logits = self.policy_fc2(F.relu(self.policy_fc1(x)))
        # Mask illegal actions with large negative value
        logits = logits - (1 - legal_mask) * 1e8
        policy = F.log_softmax(logits, dim=-1)

        # Value head
        value = torch.tanh(self.value_fc2(F.relu(self.value_fc1(x))))

        return policy, value

    def predict(self, obs, legal_mask):
        """Single-sample inference without gradients."""
        self.eval()
        with torch.no_grad():
            if obs.dim() == 1:
                obs = obs.unsqueeze(0)
                legal_mask = legal_mask.unsqueeze(0)
            log_policy, value = self(obs, legal_mask)
            policy = torch.exp(log_policy).squeeze(0)
            value = value.squeeze(0).item()
        return policy.cpu().numpy(), value
