import math
import numpy as np
import colori_python as cp
from config import NUM_ACTIONS, C_PUCT, DIRICHLET_ALPHA, DIRICHLET_EPSILON


class MCTSNode:
    __slots__ = ["parent", "action", "prior", "visit_count", "value_sum", "children", "is_expanded"]

    def __init__(self, parent=None, action=None, prior=0.0):
        self.parent = parent
        self.action = action
        self.prior = prior
        self.visit_count = 0
        self.value_sum = 0.0
        self.children = []
        self.is_expanded = False

    @property
    def q_value(self):
        if self.visit_count == 0:
            return 0.0
        return self.value_sum / self.visit_count

    def ucb_score(self, parent_visits):
        return self.q_value + C_PUCT * self.prior * math.sqrt(parent_visits) / (1 + self.visit_count)


class MCTS:
    def __init__(self, model, num_simulations, device):
        self.model = model
        self.num_simulations = num_simulations
        self.device = device

    def search(self, game, player_index, add_noise=True):
        """Run MCTS from current game state, return visit count distribution."""
        import torch

        root = MCTSNode()

        # Get initial policy and value from network
        obs = game.get_observation(player_index)
        legal_mask = game.get_legal_mask()
        obs_tensor = torch.tensor(obs, dtype=torch.float32, device=self.device)
        mask_tensor = torch.tensor(legal_mask, dtype=torch.float32, device=self.device)

        policy, _ = self.model.predict(obs_tensor, mask_tensor)

        # Expand root
        legal_actions = game.get_legal_actions()
        self._expand(root, legal_actions, policy)

        # Add Dirichlet noise to root priors for exploration
        if add_noise and len(root.children) > 0:
            noise = np.random.dirichlet([DIRICHLET_ALPHA] * len(root.children))
            for child, n in zip(root.children, noise):
                child.prior = (1 - DIRICHLET_EPSILON) * child.prior + DIRICHLET_EPSILON * n

        for _ in range(self.num_simulations):
            node = root
            game_copy = game.clone_state()

            # Select
            while node.is_expanded and len(node.children) > 0:
                node = self._select_child(node)
                game_copy.apply_action(node.action)

                # Auto-advance draw phase
                if game_copy.is_draw_phase():
                    game_copy.advance_draw_phase()

            # Evaluate
            if game_copy.is_terminal():
                rewards = game_copy.get_rewards()
                value = rewards[player_index] * 2 - 1  # map [0,1] to [-1,1]
            else:
                current_player = game_copy.get_current_player()
                obs = game_copy.get_observation(current_player)
                legal_mask = game_copy.get_legal_mask()
                obs_tensor = torch.tensor(obs, dtype=torch.float32, device=self.device)
                mask_tensor = torch.tensor(legal_mask, dtype=torch.float32, device=self.device)

                policy, value = self.model.predict(obs_tensor, mask_tensor)

                # Expand
                legal_actions = game_copy.get_legal_actions()
                self._expand(node, legal_actions, policy)

                # Adjust value to be from the searching player's perspective
                if current_player != player_index:
                    value = -value

            # Backpropagate
            self._backpropagate(node, value, player_index, game_copy)

        # Build visit count distribution
        action_visits = np.zeros(NUM_ACTIONS, dtype=np.float32)
        for child in root.children:
            action_visits[child.action] = child.visit_count

        total = action_visits.sum()
        if total > 0:
            action_visits /= total

        return action_visits

    def _expand(self, node, legal_actions, policy):
        node.is_expanded = True
        total_prior = sum(policy[a] for a in legal_actions)
        for action in legal_actions:
            prior = policy[action] / total_prior if total_prior > 0 else 1.0 / len(legal_actions)
            child = MCTSNode(parent=node, action=action, prior=prior)
            node.children.append(child)

    def _select_child(self, node):
        best_score = -float("inf")
        best_child = None
        for child in node.children:
            score = child.ucb_score(node.visit_count)
            if score > best_score:
                best_score = score
                best_child = child
        return best_child

    def _backpropagate(self, node, value, player_index, game_copy):
        while node is not None:
            node.visit_count += 1
            node.value_sum += value
            node = node.parent
