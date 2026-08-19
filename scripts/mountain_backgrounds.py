#!/usr/bin/env python3
"""Two-color card backgrounds: a sky color above a mountain range below.

Same machinery as the solid backgrounds -- one painted texture, recolored so
each area's dominant value lands exactly on its official hex -- but the card is
split by a skyline instead of being one field. The texture runs continuously
through the ridge rather than restarting, so the card reads as a single painting
in two colors rather than two images stacked.

The skyline is modeled on the Dolomite front as seen from the Venetian plain:
flat-topped massifs with vertical walls, sharp isolated pyramids, and clusters
of towers, rather than the rounded hills a generic noise profile produces. It is
an artistic profile with massifs named for the peaks that dominate that view --
not a surveyed elevation.

    python3 scripts/mountain_backgrounds.py --preview   # skyline only, no color
    python3 scripts/mountain_backgrounds.py             # all 42 cards
"""

import argparse
import importlib.util
import sys
from pathlib import Path

import numpy as np
from PIL import Image

REPO = Path(__file__).resolve().parent.parent
SCRIPTS = REPO / 'scripts'
SOURCE = REPO / 'card-backgrounds' / '_source-texture.png'
OUT_DIR = REPO / 'card-backgrounds' / 'mountain'

TERTIARY = ['Vermilion', 'Amber', 'Chartreuse', 'Teal', 'Indigo', 'Magenta']
PRIMARY = ['Red', 'Yellow', 'Blue']
SECONDARY = ['Orange', 'Green', 'Purple']

# Skyline geometry in fractions of the *trim* height, so the ridge lands at the
# same place on the cut card regardless of how much bleed is around it. Matched
# to the reference card: ridge averaging ~45% down, peaks to ~33%, valleys ~57%.
BASE = 0.57
PEAK_FLOOR = 0.33
BODY = 0.495  # saddle level joining the massifs into one range
TILT = 0.05   # skyline drifts down to the right, as in the reference card
FEATHER_PX = 5.0  # softness of the ridge edge, in pixels

# (name, center x, half-width, peak height, silhouette)
MASSIFS = [
    ('Civetta',   0.10, 0.24, 0.41, 'wall'),
    ('Pelmo',     0.27, 0.15, 0.38, 'mesa'),
    ('Marmolada', 0.44, 0.26, 0.31, 'dome'),
    ('Antelao',   0.63, 0.16, 0.36, 'pyramid'),
    ('Cadini',    0.81, 0.17, 0.40, 'towers'),
    ('Marmarole', 1.00, 0.20, 0.46, 'wall'),
]


def load(name):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / f'{name}.py')
    mod = importlib.util.module_from_spec(spec)
    sys.argv = sys.argv[:1]
    spec.loader.exec_module(mod)
    return mod


def _massif(x, center, hw, peak, base, kind):
    """One massif's silhouette.

    The exponent on the flank is what makes these read as Dolomites rather than
    as generic hills: above 1 the profile holds its height across the width and
    then plunges, which is the near-vertical limestone wall under a blocky top.
    Below 1 you get the conical scree-slope mountain of a child's drawing.
    """
    t = (x - center) / hw
    a = np.abs(t)
    span = a <= 1
    drop = base - peak

    if kind == 'pyramid':                       # sharp apex, still steep-sided
        prof = peak + drop * a ** 1.15
    elif kind == 'dome':                        # rounded crest over a big wall
        prof = peak + drop * a ** 1.7
    elif kind == 'mesa':                        # flat throne top, sheer sides
        flat = 0.40
        prof = np.where(a <= flat, peak,
                        peak + drop * np.clip((a - flat) / (1 - flat), 0, 1) ** 1.6)
    elif kind == 'wall':                        # long serrated rampart
        prof = (peak + 0.22 * drop * t
                + 0.035 * drop * np.sin(x * 70 + 1.7)
                + drop * a ** 3)
    elif kind == 'towers':                      # blunt-topped towers, narrow gaps
        prof = np.full_like(x, base)
        for off, hgt, wid in ((-0.60, 0.92, 0.44), (-0.02, 1.0, 0.42),
                              (0.56, 0.86, 0.40)):
            d = np.clip(np.abs(t - off) / wid, 0, 1)
            tip = peak + drop * (1 - hgt)
            prof = np.minimum(prof, tip + (base - tip) * d ** 1.9)
    else:
        raise ValueError(kind)

    return np.where(span, prof, np.inf)


def fbm(x, seed, octaves=4, amp=0.005):
    """Fractal wobble so the skyline reads as rock, not as a plotted function."""
    rng = np.random.default_rng(seed)
    out = np.zeros_like(x)
    for k in range(octaves):
        freq = 6 * 2 ** k
        out += (amp / 2 ** k) * np.sin(2 * np.pi * freq * x + rng.uniform(0, 2 * np.pi))
    return out


def skyline(x, seed=11, relief=1.0):
    """Ridge height per column, as a fraction of trim height (smaller = higher).

    x is in trim coordinates -- 0 and 1 are the cut edges, and the bleed runs
    slightly outside that range -- so the composition is framed by what survives
    the trim rather than by the uncut sheet.
    """
    ridge = np.full_like(x, BASE)
    for _, c, hw, peak, kind in MASSIFS:
        # relief scales every summit toward the saddle level, trading drama for
        # the softer, lower range of the reference card without changing shape
        ridge = np.minimum(ridge, _massif(x, c, hw, BODY - (BODY - peak) * relief,
                                          BASE, kind))
    # A continuous body under the whole range, so the gaps between massifs are
    # saddles rather than gaps down to the plain. Without it the skyline reads
    # as a row of detached humps instead of one range.
    ridge = np.minimum(ridge, BODY + 0.022 * np.sin(x * 4.2 + 0.6))
    ridge = ridge + TILT * (x - 0.5)          # range settles toward the plain
    return np.clip(ridge + fbm(x, seed) * relief, PEAK_FLOOR, BASE + 0.04)


def ridge_masks(width, height, bleed_px, trim_w, trim_h, seed=11,
                relief=1.0, feather=FEATHER_PX):
    """Antialiased sky/mountain coverage, ridge placed in trim coordinates."""
    ridge = skyline((np.arange(width) - bleed_px) / trim_w, seed, relief)
    rows = np.arange(height)[:, None]
    y_trim = (rows - bleed_px) / trim_h
    # Feathered a few pixels rather than cut hard: a dried watercolor edge is
    # crisp but not vector-sharp, and a 1px step reads as a digital mask.
    step = feather / trim_h
    mountain = np.clip((y_trim - ridge[None, :]) / step + 0.5, 0, 1)
    return 1 - mountain, mountain


def pairs():
    out = [(bg, mt) for bg in TERTIARY for mt in PRIMARY + SECONDARY]
    # Primary-on-primary in both directions, since which reads as sky and which
    # as range is a composition call rather than something the color pair fixes.
    out += [(a, b) for a in PRIMARY for b in PRIMARY if a != b]
    return out


def render_pair(cb, L, C, h, anchor, spread, palette, bg, mt, masks):
    sky_cov, mtn_cov = masks
    out = np.zeros(L.shape + (3,))
    report = []
    for name, cov in ((bg, sky_cov), (mt, mtn_cov)):
        sel = cov > 0
        aim = cb.solve_aim(L, C, h, anchor, palette[name], spread=spread)
        tinted = cb.render(L[sel], C[sel], h[sel], anchor, aim, spread)
        out[sel] += tinted * cov[sel][:, None]

        solid = cov[sel] > 0.99
        if solid.sum() > 50:
            sL, sC, sh = cb.to_lch(cb.linear_to_oklab(cb.srgb_to_linear(tinted[solid])))
            plateau = cb.lch_to_hex(*cb.source_anchor(sL, sC, sh))
            report.append(cb.ciede2000(cb.cielab(plateau), cb.cielab(palette[name])))
    return np.clip(out, 0, 1), report


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--preview', action='store_true',
                    help='write just the skyline silhouette, to judge the shape')
    ap.add_argument('--seed', type=int, default=11, help='fractal wobble seed')
    ap.add_argument('--relief', type=float, default=0.8,
                    help='summit height above the saddles; 1.0 is full drama')
    ap.add_argument('--feather', type=float, default=FEATHER_PX,
                    help='ridge edge softness in pixels')
    ap.add_argument('--suffix', default='', help='tag appended to output filenames')
    ap.add_argument('--only', nargs=2, metavar=('BG', 'MOUNTAIN'),
                    help='render a single pair')
    args = ap.parse_args()

    cb = load('card_backgrounds')
    W, H = cb.WIDTH, cb.HEIGHT
    bleed_px = cb.BLEED_MM / 25.4 * cb.PPI
    trim_w = cb.TRIM_MM[0] / 25.4 * cb.PPI
    trim_h = cb.TRIM_MM[1] / 25.4 * cb.PPI
    masks = ridge_masks(W, H, bleed_px, trim_w, trim_h, args.seed,
                        args.relief, args.feather)

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    if args.preview:
        sky, mtn = masks
        img = (np.dstack([1 - mtn * 0.85] * 3) * 255).astype(np.uint8)
        img[int(bleed_px), :] = [255, 0, 0]
        img[int(bleed_px + trim_h), :] = [255, 0, 0]
        Image.fromarray(img).save(OUT_DIR / '_skyline-preview.png')
        r = skyline(np.linspace(0, 1, 400), args.seed, args.relief)
        print(f'peak {r.min() * 100:.0f}%  valley {r.max() * 100:.0f}%  '
              f'mean {r.mean() * 100:.0f}% down the trim; reference card was '
              f'peak 32% mean ~45%')
        print(f'wrote {(OUT_DIR / "_skyline-preview.png").relative_to(REPO)} '
              '(red lines mark the trim edges)')
        return

    if not SOURCE.exists():
        sys.exit(f'no source texture at {SOURCE.relative_to(REPO)}')
    palette = load('color_check').load_palette()
    src = np.asarray(cb.cover_resize(Image.open(SOURCE).convert('RGB'), W, H))
    src = src.astype(np.float64) / 255
    L, C, h = cb.to_lch(cb.linear_to_oklab(cb.srgb_to_linear(src)))
    anchor = cb.source_anchor(L, C, h)
    spread = cb.spread_of(L, anchor)

    todo = [tuple(args.only)] if args.only else pairs()
    worst, tiles = 0.0, []
    for bg, mt in todo:
        for n in (bg, mt):
            if n not in palette:
                sys.exit(f'unknown color {n!r}')
        rgb, report = render_pair(cb, L, C, h, anchor, spread, palette, bg, mt, masks)
        path = OUT_DIR / f'bg-{bg.lower()}_mtn-{mt.lower()}{args.suffix}.png'
        img = Image.fromarray((rgb * 255 + 0.5).astype(np.uint8))
        img.save(path, dpi=(cb.PPI, cb.PPI))
        tiles.append(img)
        worst = max(worst, max(report) if report else 0)
        print(f'{bg:<11} sky / {mt:<9} range   worst dE {max(report):4.2f}   '
              f'{path.name}')

    if len(todo) > 1:
        cols = 7
        tw, th = W // 5, H // 5
        rows = (len(tiles) + cols - 1) // cols
        sheet = Image.new('RGB', (cols * (tw + 6) + 6, rows * (th + 6) + 6), 'white')
        for i, t in enumerate(tiles):
            sheet.paste(t.resize((tw, th), Image.LANCZOS),
                        (6 + (i % cols) * (tw + 6), 6 + (i // cols) * (th + 6)))
        sheet.save(OUT_DIR / '_contact-sheet.png')
        print(f'contact sheet: {(OUT_DIR / "_contact-sheet.png").relative_to(REPO)}')

    print(f'\n{len(todo)} cards at {W}x{H}px '
          f'({cb.TRIM_MM[0]}x{cb.TRIM_MM[1]}mm trim + {cb.BLEED_MM}mm bleed), '
          f'worst plateau dE {worst:.2f}')


if __name__ == '__main__':
    main()
