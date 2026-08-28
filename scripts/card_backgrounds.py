#!/usr/bin/env python3
"""Build the twelve card-background washes from a single watercolor texture.

One source painting is generated once (Gemini, same model the colori-art
pipeline uses) and then recolored once per wheel color -- three primaries, three
secondaries, six tertiaries. Every card therefore carries the identical texture
-- same blooms, same dried wash edges, same paper grain -- and differs only in
hue, which is what makes the color legible as an identifier next to the printed
components.

The recolor runs in OKLCh so the transform is perceptual rather than a naive
RGB tint. Each output is anchored so its dominant value lands exactly on the
official hex; texture contrast is normalized to a fixed OKLab-L swing measured
off the existing sell card; and colors that fall outside sRGB are pulled back by
reducing chroma at constant lightness and hue, never by clipping RGB, which
would twist the hue away from official.

    export GOOGLE_API_KEY=...
    python3 scripts/card_backgrounds.py --generate    # one API call, cached
    python3 scripts/card_backgrounds.py               # recolor + verify
"""

import argparse
import importlib.util
import math
import os
import sys
from pathlib import Path

import numpy as np
from PIL import Image

REPO = Path(__file__).resolve().parent.parent
OUT_DIR = REPO / 'card-backgrounds'
SOURCE = OUT_DIR / '_source-texture.png'

# Card trim in mm, plus 3mm bleed on every edge. 44x67 matches the existing
# sell-card compositions (519x791px), not the 44x68 European mini nominal.
TRIM_MM = (44, 67)
BLEED_MM = 3
PPI = 300
WIDTH = round((TRIM_MM[0] + 2 * BLEED_MM) / 25.4 * PPI)   # 591
HEIGHT = round((TRIM_MM[1] + 2 * BLEED_MM) / 25.4 * PPI)  # 874

# Wheel order rather than primary/secondary/tertiary order, so each row of the
# contact sheet is half the wheel and neighboring hues sit next to each other.
CARDS = ['Red', 'Vermilion', 'Orange', 'Amber', 'Yellow', 'Chartreuse',
         'Green', 'Teal', 'Blue', 'Indigo', 'Purple', 'Magenta']
SHEET_COLS = 6

# Texture contrast, in OKLab L, measured p5..p95 off the clean wash field of
# the existing sell card. Split evenly above and below the anchor except where
# a light target (yellow) has no headroom, in which case the swing shifts down.
SWING = 0.12
HUE_DAMP = 0.5       # how much of the source's hue drift to carry over
CHROMA_FLOOR = 0.35  # thinnest wash keeps this fraction of the target's chroma
KNEE = 0.35          # how far past the p5..p95 band the tails may still travel
MODEL = 'nano-banana-pro-preview'

PROMPT = (
    "A flat overhead scan of an abstract watercolor wash. This is a MACRO CROP from "
    "the middle of a much larger painted sheet -- the paint runs past all four edges "
    "of the frame. NO paper border, NO white margin, NO deckle edge, NO frame, "
    "NO subject, NO text, NO signature.\n"
    "- Successive translucent coats of the same muted warm gray-sepia pigment, each "
    "laid slightly over the last, building up subtle tonal steps\n"
    "- Crisp dried edges where one coat laps over another -- the hard edge a "
    "watercolor layer leaves as it dries\n"
    "- Heavy pigment granulation and mottling settling into the paper tooth; the "
    "surface is uneven and weathered, like aged fresco pigment on plaster\n"
    "- Irregular organic wash boundaries. NO geometric petal or leaf shapes, "
    "NO symmetry, NO radial or centered composition, NO clean vector-like curves\n"
    "- The pigment is CONTINUOUS across the entire frame: washes overlap and merge, "
    "leaving NO white gaps, NO unpainted channels or streaks between them\n"
    "- Calm and restful, not busy. NO cauliflower backruns, NO blotches, "
    "NO repeating circular marks, NO speckled splatter\n"
    "- Mid-tone overall: gentle drift from thin light wash to slightly deeper pooled "
    "areas, but NO pure white and NO black anywhere\n"
    "- No shadows, no vignette, no lighting effects, no drop shadow, no 3D depth"
)

# Trim this fraction off each edge of whatever the model returns. Image models
# habitually paint a sheet of paper with a white margin instead of a full-bleed
# field; the card has no room for that, so the border gets cut off regardless.
EDGE_INSET = 0.07


_SIBLINGS = {}


def sibling(name):
    """Import a sibling script by path, once per process."""
    if name not in _SIBLINGS:
        spec = importlib.util.spec_from_file_location(name, REPO / 'scripts' / f'{name}.py')
        mod = importlib.util.module_from_spec(spec)
        sys.argv = sys.argv[:1]
        spec.loader.exec_module(mod)
        _SIBLINGS[name] = mod
    return _SIBLINGS[name]


def load_palette():
    return sibling('color_check').load_palette()


# ---------------------------------------------------------------- color space

def srgb_to_linear(a):
    return np.where(a <= 0.04045, a / 12.92, ((a + 0.055) / 1.055) ** 2.4)


def linear_to_srgb(a):
    return np.where(a <= 0.0031308, a * 12.92, 1.055 * np.abs(a) ** (1 / 2.4) - 0.055)


def linear_to_oklab(rgb):
    r, g, b = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b
    m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b
    s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b
    l_, m_, s_ = np.cbrt(l), np.cbrt(m), np.cbrt(s)
    return np.stack([
        0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
        1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
        0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
    ], axis=-1)


def oklab_to_linear(lab):
    L, a, b = lab[..., 0], lab[..., 1], lab[..., 2]
    l_ = L + 0.3963377774 * a + 0.2158037573 * b
    m_ = L - 0.1055613458 * a - 0.0638541728 * b
    s_ = L - 0.0894841775 * a - 1.2914855480 * b
    l, m, s = l_ ** 3, m_ ** 3, s_ ** 3
    return np.stack([
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
    ], axis=-1)


def to_lch(lab):
    L = lab[..., 0]
    C = np.hypot(lab[..., 1], lab[..., 2])
    h = np.degrees(np.arctan2(lab[..., 2], lab[..., 1])) % 360
    return L, C, h


def from_lch(L, C, h):
    r = np.radians(h)
    return np.stack([L, C * np.cos(r), C * np.sin(r)], axis=-1)


def hex_to_lch(hex_str):
    v = np.array([[int(hex_str.lstrip('#')[i:i + 2], 16) / 255 for i in (0, 2, 4)]])
    L, C, h = to_lch(linear_to_oklab(srgb_to_linear(v)))
    return float(L[0]), float(C[0]), float(h[0])


def lch_to_hex(L, C, h):
    lin = oklab_to_linear(from_lch(np.array([L]), np.array([C]), np.array([h])))
    v = np.clip(linear_to_srgb(lin), 0, 1)[0]
    return '#%02X%02X%02X' % tuple(int(round(c * 255)) for c in v)


def fit_to_gamut(L, C, h, steps=18):
    """Shrink chroma per pixel until the color fits in sRGB, holding L and h."""
    lo = np.zeros_like(C)
    hi = np.ones_like(C)
    for _ in range(steps):
        mid = (lo + hi) / 2
        lin = oklab_to_linear(from_lch(L, C * mid, h))
        inside = np.all((lin >= -1e-4) & (lin <= 1 + 1e-4), axis=-1)
        lo = np.where(inside, mid, lo)
        hi = np.where(inside, hi, mid)
    return C * lo


# ---------------------------------------------------------------- source prep

def cover_resize(img, w, h):
    """Crop to the target aspect, then scale. Keeps the paper grain isotropic."""
    sw, sh = img.size
    scale = max(w / sw, h / sh)
    nw, nh = math.ceil(sw * scale), math.ceil(sh * scale)
    img = img.resize((nw, nh), Image.LANCZOS)
    left, top = (nw - w) // 2, (nh - h) // 2
    return img.crop((left, top, left + w, top + h))


def generate_source(dest=None, label=''):
    from google import genai
    from google.genai import types
    from io import BytesIO

    dest = dest or SOURCE
    key = os.environ.get('GOOGLE_API_KEY')
    if not key:
        sys.exit('GOOGLE_API_KEY is not set')
    client = genai.Client(api_key=key)
    print(f'requesting source texture{label} from {MODEL} ...')
    # 2:3 is the closest portrait ratio to the card's 591:874; asking for it
    # natively at 4K beats cropping a slice out of the default landscape frame
    # and upscaling, which throws away most of the painting and softens the grain.
    response = client.models.generate_content(
        model=MODEL,
        contents=[PROMPT],
        config=types.GenerateContentConfig(
            response_modalities=['IMAGE', 'TEXT'],
            image_config=types.ImageConfig(aspect_ratio='2:3', image_size='4K'),
        ),
    )
    for part in response.candidates[0].content.parts:
        if part.inline_data is not None and part.inline_data.data:
            img = Image.open(BytesIO(part.inline_data.data)).convert('RGB')
            print(f'  received {img.size[0]}x{img.size[1]}')
            w, h = img.size
            dx, dy = int(w * EDGE_INSET), int(h * EDGE_INSET)
            img = img.crop((dx, dy, w - dx, h - dy))
            print(f'  inset to {img.size[0]}x{img.size[1]} to drop any paper border')
            OUT_DIR.mkdir(exist_ok=True)
            # Keep the full-resolution crop: a later change to the trim size can
            # then be re-derived from it instead of costing another generation.
            raw = dest.with_name(dest.stem + '-raw.png')
            img.save(raw)
            cover_resize(img, WIDTH, HEIGHT).save(dest, dpi=(PPI, PPI))
            print(f'  wrote {dest.relative_to(REPO)} at {WIDTH}x{HEIGHT}'
                  f' (full-res kept as {raw.name})')
            return True
    sys.exit('no image came back from the model')


def synthetic_source(seed=3):
    """A procedural stand-in wash, for validating the pipeline without an API call.

    Good enough to prove the recolor math and to compare the six side by side.
    Not the deliverable -- the real source comes from --generate.
    """
    from PIL import ImageFilter
    rng = np.random.default_rng(seed)
    H, W = HEIGHT, WIDTH

    def blur(a, r):
        im = Image.fromarray(np.clip(a * 255, 0, 255).astype(np.uint8))
        return np.asarray(im.filter(ImageFilter.GaussianBlur(r))).astype(np.float64) / 255

    density = np.zeros((H, W))
    yy, xx = np.mgrid[0:H, 0:W]
    for _ in range(11):
        cy, cx = rng.uniform(-0.1, 1.1) * H, rng.uniform(-0.1, 1.1) * W
        ry, rx = rng.uniform(0.25, 0.7) * H, rng.uniform(0.3, 0.9) * W
        blob = np.exp(-(((yy - cy) / ry) ** 2 + ((xx - cx) / rx) ** 2) ** 1.4)
        blob = blur(blob * (0.9 + 0.2 * rng.random((H, W))), 9)
        gy, gx = np.gradient(blob)
        rim = np.hypot(gy, gx)
        density += blob * rng.uniform(0.10, 0.22) + rim * 14 * rng.uniform(0.3, 0.9)

    granulation = blur(rng.random((H, W)), 5) - 0.5
    grain = rng.normal(0, 1, (H, W))
    density = density / max(density.max(), 1e-6)
    value = 0.62 - 0.30 * density + 0.05 * granulation + 0.010 * grain
    value = np.clip(value, 0.05, 0.98)

    warm = 0.5 + 0.5 * blur(rng.random((H, W)), 40)          # slow hue drift
    rgb = np.stack([value * (1.00 + 0.05 * warm),
                    value * (0.93 - 0.02 * warm),
                    value * (0.84 - 0.06 * warm)], axis=-1)
    return np.clip(rgb, 0, 1)


def source_anchor(L, C, h):
    """The texture's plateau: the value the wash spends most of its area at."""
    bins = 512
    hist, edges = np.histogram(L, bins=bins, range=(0, 1))
    # Smooth so grain doesn't create a spurious peak, then interpolate the peak
    # parabolically -- bin-resolution alone is too coarse for the aim-point
    # correction to converge, and it stalls a couple of dE off target.
    k = np.exp(-np.linspace(-2, 2, 9) ** 2)
    sm = np.convolve(hist.astype(float), k / k.sum(), mode='same')
    peak = int(sm.argmax())
    L_mode = (edges[peak] + edges[peak + 1]) / 2
    if 0 < peak < bins - 1:
        y0, y1, y2 = sm[peak - 1], sm[peak], sm[peak + 1]
        denom = y0 - 2 * y1 + y2
        if denom != 0:
            L_mode += 0.5 * (y0 - y2) / denom / bins
    band = np.abs(L - L_mode) < 0.015
    if band.sum() < 100:
        band = np.ones_like(L, dtype=bool)
    C_mode = float(np.median(C[band]))
    ang = np.radians(h[band])
    h_mode = float(np.degrees(np.arctan2(np.sin(ang).mean(), np.cos(ang).mean())) % 360)
    return float(L_mode), C_mode, h_mode


def spread_of(L, anchor):
    """The source's reach above and below its plateau, used to scale the ramp."""
    L_mode = anchor[0]
    return (max(np.percentile(L, 95) - L_mode, 1e-6),
            max(L_mode - np.percentile(L, 5), 1e-6))


def _map(L, C, h, anchor, aim, spread=None):
    """Apply the ramp for one candidate anchor point, returning OKLCh channels.

    spread is passed in when the caller is recoloring a subset of a larger
    painting (the wheel recolors region by region): the ramp must come from the
    whole texture's statistics, or each region would be normalized against its
    own crop and the wash would step in brightness at every boundary.
    """
    L_mode, C_mode, h_mode = anchor
    aL, aC, ah = aim

    # Preserve total contrast, but never let a light target clip against white.
    hi = min(SWING / 2, (1 - aL) * 0.9)
    lo = SWING - hi
    p_hi, p_lo = spread if spread else spread_of(L, anchor)
    dL = L - L_mode

    # The p5..p95 band maps linearly, but the source's tails run to ~1.9x that
    # reach, so a straight linear ramp overshoots to nearly double the intended
    # swing and blows the highlights out to bare white. Past the band, ease off.
    t = np.where(dL > 0, dL / p_hi, dL / p_lo)
    a = np.abs(t)
    a = np.where(a <= 1, a, 1 + KNEE * np.tanh((a - 1) / KNEE))
    L_out = np.clip(aL + np.sign(t) * a * np.where(dL > 0, hi, lo), 0.02, 0.995)

    # Thin washes carry less pigment, so relative chroma is preserved -- but with
    # a floor, or near-neutral patches in the source (bare paper) would recolor to
    # gray blotches and break the card's read as a single identifying color.
    C_out = np.clip(aC * (C / max(C_mode, 1e-6)),
                    max(aC, 1e-6) * CHROMA_FLOOR, max(aC, 1e-6) * 2.0)

    dh = (h - h_mode + 180) % 360 - 180
    h_out = (ah + dh * HUE_DAMP) % 360

    return L_out, fit_to_gamut(L_out, C_out, h_out), h_out


def solve_aim(L, C, h, anchor, target_hex, iters=4, spread=None):
    """Find the aim point whose output plateau lands on target_hex.

    Gamut-fitting pulls high-chroma pixels inward, which drags the density peak
    off the nominal target for colors near the sRGB boundary (blue, purple). So
    the aim is corrected against the measured output plateau until the two agree,
    rather than trusting the nominal target to survive the transform.

    Converged on a subsample: the plateau is a distribution statistic, so it
    barely moves with pixel count, while the gamut bisection is far too expensive
    to run several times over a full-resolution image.
    """
    L_t, C_t, h_t = hex_to_lch(target_hex)
    aim = [L_t, C_t, h_t]
    step = max(1, int(np.sqrt(L.size / 250_000)))
    sL, sC, sh = L[::step, ::step], C[::step, ::step], h[::step, ::step]
    spread = spread or spread_of(L, anchor)
    for _ in range(iters):
        pL, pC, ph = source_anchor(*_map(sL, sC, sh, anchor, aim, spread))
        aim[0] += L_t - pL
        aim[1] += C_t - pC
        aim[2] += (h_t - ph + 180) % 360 - 180
    return aim


def render(L, C, h, anchor, aim, spread=None):
    """Apply a solved aim point and return sRGB in [0, 1]."""
    lin = oklab_to_linear(from_lch(*_map(L, C, h, anchor, aim, spread)))
    return np.clip(linear_to_srgb(np.clip(lin, 0, 1)), 0, 1)


def recolor(L, C, h, anchor, target_hex, iters=4):
    return render(L, C, h, anchor, solve_aim(L, C, h, anchor, target_hex, iters))


# ---------------------------------------------------------------- reporting

def ciede2000(lab1, lab2):
    return sibling('color_check').ciede2000(lab1, lab2)


def cielab(hex_str):
    cc = sibling('color_check')
    return cc.lab(cc.linear_rgb(hex_str))


def dominant_hex(rgb01):
    flat = (rgb01.reshape(-1, 3) * 255).astype(np.uint8) // 6 * 6
    vals, counts = np.unique(flat, axis=0, return_counts=True)
    return '#%02X%02X%02X' % tuple(int(v) for v in vals[counts.argmax()])


def mean_hex(rgb01):
    m = rgb01.reshape(-1, 3).mean(0)
    return '#%02X%02X%02X' % tuple(int(round(c * 255)) for c in m)


CMYK_PROFILE = '/System/Library/ColorSync/Profiles/Generic CMYK Profile.icc'


def soft_proof(tiles, tw, th):
    """Round-trip the twelve through CMYK so the gamut loss is visible up front.

    Blue, purple and indigo sit well outside CMYK; they print duller than the file.
    That is fine as long as every component ships through the same process --
    they shift together and still match on the table -- but it should not be a
    surprise at the proof stage.
    """
    if not os.path.exists(CMYK_PROFILE):
        print('(skipping soft proof: no CMYK profile on this machine)')
        return
    from PIL import ImageCms
    srgb_p = ImageCms.createProfile('sRGB')
    cmyk_p = ImageCms.getOpenProfile(CMYK_PROFILE)
    fwd = ImageCms.buildTransform(srgb_p, cmyk_p, 'RGB', 'CMYK', renderingIntent=0)
    rev = ImageCms.buildTransform(cmyk_p, srgb_p, 'CMYK', 'RGB', renderingIntent=0)

    rows = (len(tiles) + SHEET_COLS - 1) // SHEET_COLS
    sheet = Image.new('RGB', (SHEET_COLS * (tw + 8) + 8,
                              rows * (2 * th + 16) + 8), 'white')
    for i, (_, img) in enumerate(tiles):
        small = img.resize((tw, th), Image.LANCZOS)
        proof = ImageCms.applyTransform(ImageCms.applyTransform(small, fwd), rev)
        x = 8 + (i % SHEET_COLS) * (tw + 8)
        y = 8 + (i // SHEET_COLS) * (2 * th + 16)
        sheet.paste(small, (x, y))
        sheet.paste(proof, (x, y + th + 8))
    path = OUT_DIR / '_cmyk-soft-proof.png'
    sheet.save(path)
    print(f'soft proof:    {path.relative_to(REPO)}  '
          '(each card sRGB above, the same through CMYK below)')


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--generate', action='store_true',
                    help='call the image API for a fresh source texture (overwrites)')
    ap.add_argument('--synthetic-source', action='store_true',
                    help='write a procedural stand-in source instead of calling the API')
    ap.add_argument('--variants', type=int, metavar='N',
                    help='generate N candidate textures to _variant-N.png and stop')
    ap.add_argument('--pick', type=int, metavar='N',
                    help='promote _variant-N.png to the source texture')
    args = ap.parse_args()

    if args.variants:
        OUT_DIR.mkdir(exist_ok=True)
        for i in range(1, args.variants + 1):
            generate_source(OUT_DIR / f'_variant-{i}.png', f' {i}/{args.variants}')
        sheet = Image.new('RGB', ((WIDTH // 2 + 8) * args.variants + 8, HEIGHT // 2 + 16), 'white')
        for i in range(1, args.variants + 1):
            v = Image.open(OUT_DIR / f'_variant-{i}.png').resize((WIDTH // 2, HEIGHT // 2))
            sheet.paste(v, (8 + (i - 1) * (WIDTH // 2 + 8), 8))
        sheet.save(OUT_DIR / '_variants.png')
        print(f'\nwrote {OUT_DIR.relative_to(REPO)}/_variants.png -- pick one with --pick N')
        return

    if args.pick:
        src = OUT_DIR / f'_variant-{args.pick}.png'
        if not src.exists():
            sys.exit(f'no {src.relative_to(REPO)}')
        raw = src.with_name(src.stem + '-raw.png')
        chosen = Image.open(raw if raw.exists() else src)
        cover_resize(chosen, WIDTH, HEIGHT).save(SOURCE, dpi=(PPI, PPI))
        print(f'promoted variant {args.pick} to {SOURCE.relative_to(REPO)} '
              f'at {WIDTH}x{HEIGHT}')

    if args.synthetic_source:
        OUT_DIR.mkdir(exist_ok=True)
        Image.fromarray((synthetic_source() * 255 + 0.5).astype(np.uint8)).save(
            SOURCE, dpi=(PPI, PPI))
        print(f'wrote procedural stand-in to {SOURCE.relative_to(REPO)} (NOT final art)')
    elif args.generate or not SOURCE.exists():
        if not args.generate:
            sys.exit(f'no source texture at {SOURCE}; run with --generate or --synthetic-source')
        generate_source()

    palette = load_palette()
    OUT_DIR.mkdir(exist_ok=True)

    src = np.asarray(Image.open(SOURCE).convert('RGB')).astype(np.float64) / 255
    if src.shape[:2] != (HEIGHT, WIDTH):
        src = np.asarray(cover_resize(Image.fromarray((src * 255).astype(np.uint8)),
                                      WIDTH, HEIGHT)).astype(np.float64) / 255
    L, C, h = to_lch(linear_to_oklab(srgb_to_linear(src)))
    anchor = source_anchor(L, C, h)
    print(f'source plateau: L {anchor[0]:.3f}  C {anchor[1]:.3f}  h {anchor[2]:.1f}'
          f'   ({lch_to_hex(*anchor)})')
    print(f'source contrast p5..p95: {np.percentile(L, 95) - np.percentile(L, 5):.3f} OKLab L\n')

    print(f"{'card':<11} {'official':<9} {'plateau':<9} {'dE':>5}   {'mean':<9} {'dE':>5}   file")
    tiles = []
    for name in CARDS:
        target = palette[name]
        out = recolor(L, C, h, anchor, target)
        img = Image.fromarray((out * 255 + 0.5).astype(np.uint8))
        path = OUT_DIR / f'{name.lower()}.png'
        img.save(path, dpi=(PPI, PPI))

        # measure the file that was actually written, 8-bit quantization included
        back = np.asarray(Image.open(path).convert('RGB')).astype(np.float64) / 255
        plateau = lch_to_hex(*source_anchor(*to_lch(linear_to_oklab(srgb_to_linear(back)))))
        mn = mean_hex(back)
        d_plat = ciede2000(cielab(plateau), cielab(target))
        d_mean = ciede2000(cielab(mn), cielab(target))
        print(f'{name:<11} {target:<9} {plateau:<9} {d_plat:5.2f}   {mn:<9} {d_mean:5.2f}   '
              f'{path.relative_to(REPO)}')
        tiles.append((name, img))

    # contact sheet for eyeballing them side by side
    tw, th = WIDTH // 3, HEIGHT // 3
    rows = (len(tiles) + SHEET_COLS - 1) // SHEET_COLS
    sheet = Image.new('RGB', (SHEET_COLS * (tw + 8) + 8, rows * (th + 8) + 8), 'white')
    for i, (_, img) in enumerate(tiles):
        sheet.paste(img.resize((tw, th), Image.LANCZOS),
                    (8 + (i % SHEET_COLS) * (tw + 8), 8 + (i // SHEET_COLS) * (th + 8)))
    sheet.save(OUT_DIR / '_contact-sheet.png')
    print(f'\ncontact sheet: {(OUT_DIR / "_contact-sheet.png").relative_to(REPO)}')
    soft_proof(tiles, tw, th)
    print(f'each card {WIDTH}x{HEIGHT}px = {TRIM_MM[0] + 2 * BLEED_MM}x{TRIM_MM[1] + 2 * BLEED_MM}mm '
          f'({TRIM_MM[0]}x{TRIM_MM[1]}mm trim + {BLEED_MM}mm bleed) at {PPI}ppi')


if __name__ == '__main__':
    main()
