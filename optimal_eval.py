"""
Optimal CMA-ES evaluation budget allocation for Bernoulli (win/loss) fitness.

Key insight: CMA-ES progress per generation is proportional to:
    c_μw(λ) * ρ(n, σ_f)

where:
  - c_μw(λ) = weighted selection intensity (how much info the ranking provides)
  - ρ(n, σ_f) = σ_f / sqrt(σ_f² + 0.25/n) = correlation between true & noisy fitness
  - σ_f = std dev of true fitnesses among offspring (the "signal")
  - 0.25/n = Bernoulli noise variance per game (worst case at p=0.5)

For fixed budget B = λ*n, we maximize c_μw(B/n) * ρ(n, σ_f) over n.
"""

import numpy as np
import matplotlib.pyplot as plt

np.random.seed(42)

# ── Precompute weighted selection intensity c_μw(λ) via Monte Carlo ──

def cma_weights(lam):
    mu = lam // 2
    raw = np.log(mu + 0.5) - np.log(np.arange(1, mu + 1).astype(float))
    return raw / raw.sum()

print("Precomputing c_μw(λ) for λ = 4..400...")
N_MC = 500_000
rng = np.random.default_rng(42)
c_muw = {}
for lam in range(4, 402, 2):
    mu = lam // 2
    w = cma_weights(lam)
    samples = rng.standard_normal((N_MC, lam))
    sorted_desc = np.sort(samples, axis=1)[:, ::-1]
    eos = sorted_desc[:, :mu].mean(axis=0)
    c_muw[lam] = float(np.sum(w * eos))

def get_c_muw(lam):
    lam = max(4, min(400, int(round(lam / 2) * 2)))
    return c_muw.get(lam, c_muw[400])

def rho(n, sigma_f):
    return sigma_f / np.sqrt(sigma_f**2 + 0.25 / n)

def progress(lam, n, sigma_f):
    """Progress per generation ∝ c_μw(λ) * ρ(n, σ_f)."""
    return get_c_muw(lam) * rho(n, sigma_f)

def find_optimal(B, sigma_f):
    """Find optimal (n, λ) split for budget B and signal strength σ_f."""
    best_val, best_n, best_lam = -1, 1, B
    for n in range(1, B // 4 + 1):
        lam = B // n
        if lam < 4 or lam > 400:
            continue
        lam = int(round(lam / 2) * 2)
        val = progress(lam, n, sigma_f)
        if val > best_val:
            best_val, best_n, best_lam = val, n, lam
    return best_n, best_lam, best_val


# ── Charts ──

fig, axes = plt.subplots(2, 2, figsize=(15, 12))

# ━━━ Chart 1: Progress landscape for B=1400 ━━━
ax = axes[0, 0]
B = 1400
for sf in [0.01, 0.02, 0.03, 0.05, 0.08]:
    ns = []
    vals = []
    for n in range(2, B // 4 + 1):
        lam = B // n
        if lam < 4 or lam > 400:
            continue
        lam = int(round(lam / 2) * 2)
        ns.append(n)
        vals.append(progress(lam, n, sf))
    vals = np.array(vals)
    if len(vals) == 0:
        continue
    ax.plot(ns, vals / vals.max(), label=f"σ_f={sf:.2f}", linewidth=1.5)

ax.axvline(x=100, color='red', linestyle='--', alpha=0.6, linewidth=1)
ax.text(105, 0.95, 'current\n(n=100)', color='red', fontsize=8)
ax.set_xlabel("n (games per eval)")
ax.set_ylabel("Relative progress per generation")
ax.set_title(f"Budget B = {B} games/gen: efficiency vs allocation")
ax.legend(fontsize=9)
ax.grid(True, alpha=0.3)
ax.set_xlim(0, 350)

# ━━━ Chart 2: Optimal n vs σ_f for various budgets ━━━
ax = axes[0, 1]
sigma_fs = np.linspace(0.005, 0.12, 300)
for B in [700, 1400, 2800, 5600]:
    opt_ns = [find_optimal(B, sf)[0] for sf in sigma_fs]
    ax.plot(sigma_fs, opt_ns, label=f"B={B}", linewidth=1.5)

ax.set_xlabel("σ_f (true fitness spread among offspring)")
ax.set_ylabel("Optimal n (games per eval)")
ax.set_title("Optimal games per eval vs signal strength")
ax.legend(fontsize=9)
ax.grid(True, alpha=0.3)
# Add secondary x-axis showing "typical win-rate range"
# range ≈ 4*σ_f for 14 individuals (actually E[range] ≈ 3.5*σ_f for n=14)
ax2 = ax.twiny()
ax2.set_xlim(ax.get_xlim())
range_ticks = [0.02, 0.04, 0.06, 0.08, 0.10, 0.12]
ax2.set_xticks(range_ticks)
ax2.set_xticklabels([f"{4*t:.0%}" for t in range_ticks])
ax2.set_xlabel("Approx range of true win-rates (≈4σ_f)", fontsize=9)

# ━━━ Chart 3: Efficiency gain from optimal vs current (n=100, λ=14) ━━━
ax = axes[1, 0]
B = 1400
current_lam, current_n = 14, 100
for sf in sigma_fs:
    pass  # compute below

gains = []
opt_ns_1400 = []
opt_lams = []
for sf in sigma_fs:
    current_prog = progress(current_lam, current_n, sf)
    opt_n, opt_lam, opt_prog = find_optimal(B, sf)
    gains.append(opt_prog / current_prog if current_prog > 0 else 1)
    opt_ns_1400.append(opt_n)
    opt_lams.append(opt_lam)

ax.plot(sigma_fs, gains, color='steelblue', linewidth=2)
ax.axhline(y=1.0, color='gray', linestyle='--', alpha=0.5)
ax.set_xlabel("σ_f (true fitness spread)")
ax.set_ylabel("Speedup (optimal / current)")
ax.set_title(f"Potential speedup over current setup (λ={current_lam}, n={current_n})")
ax.grid(True, alpha=0.3)
# Annotate
ax.fill_between(sigma_fs, 1, gains, alpha=0.15, color='steelblue')

# Add text showing optimal configs at key points
for sf_mark in [0.02, 0.04, 0.06, 0.08]:
    idx = np.argmin(np.abs(sigma_fs - sf_mark))
    opt_n, opt_lam, _ = find_optimal(B, sf_mark)
    ax.annotate(f"λ={opt_lam},n={opt_n}",
                xy=(sf_mark, gains[idx]),
                xytext=(sf_mark + 0.01, gains[idx] + 0.05),
                fontsize=7, arrowprops=dict(arrowstyle='->', color='gray', lw=0.8))

# ━━━ Chart 4: Decomposition — ρ and c_μw contributions ━━━
ax = axes[1, 1]
B = 1400
sf = 0.03  # typical
ns = np.arange(2, B // 4 + 1)
rhos = [rho(n, sf) for n in ns]
lams = [B // n for n in ns]
c_muws = [get_c_muw(l) for l in lams]
products = [c * r for c, r in zip(c_muws, rhos)]

ax.plot(ns, np.array(rhos) / max(rhos), label="ρ(n) — noise reduction", linewidth=1.5)
ax.plot(ns, np.array(c_muws) / max(c_muws), label="c_μw(B/n) — selection quality from pop size", linewidth=1.5)
ax.plot(ns, np.array(products) / max(products), label="product (overall progress)", linewidth=2, color='black')
opt_n, _, _ = find_optimal(B, sf)
ax.axvline(x=opt_n, color='green', linestyle='--', alpha=0.6)
ax.text(opt_n + 3, 0.5, f"optimal n={opt_n}", color='green', fontsize=9, rotation=90)
ax.axvline(x=100, color='red', linestyle='--', alpha=0.4)
ax.text(103, 0.45, "current n=100", color='red', fontsize=9, rotation=90)
ax.set_xlabel("n (games per eval)")
ax.set_ylabel("Normalized value")
ax.set_title(f"Decomposition at σ_f={sf} (B={B})")
ax.legend(fontsize=9)
ax.grid(True, alpha=0.3)
ax.set_xlim(0, 300)

plt.tight_layout()
plt.savefig("optimal_eval.png", dpi=150, bbox_inches='tight')
print("Saved optimal_eval.png")

# ── Print summary table ──
print("\n" + "="*75)
print("SUMMARY: Optimal allocation for B=1400 games/generation")
print("="*75)
print(f"{'σ_f':>8} {'true range':>12} {'opt n':>7} {'opt λ':>7} {'current':>10} {'optimal':>10} {'speedup':>8}")
print("-"*75)
for sf in [0.01, 0.015, 0.02, 0.025, 0.03, 0.04, 0.05, 0.06, 0.08, 0.10]:
    opt_n, opt_lam, opt_prog = find_optimal(1400, sf)
    cur_prog = progress(14, 100, sf)
    rng_pct = f"±{2*sf:.0%}"
    print(f"{sf:>8.3f} {rng_pct:>12} {opt_n:>7} {opt_lam:>7} {cur_prog:>10.4f} {opt_prog:>10.4f} {opt_prog/cur_prog:>7.2f}x")

print("\n" + "="*75)
print("HOW TO ESTIMATE σ_f FROM YOUR DATA")
print("="*75)
print("""
From a generation's observed win-rates (e.g., best=0.55, avg=0.485, worst=0.32):

  σ_observed ≈ (best - worst) / 3.6    (for λ=14)
  σ_f = sqrt(σ_observed² - 0.25/n)

Example with n=100, best=0.55, worst=0.32:
  σ_observed ≈ 0.23 / 3.6 ≈ 0.064
  σ_f = sqrt(0.064² - 0.25/100) = sqrt(0.0041 - 0.0025) ≈ 0.040

With σ_f≈0.04, the optimal allocation for B=1400 is shown above.
Note: σ_f changes during optimization — it shrinks as CMA-ES converges.
Early generations have large σ_f (less noise-sensitive), late ones have small σ_f.
""")
