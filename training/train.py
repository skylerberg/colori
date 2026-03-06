import torch
import torch.nn.functional as F
from config import BATCH_SIZE, LEARNING_RATE, WEIGHT_DECAY, NUM_EPOCHS


def train_epoch(model, optimizer, replay_buffer, device):
    """Train one epoch on samples from the replay buffer."""
    model.train()
    total_policy_loss = 0
    total_value_loss = 0
    num_batches = 0

    num_samples = len(replay_buffer)
    num_batches_per_epoch = max(1, num_samples // BATCH_SIZE)

    for _ in range(num_batches_per_epoch):
        obs, legal_masks, target_policies, target_values = replay_buffer.sample(BATCH_SIZE)

        obs_t = torch.tensor(obs, dtype=torch.float32, device=device)
        masks_t = torch.tensor(legal_masks, dtype=torch.float32, device=device)
        target_pi = torch.tensor(target_policies, dtype=torch.float32, device=device)
        target_v = torch.tensor(target_values, dtype=torch.float32, device=device)

        log_policy, value = model(obs_t, masks_t)
        value = value.squeeze(-1)

        # Policy loss: cross-entropy with MCTS visit distribution
        policy_loss = -torch.sum(target_pi * log_policy, dim=-1).mean()

        # Value loss: MSE
        value_loss = F.mse_loss(value, target_v)

        loss = policy_loss + value_loss

        optimizer.zero_grad()
        loss.backward()
        optimizer.step()

        total_policy_loss += policy_loss.item()
        total_value_loss += value_loss.item()
        num_batches += 1

    avg_policy_loss = total_policy_loss / max(1, num_batches)
    avg_value_loss = total_value_loss / max(1, num_batches)
    return avg_policy_loss, avg_value_loss


def create_optimizer(model):
    return torch.optim.Adam(
        model.parameters(),
        lr=LEARNING_RATE,
        weight_decay=WEIGHT_DECAY,
    )
