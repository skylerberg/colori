import math
import numpy as np
import torch
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


def _select_child(node):
    best_score = -float("inf")
    best_child = None
    for child in node.children:
        score = child.ucb_score(node.visit_count)
        if score > best_score:
            best_score = score
            best_child = child
    return best_child


def _expand(node, legal_actions, policy):
    node.is_expanded = True
    total_prior = sum(policy[a] for a in legal_actions)
    for action in legal_actions:
        prior = policy[action] / total_prior if total_prior > 0 else 1.0 / len(legal_actions)
        child = MCTSNode(parent=node, action=action, prior=prior)
        node.children.append(child)


def _backpropagate(node, value):
    while node is not None:
        node.visit_count += 1
        node.value_sum += value
        node = node.parent


class MCTS:
    """Single-game MCTS (CPU, batch-1 inference). Kept for compatibility."""

    def __init__(self, model, num_simulations, device):
        self.model = model
        self.num_simulations = num_simulations
        self.device = device

    def search(self, game, player_index, add_noise=True):
        """Run MCTS from current game state, return visit count distribution."""
        root = MCTSNode()

        obs = game.get_observation(player_index)
        legal_mask = game.get_legal_mask()
        obs_tensor = torch.tensor(obs, dtype=torch.float32, device=self.device)
        mask_tensor = torch.tensor(legal_mask, dtype=torch.float32, device=self.device)

        policy, _ = self.model.predict(obs_tensor, mask_tensor)

        legal_actions = game.get_legal_actions()
        _expand(root, legal_actions, policy)

        if add_noise and len(root.children) > 0:
            noise = np.random.dirichlet([DIRICHLET_ALPHA] * len(root.children))
            for child, n in zip(root.children, noise):
                child.prior = (1 - DIRICHLET_EPSILON) * child.prior + DIRICHLET_EPSILON * n

        for _ in range(self.num_simulations):
            node = root
            game_copy = game.clone_state()

            while node.is_expanded and len(node.children) > 0:
                node = _select_child(node)
                game_copy.apply_action(node.action)
                if game_copy.is_draw_phase():
                    game_copy.advance_draw_phase()

            if game_copy.is_terminal():
                rewards = game_copy.get_rewards()
                value = rewards[player_index] * 2 - 1
            else:
                current_player = game_copy.get_current_player()
                obs = game_copy.get_observation(current_player)
                legal_mask = game_copy.get_legal_mask()
                obs_tensor = torch.tensor(obs, dtype=torch.float32, device=self.device)
                mask_tensor = torch.tensor(legal_mask, dtype=torch.float32, device=self.device)

                policy, value = self.model.predict(obs_tensor, mask_tensor)

                legal_actions = game_copy.get_legal_actions()
                _expand(node, legal_actions, policy)

                if current_player != player_index:
                    value = -value

            _backpropagate(node, value)

        action_visits = np.zeros(NUM_ACTIONS, dtype=np.float32)
        for child in root.children:
            action_visits[child.action] = child.visit_count

        total = action_visits.sum()
        if total > 0:
            action_visits /= total

        return action_visits


class BatchedMCTS:
    """Runs MCTS for multiple games in lockstep, batching NN evaluations on GPU."""

    def __init__(self, model, num_simulations, device):
        self.model = model
        self.num_simulations = num_simulations
        self.device = device

    def search_batch(self, games, player_indices, add_noise=True):
        """Run MCTS for multiple games simultaneously, batching leaf evaluations.

        Args:
            games: list of game states (one per concurrent game)
            player_indices: list of current player index per game
        Returns:
            list of visit count distributions (one per game)
        """
        n = len(games)
        roots = [MCTSNode() for _ in range(n)]

        # Batch-evaluate root nodes
        obs_list = []
        mask_list = []
        for i in range(n):
            obs_list.append(games[i].get_observation(player_indices[i]))
            mask_list.append(games[i].get_legal_mask())

        obs_batch = torch.tensor(np.array(obs_list), dtype=torch.float32, device=self.device)
        mask_batch = torch.tensor(np.array(mask_list), dtype=torch.float32, device=self.device)
        policies, _ = self.model.predict_batch(obs_batch, mask_batch)

        # Expand roots and add noise
        for i in range(n):
            legal_actions = games[i].get_legal_actions()
            _expand(roots[i], legal_actions, policies[i])
            if add_noise and len(roots[i].children) > 0:
                noise = np.random.dirichlet([DIRICHLET_ALPHA] * len(roots[i].children))
                for child, nv in zip(roots[i].children, noise):
                    child.prior = (1 - DIRICHLET_EPSILON) * child.prior + DIRICHLET_EPSILON * nv

        # Run simulations in lockstep
        for _ in range(self.num_simulations):
            # Phase 1: Selection — traverse trees, collect leaf states
            leaf_nodes = []
            leaf_game_copies = []
            leaf_players = []  # searching player for each leaf
            leaf_current_players = []  # current player at leaf (for NN eval)
            terminal_indices = []  # games that hit terminal during selection

            for i in range(n):
                node = roots[i]
                game_copy = games[i].clone_state()

                # Select down the tree
                while node.is_expanded and len(node.children) > 0:
                    node = _select_child(node)
                    game_copy.apply_action(node.action)
                    if game_copy.is_draw_phase():
                        game_copy.advance_draw_phase()

                if game_copy.is_terminal():
                    # Handle terminal immediately
                    rewards = game_copy.get_rewards()
                    value = rewards[player_indices[i]] * 2 - 1
                    _backpropagate(node, value)
                    terminal_indices.append(i)
                else:
                    leaf_nodes.append(node)
                    leaf_game_copies.append(game_copy)
                    leaf_players.append(player_indices[i])
                    leaf_current_players.append(game_copy.get_current_player())

            if not leaf_nodes:
                continue

            # Phase 2: Batch evaluate all non-terminal leaves on GPU
            obs_list = []
            mask_list = []
            for j, game_copy in enumerate(leaf_game_copies):
                obs_list.append(game_copy.get_observation(leaf_current_players[j]))
                mask_list.append(game_copy.get_legal_mask())

            obs_batch = torch.tensor(np.array(obs_list), dtype=torch.float32, device=self.device)
            mask_batch = torch.tensor(np.array(mask_list), dtype=torch.float32, device=self.device)
            policies, values = self.model.predict_batch(obs_batch, mask_batch)

            # Phase 3: Expand and backpropagate
            for j in range(len(leaf_nodes)):
                node = leaf_nodes[j]
                game_copy = leaf_game_copies[j]
                policy = policies[j]
                value = float(values[j])

                legal_actions = game_copy.get_legal_actions()
                _expand(node, legal_actions, policy)

                # Adjust value to searching player's perspective
                if leaf_current_players[j] != leaf_players[j]:
                    value = -value

                _backpropagate(node, value)

        # Build visit distributions
        results = []
        for i in range(n):
            action_visits = np.zeros(NUM_ACTIONS, dtype=np.float32)
            for child in roots[i].children:
                action_visits[child.action] = child.visit_count
            total = action_visits.sum()
            if total > 0:
                action_visits /= total
            results.append(action_visits)

        return results
