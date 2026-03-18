"""
Chart showing 95% confidence interval width for CMA-ES win-rate evaluation
at various numbers of games.

Each game outcome is approximately Bernoulli (win=1, loss=0, tie=0.5).
For a true win-rate p, the standard error of the estimated win-rate after
n games is sqrt(p*(1-p)/n), and the 95% CI is ±1.96 * SE.

We show this for several true win-rates, with worst case (p=0.5) highlighted.
"""

import numpy as np
import matplotlib.pyplot as plt

games = np.arange(10, 1001, 1)
true_rates = [0.50, 0.55, 0.60, 0.45, 0.40]

fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))

# Left: 95% CI half-width (margin of error) vs number of games
for p in sorted(true_rates):
    margin = 1.96 * np.sqrt(p * (1 - p) / games)
    label = f"true win-rate = {p:.0%}"
    lw = 2.5 if p == 0.50 else 1.2
    alpha = 1.0 if p == 0.50 else 0.6
    ax1.plot(games, margin, label=label, linewidth=lw, alpha=alpha)

ax1.set_xlabel("Number of Evaluation Games")
ax1.set_ylabel("95% CI Half-Width (± margin of error)")
ax1.set_title("Margin of Error vs. Evaluation Games")
ax1.legend()
ax1.grid(True, alpha=0.3)
ax1.axhline(y=0.05, color='red', linestyle='--', alpha=0.4, label='±5%')
ax1.axhline(y=0.03, color='orange', linestyle='--', alpha=0.4, label='±3%')
ax1.text(950, 0.052, '±5%', color='red', alpha=0.6, ha='right', fontsize=9)
ax1.text(950, 0.032, '±3%', color='orange', alpha=0.6, ha='right', fontsize=9)

# Right: Full CI band for p=0.50 (worst case)
p = 0.50
margin = 1.96 * np.sqrt(p * (1 - p) / games)
ax2.fill_between(games, p - margin, p + margin, alpha=0.3, color='steelblue')
ax2.plot(games, p + margin, color='steelblue', linewidth=1)
ax2.plot(games, p - margin, color='steelblue', linewidth=1)
ax2.axhline(y=0.50, color='black', linestyle='-', linewidth=0.8)
ax2.set_xlabel("Number of Evaluation Games")
ax2.set_ylabel("Measured Win-Rate")
ax2.set_title("95% CI Band (worst case: true win-rate = 50%)")
ax2.grid(True, alpha=0.3)

# Annotate specific game counts
for n in [50, 100, 200, 500]:
    m = 1.96 * np.sqrt(p * (1 - p) / n)
    ax2.annotate(
        f"n={n}\n±{m:.1%}",
        xy=(n, p + m), xytext=(n + 40, p + m + 0.04),
        arrowprops=dict(arrowstyle='->', color='gray'),
        fontsize=8, ha='center',
    )

# Table of key values
print("Number of Games | 95% CI Half-Width (worst case p=0.50)")
print("-" * 55)
for n in [25, 50, 75, 100, 150, 200, 300, 500, 750, 1000]:
    m = 1.96 * np.sqrt(0.5 * 0.5 / n)
    print(f"  {n:>4}           |  ±{m:.4f}  ({m:.1%})")

plt.tight_layout()
plt.savefig("eval_confidence.png", dpi=150, bbox_inches='tight')
print("\nSaved to eval_confidence.png")
