"""
Quantify CMA-ES selection pressure under Bernoulli evaluation noise.

Key question: how often does noise cause a generation to move the mean
in the WRONG direction (away from the true optimum)?

The mean update is Δm = Σ w_i * (x_{π(i)} - m), where π(i) ranks by
noisy fitness. The "true quality" of this update is Σ w_i * f_true(x_{π(i)}).
If this is negative, the generation made things worse.
"""

import numpy as np
import matplotlib.pyplot as plt
from scipy.stats import norm

np.random.seed(42)
N_MC = 500_000

def cma_weights(lam):
    mu = lam // 2
    raw = np.log(mu + 0.5) - np.log(np.arange(1, mu + 1).astype(float))
    return raw / raw.sum()

def mu_eff(lam):
    w = cma_weights(lam)
    return 1.0 / np.sum(w**2)

LAM = 14
MU = LAM // 2
W = cma_weights(LAM)
MU_EFF = mu_eff(LAM)

print(f"λ={LAM}, μ={MU}, μ_eff={MU_EFF:.2f}")
print(f"Weights: {np.round(W, 3)}")
print()

def simulate_selection(rho, n_trials=N_MC):
    """
    Simulate one CMA-ES generation's selection step in 1D.

    True fitnesses z_i ~ N(0,1).
    Noisy fitnesses g_i = z_i + ε_i, Corr(z,g) = rho.
    Select top-μ by g, compute weighted sum of true z values.
    """
    sigma_eps = np.sqrt((1 - rho**2) / rho**2) if rho < 0.999 else 0.0

    z = np.random.randn(n_trials, LAM)  # true fitness
    eps = sigma_eps * np.random.randn(n_trials, LAM)  # noise
    g = z + eps  # noisy fitness

    # Rank by noisy fitness (descending)
    noisy_order = np.argsort(-g, axis=1)  # indices sorted by noisy fitness desc
    true_order = np.argsort(-z, axis=1)   # true ranking

    # Weighted selection response (true fitness of noisy-selected individuals)
    selection_response = np.zeros(n_trials)
    for i in range(MU):
        selected_idx = noisy_order[:, i]
        selected_true = z[np.arange(n_trials), selected_idx]
        selection_response += W[i] * selected_true

    # Noiseless selection response (for comparison)
    noiseless_response = np.zeros(n_trials)
    for i in range(MU):
        selected_idx = true_order[:, i]
        selected_true = z[np.arange(n_trials), selected_idx]
        noiseless_response += W[i] * selected_true

    # Count intruders: true bottom-half individuals that got selected
    intruders = np.zeros(n_trials)
    for trial in range(min(n_trials, 100_000)):  # limit for speed
        true_bottom = set(true_order[trial, MU:])
        noisy_top = set(noisy_order[trial, :MU])
        intruders[trial] = len(true_bottom & noisy_top)

    # True fitness of individual ranked #1 by noisy eval
    best_noisy_idx = noisy_order[:, 0]
    true_fitness_of_noisy_best = z[np.arange(n_trials), best_noisy_idx]

    # True rank of individual ranked #1 by noisy eval
    true_ranks = np.argsort(np.argsort(-z, axis=1), axis=1)  # 0-indexed true rank
    true_rank_of_noisy_best = true_ranks[np.arange(n_trials), best_noisy_idx] + 1

    return {
        'selection_response': selection_response,
        'noiseless_response': noiseless_response,
        'p_regression': np.mean(selection_response < 0),
        'mean_response': np.mean(selection_response),
        'mean_noiseless': np.mean(noiseless_response),
        'efficiency': np.mean(selection_response) / np.mean(noiseless_response),
        'mean_intruders': np.mean(intruders[:100_000]),
        'true_fitness_of_best': true_fitness_of_noisy_best,
        'true_rank_of_best': true_rank_of_noisy_best,
    }


# ── Compute for range of rho values ──

def rho_from_n(n_games, sigma_f):
    return sigma_f / np.sqrt(sigma_f**2 + 0.25 / n_games)

# Build results for various (n_games, sigma_f) combos
sigma_f_values = [0.02, 0.03, 0.04, 0.06]
n_games_range = [10, 25, 50, 75, 100, 150, 200, 300, 500, 1000]

fig, axes = plt.subplots(2, 2, figsize=(15, 12))

# ━━━ Chart 1: P(regression) vs n_games ━━━
ax = axes[0, 0]
print("P(generation moves mean in WRONG direction):")
print(f"{'n_games':>8}", end="")
for sf in sigma_f_values:
    print(f"  σ_f={sf:.2f}", end="")
print()
print("-" * 60)

for sf in sigma_f_values:
    p_regs = []
    for n in n_games_range:
        r = rho_from_n(n, sf)
        result = simulate_selection(r, n_trials=200_000)
        p_regs.append(result['p_regression'])
    ax.plot(n_games_range, p_regs, 'o-', label=f"σ_f={sf}", linewidth=1.5, markersize=4)

    # Print
    for n, p in zip(n_games_range, p_regs):
        pass
    print(f"  n={n_games_range[-1]:>4}:", "  ".join(f"{p:.1%}" for p in p_regs[-1:]))

# Print full table
print()
print("Full table: P(regression)")
print(f"{'n_games':>8}", end="")
for sf in sigma_f_values:
    print(f"{'σ_f='+str(sf):>10}", end="")
print()

all_p_regs = {}
for sf in sigma_f_values:
    all_p_regs[sf] = []
    for n in n_games_range:
        r = rho_from_n(n, sf)
        result = simulate_selection(r, n_trials=200_000)
        all_p_regs[sf].append(result['p_regression'])

for i, n in enumerate(n_games_range):
    print(f"{n:>8}", end="")
    for sf in sigma_f_values:
        print(f"{all_p_regs[sf][i]:>10.1%}", end="")
    print()

ax.axvline(x=100, color='red', linestyle='--', alpha=0.5)
ax.text(105, ax.get_ylim()[1] * 0.9 if ax.get_ylim()[1] > 0 else 0.3, 'current', color='red', fontsize=9)
ax.set_xlabel("n (games per eval)")
ax.set_ylabel("P(generation hurts)")
ax.set_title("Probability a generation moves mean AWAY from optimum")
ax.legend()
ax.grid(True, alpha=0.3)

# ━━━ Chart 2: Expected intruders vs n_games ━━━
ax = axes[0, 1]
for sf in sigma_f_values:
    intruders = []
    for n in n_games_range:
        r = rho_from_n(n, sf)
        result = simulate_selection(r, n_trials=100_000)
        intruders.append(result['mean_intruders'])
    ax.plot(n_games_range, intruders, 'o-', label=f"σ_f={sf}", linewidth=1.5, markersize=4)

ax.axhline(y=0, color='gray', linestyle='-', alpha=0.3)
ax.axhline(y=MU/2, color='gray', linestyle=':', alpha=0.3)
ax.text(800, MU/2 + 0.1, f"random selection ({MU/2})", fontsize=8, color='gray')
ax.axvline(x=100, color='red', linestyle='--', alpha=0.5)
ax.set_xlabel("n (games per eval)")
ax.set_ylabel(f"Expected intruders (out of μ={MU} selected)")
ax.set_title("Bottom-half individuals sneaking into selection")
ax.legend()
ax.grid(True, alpha=0.3)

# ━━━ Chart 3: True rank of noisy-best individual ━━━
ax = axes[1, 0]
sf = 0.03  # focus on one sigma_f
key_ns = [25, 50, 100, 200, 500]
for n in key_ns:
    r = rho_from_n(n, sf)
    result = simulate_selection(r, n_trials=300_000)
    ranks = result['true_rank_of_best']
    # Histogram of true ranks
    counts = np.bincount(ranks.astype(int), minlength=LAM+1)[1:LAM+1]
    ax.bar(np.arange(1, LAM+1) + (key_ns.index(n) - 2) * 0.15,
           counts / len(ranks), width=0.15, label=f"n={n}", alpha=0.8)

ax.set_xlabel("True rank of individual ranked #1 by noisy evaluation")
ax.set_ylabel("Probability")
ax.set_title(f"How good is the 'best' individual really? (σ_f={sf})")
ax.legend()
ax.grid(True, alpha=0.3)
ax.set_xticks(range(1, LAM+1))

# ━━━ Chart 4: Selection efficiency ━━━
ax = axes[1, 1]
print("\nSelection efficiency (actual / noiseless progress):")
for sf in sigma_f_values:
    effs = []
    for n in n_games_range:
        r = rho_from_n(n, sf)
        result = simulate_selection(r, n_trials=200_000)
        effs.append(result['efficiency'])
    ax.plot(n_games_range, effs, 'o-', label=f"σ_f={sf}", linewidth=1.5, markersize=4)

ax.axhline(y=1.0, color='gray', linestyle='--', alpha=0.5)
ax.axvline(x=100, color='red', linestyle='--', alpha=0.5)
ax.set_xlabel("n (games per eval)")
ax.set_ylabel("Efficiency (actual / noiseless progress)")
ax.set_title("Selection efficiency: fraction of theoretical progress achieved")
ax.legend()
ax.grid(True, alpha=0.3)
ax.set_ylim(0, 1.05)

plt.tight_layout()
plt.savefig("selection_pressure.png", dpi=150, bbox_inches='tight')
print("\nSaved selection_pressure.png")

# ── Detailed printout for current setup ──
print("\n" + "="*70)
print("DETAILED ANALYSIS FOR CURRENT SETUP (λ=14, n=100)")
print("="*70)
for sf in [0.02, 0.03, 0.04, 0.05]:
    r = rho_from_n(100, sf)
    result = simulate_selection(r, n_trials=500_000)
    print(f"\nσ_f = {sf} (ρ = {r:.3f}):")
    print(f"  P(generation hurts):           {result['p_regression']:.1%}")
    print(f"  Expected intruders in top-7:    {result['mean_intruders']:.2f} / 7")
    print(f"  Selection efficiency:           {result['efficiency']:.1%}")
    print(f"  P('best' is truly top-3):       {np.mean(result['true_rank_of_best'] <= 3):.1%}")
    print(f"  P('best' is truly bottom-half): {np.mean(result['true_rank_of_best'] > 7):.1%}")
