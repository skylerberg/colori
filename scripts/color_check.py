#!/usr/bin/env python3
"""Score the Colori palette on the two axes we care about.

Separation: how hard is it to confuse two of the twelve swatches? Measured as
CIEDE2000 between every pair, for normal vision and for the three dichromacies.
The bottleneck is the *worst* pair, not the average.

Name fidelity: how close does each swatch sit to the canonical color its name
promises? Measured as CIEDE2000 to a reference hex, plus a confusion check --
if the reference for "Amber" is closer to our Yellow than to our Amber, the
name is lying.

Run bare for the current palette, or pass overrides to score a proposal:

    python3 scripts/color_check.py --set Amber=#FFBF00 --set Green=#01A200
"""

import argparse
import importlib.util
import math
import sys
from pathlib import Path

WHEEL_ORDER = ['Red', 'Vermilion', 'Orange', 'Amber', 'Yellow', 'Chartreuse',
               'Green', 'Teal', 'Blue', 'Indigo', 'Purple', 'Magenta']

# Canonical hex per name, with the source we are deferring to. Several of these
# names have no single ground truth -- see ALT_REFERENCE for the rival reading.
REFERENCE = {
    'Red':        ('#FF0000', 'CSS red'),
    'Vermilion':  ('#E34234', 'Wikipedia vermilion'),
    'Orange':     ('#FFA500', 'CSS orange'),
    'Amber':      ('#FFBF00', 'Wikipedia amber'),
    'Yellow':     ('#FFFF00', 'CSS yellow'),
    'Chartreuse': ('#7FFF00', 'CSS chartreuse'),
    'Green':      ('#00FF00', 'CSS lime / additive green primary'),
    'Teal':       ('#008080', 'CSS teal'),
    'Blue':       ('#0000FF', 'CSS blue'),
    'Indigo':     ('#4B0082', 'CSS indigo'),
    'Purple':     ('#800080', 'CSS purple'),
    'Magenta':    ('#FF00FF', 'CSS magenta'),
}

# Where the name is genuinely contested, the other defensible reference.
ALT_REFERENCE = {
    'Green':   ('#008000', 'CSS green (the darker, subtractive reading)'),
    'Indigo':  ('#3F00FF', 'electric indigo (the blue-violet wheel slot)'),
    'Purple':  ('#7F00FF', 'violet (the blue-leaning wheel slot)'),
    'Red':     ('#E32636', 'alizarin / pigment red'),
}


def load_palette():
    """Pull the live palette out of color_wheel.py so there is one source of truth.

    Wheel geometry fixes the mapping: primaries sit on the three spokes, petals
    are the secondaries between them, ring sectors are the six tertiaries.
    """
    path = Path(__file__).with_name('color_wheel.py')
    spec = importlib.util.spec_from_file_location('color_wheel', path)
    mod = importlib.util.module_from_spec(spec)
    sys.argv = sys.argv[:1]  # color_wheel parses args only under __main__, but be safe
    spec.loader.exec_module(mod)
    return dict(zip(['Red', 'Yellow', 'Blue'], mod.PRIMARY)) | \
        dict(zip(['Orange', 'Green', 'Purple'], mod.PETAL)) | \
        dict(zip(['Vermilion', 'Amber', 'Chartreuse', 'Teal', 'Indigo', 'Magenta'], mod.RING))


def srgb(hex_str):
    h = hex_str.lstrip('#')
    return tuple(int(h[i:i + 2], 16) / 255 for i in (0, 2, 4))


def to_linear(c):
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def linear_rgb(hex_str):
    return tuple(to_linear(c) for c in srgb(hex_str))


def lab(rgb_lin):
    r, g, b = rgb_lin
    x = 0.4124564 * r + 0.3575761 * g + 0.1804375 * b
    y = 0.2126729 * r + 0.7151522 * g + 0.0721750 * b
    z = 0.0193339 * r + 0.1191920 * g + 0.9503041 * b
    xn, yn, zn = 0.95047, 1.0, 1.08883

    def f(t):
        return t ** (1 / 3) if t > 216 / 24389 else (841 / 108) * t + 4 / 29

    fx, fy, fz = f(x / xn), f(y / yn), f(z / zn)
    return (116 * fy - 16, 500 * (fx - fy), 200 * (fy - fz))


def oklch(rgb_lin):
    r, g, b = rgb_lin
    l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b
    m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b
    s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b
    l_, m_, s_ = (v ** (1 / 3) if v > 0 else -((-v) ** (1 / 3)) for v in (l, m, s))
    L = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_
    a = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_
    bb = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_
    return L, math.hypot(a, bb), math.degrees(math.atan2(bb, a)) % 360


def ciede2000(lab1, lab2):
    L1, a1, b1 = lab1
    L2, a2, b2 = lab2
    C1, C2 = math.hypot(a1, b1), math.hypot(a2, b2)
    Cbar = (C1 + C2) / 2
    G = 0.5 * (1 - math.sqrt(Cbar ** 7 / (Cbar ** 7 + 25 ** 7))) if Cbar else 0.5
    a1p, a2p = (1 + G) * a1, (1 + G) * a2
    C1p, C2p = math.hypot(a1p, b1), math.hypot(a2p, b2)
    h1p = math.degrees(math.atan2(b1, a1p)) % 360 if (a1p or b1) else 0.0
    h2p = math.degrees(math.atan2(b2, a2p)) % 360 if (a2p or b2) else 0.0

    dLp = L2 - L1
    dCp = C2p - C1p
    if C1p * C2p == 0:
        dhp = 0.0
    elif abs(h2p - h1p) <= 180:
        dhp = h2p - h1p
    elif h2p - h1p > 180:
        dhp = h2p - h1p - 360
    else:
        dhp = h2p - h1p + 360
    dHp = 2 * math.sqrt(C1p * C2p) * math.sin(math.radians(dhp) / 2)

    Lbp = (L1 + L2) / 2
    Cbp = (C1p + C2p) / 2
    if C1p * C2p == 0:
        hbp = h1p + h2p
    elif abs(h1p - h2p) <= 180:
        hbp = (h1p + h2p) / 2
    elif h1p + h2p < 360:
        hbp = (h1p + h2p + 360) / 2
    else:
        hbp = (h1p + h2p - 360) / 2

    T = (1 - 0.17 * math.cos(math.radians(hbp - 30))
         + 0.24 * math.cos(math.radians(2 * hbp))
         + 0.32 * math.cos(math.radians(3 * hbp + 6))
         - 0.20 * math.cos(math.radians(4 * hbp - 63)))
    dtheta = 30 * math.exp(-(((hbp - 275) / 25) ** 2))
    RC = 2 * math.sqrt(Cbp ** 7 / (Cbp ** 7 + 25 ** 7)) if Cbp else 0.0
    SL = 1 + 0.015 * (Lbp - 50) ** 2 / math.sqrt(20 + (Lbp - 50) ** 2)
    SC = 1 + 0.045 * Cbp
    SH = 1 + 0.015 * Cbp * T
    RT = -math.sin(math.radians(2 * dtheta)) * RC

    dL, dC, dH = dLp / SL, dCp / SC, dHp / SH
    return math.sqrt(dL * dL + dC * dC + dH * dH + RT * dC * dH)


# Viénot, Brettel & Mollon (1999): project onto the dichromat's reduced plane in
# Smith-Pokorny LMS. Applied in linear light, unlike the common JS ports.
LMS_FROM_RGB = ((17.8824, 43.5161, 4.11935),
                (3.45565, 27.1554, 3.86714),
                (0.0299566, 0.184309, 1.46709))
RGB_FROM_LMS = ((0.0809444479, -0.130504409, 0.116721066),
                (-0.0102485335, 0.0540193266, -0.113614708),
                (-0.000365296938, -0.00412161469, 0.693511405))
DICHROMAT = {
    'protanopia': ((0, 2.02344, -2.52581), (0, 1, 0), (0, 0, 1)),
    'deuteranopia': ((1, 0, 0), (0.494207, 0, 1.24827), (0, 0, 1)),
    'tritanopia': ((1, 0, 0), (0, 1, 0), (-0.395913, 0.801109, 0)),
}


def matmul(m, v):
    return tuple(sum(row[i] * v[i] for i in range(3)) for row in m)


def simulate(rgb_lin, kind):
    lms = matmul(LMS_FROM_RGB, rgb_lin)
    out = matmul(RGB_FROM_LMS, matmul(DICHROMAT[kind], lms))
    return tuple(min(1.0, max(0.0, c)) for c in out)


def pairs(palette, transform=lambda v: v):
    labs = {n: lab(transform(linear_rgb(h))) for n, h in palette.items()}
    names = list(palette)
    return sorted(((ciede2000(labs[a], labs[b]), a, b)
                   for i, a in enumerate(names) for b in names[i + 1:]))


def neighbor_gaps(palette, transform=lambda v: v):
    labs = {n: lab(transform(linear_rgb(h))) for n, h in palette.items()}
    n = len(WHEEL_ORDER)
    return [(WHEEL_ORDER[i], WHEEL_ORDER[(i + 1) % n],
             ciede2000(labs[WHEEL_ORDER[i]], labs[WHEEL_ORDER[(i + 1) % n]]))
            for i in range(n)]


def fidelity(palette, name, ref_hex):
    return ciede2000(lab(linear_rgb(palette[name])), lab(linear_rgb(ref_hex)))


def nearest_to_reference(palette, name, ref_hex):
    ref = lab(linear_rgb(ref_hex))
    return min(palette, key=lambda n: ciede2000(lab(linear_rgb(palette[n])), ref))


def report(palette, title):
    print(f"\n{'=' * 72}\n{title}\n{'=' * 72}")

    print("\nName fidelity  (dE00 to the canonical color for that name)")
    print(f"  {'color':<11} {'swatch':<9} {'ref':<9} {'dE00':>6}  nearest palette entry to ref")
    total = 0.0
    for n in WHEEL_ORDER:
        ref_hex, _ = REFERENCE[n]
        d = fidelity(palette, n, ref_hex)
        total += d
        near = nearest_to_reference(palette, n, ref_hex)
        flag = '' if near == n else f'  <-- {near}, not {n}'
        print(f"  {n:<11} {palette[n]:<9} {ref_hex:<9} {d:6.1f}  {near}{flag}")
    print(f"  {'':<11} {'':<9} {'mean':<9} {total / 12:6.1f}")

    alts = [(n, h, s) for n, (h, s) in ALT_REFERENCE.items()]
    print("\n  against the rival reference where the name is contested:")
    for n, h, src in alts:
        print(f"    {n:<11} vs {h} ({src}): {fidelity(palette, n, h):5.1f}")

    ps = pairs(palette)
    print(f"\nSeparation  (dE00 over all 66 pairs)")
    print(f"  worst pair {ps[0][0]:6.1f}   {ps[0][1]} / {ps[0][2]}")
    print(f"  median     {ps[len(ps) // 2][0]:6.1f}")
    print("  five tightest pairs:")
    for d, a, b in ps[:5]:
        print(f"    {d:6.1f}  {a} / {b}")

    print("\n  adjacent on the wheel (where confusion actually costs you):")
    gaps = neighbor_gaps(palette)
    for a, b, d in gaps:
        bar = '#' * int(d / 2)
        print(f"    {a:>11} - {b:<11} {d:6.1f}  {bar}")
    print(f"    {'worst neighbor gap':>23} {min(g[2] for g in gaps):6.1f}")

    print("\nColor vision deficiency  (dE00, dichromat simulation)")
    for kind in DICHROMAT:
        cps = pairs(palette, lambda v, k=kind: simulate(v, k))
        collapsed = [p for p in cps if p[0] < 10]
        print(f"  {kind:<13} worst {cps[0][0]:5.1f}  ({cps[0][1]} / {cps[0][2]}),"
              f" {len(collapsed)} pairs under dE00 10")

    print("\nHue placement  (OKLCh -- a clean wheel wants ~30 deg per step)")
    hues = {n: oklch(linear_rgb(palette[n])) for n in WHEEL_ORDER}
    for i, n in enumerate(WHEEL_ORDER):
        nxt = WHEEL_ORDER[(i + 1) % 12]
        step = (hues[nxt][2] - hues[n][2]) % 360
        L, C, h = hues[n]
        print(f"  {n:<11} h {h:6.1f}  L {L:.2f}  C {C:.3f}   step to {nxt:<11} {step:5.1f}"
              f"  {'(crowded)' if step < 20 else '(stretched)' if step > 45 else ''}")


def compare(base, prop, changed):
    print(f"\n{'=' * 72}\nNET EFFECT\n{'=' * 72}")
    print(f"\n  {'metric':<38} {'current':>9} {'proposed':>9} {'delta':>8}")

    def line(label, a, b, higher_is_better=True):
        d = b - a
        mark = '' if abs(d) < 0.05 else (' better' if (d > 0) == higher_is_better else ' worse')
        print(f"  {label:<38} {a:9.1f} {b:9.1f} {d:+8.1f}{mark}")

    line('worst pair (all 66)', pairs(base)[0][0], pairs(prop)[0][0])
    line('median pair', pairs(base)[len(pairs(base)) // 2][0], pairs(prop)[len(pairs(prop)) // 2][0])
    line('worst wheel-neighbor gap',
         min(g[2] for g in neighbor_gaps(base)), min(g[2] for g in neighbor_gaps(prop)))
    for kind in DICHROMAT:
        line(f'worst pair, {kind}',
             pairs(base, lambda v, k=kind: simulate(v, k))[0][0],
             pairs(prop, lambda v, k=kind: simulate(v, k))[0][0])
    line('mean name fidelity (dE00 to ref)',
         sum(fidelity(base, n, REFERENCE[n][0]) for n in WHEEL_ORDER) / 12,
         sum(fidelity(prop, n, REFERENCE[n][0]) for n in WHEEL_ORDER) / 12,
         higher_is_better=False)

    print("\n  per changed color:")
    for n in changed:
        ref_hex, src = REFERENCE[n]
        print(f"    {n} {base[n]} -> {prop[n]}")
        print(f"      name fidelity vs {ref_hex} ({src}): "
              f"{fidelity(base, n, ref_hex):.1f} -> {fidelity(prop, n, ref_hex):.1f}")
        if n in ALT_REFERENCE:
            alt_hex, alt_src = ALT_REFERENCE[n]
            print(f"      name fidelity vs {alt_hex} ({alt_src}): "
                  f"{fidelity(base, n, alt_hex):.1f} -> {fidelity(prop, n, alt_hex):.1f}")
        i = WHEEL_ORDER.index(n)
        for j in ((i - 1) % 12, (i + 1) % 12):
            o = WHEEL_ORDER[j]
            a = ciede2000(lab(linear_rgb(base[n])), lab(linear_rgb(base[o])))
            b = ciede2000(lab(linear_rgb(prop[n])), lab(linear_rgb(prop[o])))
            print(f"      gap to {o:<11} {a:5.1f} -> {b:5.1f} ({b - a:+.1f})")
        Lb, Cb, hb = oklch(linear_rgb(base[n]))
        Lp, Cp, hp = oklch(linear_rgb(prop[n]))
        print(f"      OKLCh  L {Lb:.2f} -> {Lp:.2f}   C {Cb:.3f} -> {Cp:.3f}   "
              f"h {hb:.1f} -> {hp:.1f}")


def self_test():
    """Sharma's CIEDE2000 reference pairs, plus a known sRGB->Lab anchor."""
    cases = [
        ((50, 2.6772, -79.7751), (50, 0, -82.7485), 2.0425),
        ((50, 3.1571, -77.2803), (50, 0, -82.7485), 2.8615),
        ((50, 2.8361, -74.0200), (50, 0, -82.7485), 3.4412),
        ((50, -1.3802, -84.2814), (50, 0, -82.7485), 1.0000),
        ((50, 0, 0), (50, -1, 2), 2.3669),
        ((50, 2.49, -0.001), (50, -2.49, 0.0009), 7.1792),
        ((60.2574, -34.0099, 36.2677), (60.4626, -34.1751, 39.4387), 1.2644),
        ((63.0109, -31.0961, -5.8663), (62.8187, -29.7946, -4.0864), 1.2630),
        ((2.0776, 0.0795, -1.1350), (0.9033, -0.0636, -0.5514), 0.9082),
    ]
    ok = True
    for a, b, want in cases:
        got = ciede2000(a, b)
        if abs(got - want) > 0.0002:
            print(f"FAIL ciede2000{a}{b}: got {got:.4f} want {want:.4f}")
            ok = False
    for hex_str, want in (('#FFFFFF', (100.0, 0.0, 0.0)), ('#FF0000', (53.2408, 80.0925, 67.2032))):
        got = lab(linear_rgb(hex_str))
        if any(abs(g - w) > 0.01 for g, w in zip(got, want)):
            print(f"FAIL lab({hex_str}): got {got} want {want}")
            ok = False
    print('self-test: ' + ('all reference values match' if ok else 'FAILURES ABOVE'))
    return ok


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--set', action='append', default=[], metavar='Name=#HEX',
                    help='override a color and report the before/after effect')
    ap.add_argument('--self-test', action='store_true', help='verify the color math and exit')
    args = ap.parse_args()

    if args.self_test:
        sys.exit(0 if self_test() else 1)

    base = load_palette()
    missing = [n for n in WHEEL_ORDER if n not in base]
    if missing:
        sys.exit(f'palette is missing {missing}')

    prop = dict(base)
    changed = []
    for spec in args.set:
        name, _, hex_str = spec.partition('=')
        name = name.strip().capitalize()
        if name not in base:
            sys.exit(f'unknown color {name!r}; expected one of {WHEEL_ORDER}')
        prop[name] = '#' + hex_str.strip().lstrip('#').upper()
        changed.append(name)

    if not changed:
        report(base, 'CURRENT PALETTE')
        return
    report(base, 'CURRENT PALETTE')
    report(prop, 'PROPOSED PALETTE  (' + ', '.join(f'{n} {base[n]}->{prop[n]}' for n in changed) + ')')
    compare(base, prop, changed)


if __name__ == '__main__':
    main()
