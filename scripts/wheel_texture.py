#!/usr/bin/env python3
"""Give the color wheel a watercolor fill without moving any of its colors.

Same treatment as the card backgrounds: one painted texture, recolored per
region so that each region's dominant value lands exactly on its official hex.
The texture runs continuously across the whole wheel rather than restarting at
every boundary, so it reads as one painted object seen through twelve windows.

Region masks are derived analytically from the wheel geometry in color_wheel.py
-- same outer/rim/alpha/reach knobs -- so this cannot drift out of sync with the
flat wheel. Output is an SVG that embeds the textured fill as a raster and keeps
every stroke as vector, so the black linework stays crisp at any size.

    export GOOGLE_API_KEY=...
    python3 scripts/wheel_texture.py --generate   # one API call, square texture
    python3 scripts/wheel_texture.py              # build the wheel
"""

import argparse
import base64
import importlib.util
import io
import math
import sys
from pathlib import Path

import numpy as np
from PIL import Image

REPO = Path(__file__).resolve().parent.parent
SCRIPTS = REPO / 'scripts'
SOURCE = REPO / 'card-backgrounds' / '_wheel-source.png'
OUT_PNG = REPO / 'color_wheel_textured.png'          # fill + baked strokes
OUT_FILL = REPO / 'color_wheel_textured-fill.png'    # fill only, for your own linework
OUT_SVG = REPO / 'color_wheel_textured.svg'

RING_NAMES = ['Vermilion', 'Amber', 'Chartreuse', 'Teal', 'Indigo', 'Magenta']
PETAL_NAMES = ['Orange', 'Green', 'Purple']
PRIMARY_NAMES = ['Red', 'Yellow', 'Blue']


def load(name):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / f'{name}.py')
    mod = importlib.util.module_from_spec(spec)
    sys.argv = sys.argv[:1]
    spec.loader.exec_module(mod)
    return mod


def region_masks(p, size, ss):
    """Analytic masks for the twelve regions, supersampled for clean edges.

    The wheel is circles and wedges, so the masks come straight out of the
    geometry rather than needing an SVG rasterizer: a ring sector is an annulus
    slice, a petal is a circle clipped by the rim, and a primary is what is left
    inside the rim once the petals are removed, split by wedge.
    """
    cw = load('color_wheel')
    rc, rho = cw.circle_params(p.rim, p.alpha, p.reach)
    half = p.outer + 8
    n = size * ss
    axis = -half + (np.arange(n) + 0.5) * (2 * half) / n
    x = axis[None, :]
    y = axis[:, None]
    r = np.hypot(x, y)
    ang = np.degrees(np.arctan2(-y, x)) % 360

    def wedge(center, span):
        return (np.abs((ang - center + 180) % 360 - 180) < span / 2)

    petals = []
    for i in range(3):
        a = math.radians(30 - 120 * i)
        cx, cy = rc * math.cos(a), -rc * math.sin(a)
        petals.append((np.hypot(x - cx, y - cy) < rho) & (r < p.rim))
    any_petal = petals[0] | petals[1] | petals[2]

    masks = {}
    for i, name in enumerate(RING_NAMES):
        masks[name] = (r > p.rim) & (r < p.outer) & wedge(60 - 60 * i, 60)
    for i, name in enumerate(PETAL_NAMES):
        masks[name] = petals[i]
    for i, name in enumerate(PRIMARY_NAMES):
        masks[name] = (r < p.rim) & ~any_petal & wedge(90 - 120 * i, 120)

    disc = r < p.outer
    return masks, disc


def stroke_coverage(p, size):
    """Antialiased coverage for the wheel's linework, drawn analytically.

    Every stroke is a circle, a radial segment or a petal arc, so coverage comes
    from the distance to each centerline: a pixel is fully inked when it sits
    more than half a stroke width inside, and feathers over one pixel at the
    edge. No supersampling and no external rasterizer needed, and it stays in
    lockstep with the same geometry the masks come from.
    """
    cw = load('color_wheel')
    rc, rho = cw.circle_params(p.rim, p.alpha, p.reach)
    half = p.outer + 8
    px = 2 * half / size
    axis = (-half + (np.arange(size) + 0.5) * px).astype(np.float32)
    x, y = axis[None, :], axis[:, None]
    r = np.hypot(x, y)
    w = p.stroke / 2

    def ink(dist):
        return np.clip((w - dist) / px + 0.5, 0, 1)

    def seg(x0, y0, x1, y1):
        dx, dy = x1 - x0, y1 - y0
        t = np.clip(((x - x0) * dx + (y - y0) * dy) / (dx * dx + dy * dy), 0, 1)
        return np.hypot(x - (x0 + t * dx), y - (y0 + t * dy))

    def polar(radius, deg):
        a = math.radians(deg)
        return radius * math.cos(a), -radius * math.sin(a)

    cov = np.maximum(ink(np.abs(r - p.outer)), ink(np.abs(r - p.rim)))

    for i in range(6):                                    # ticks across the ring
        a = 90 - 60 * i
        cov = np.maximum(cov, ink(seg(*polar(p.rim, a), *polar(p.outer, a))))

    for i in range(3):                                    # hub spokes
        a = 30 - 120 * i
        cov = np.maximum(cov, ink(seg(0, 0, *polar(p.reach, a))))

    inside_rim = np.clip((p.rim - r) / px + 0.5, 0, 1)
    for i in range(3):                                    # petal arcs, clipped to the rim
        cx, cy = polar(rc, 30 - 120 * i)
        cov = np.maximum(cov, ink(np.abs(np.hypot(x - cx, y - cy) - rho)) * inside_rim)

    return cov


def downsample(mask, ss):
    """Average the supersampled mask down to coverage in [0, 1]."""
    h, w = mask.shape
    return mask.reshape(h // ss, ss, w // ss, ss).mean(axis=(1, 3))


def build(p, palette, texture, size, ss, swing):
    cb = load('card_backgrounds')
    cb.SWING = swing

    masks, disc = region_masks(p, size, ss)
    tex = np.asarray(cb.cover_resize(texture, size, size)).astype(np.float64) / 255
    L, C, h = cb.to_lch(cb.linear_to_oklab(cb.srgb_to_linear(tex)))
    # One anchor and one ramp for the whole painting, so the wash stays visually
    # continuous across boundaries; only the target color changes region to
    # region. Each region is then rendered over just its own pixels.
    anchor = cb.source_anchor(L, C, h)
    spread = cb.spread_of(L, anchor)

    out = np.zeros((size, size, 3))
    alpha = np.zeros((size, size))
    report = []
    for name, mask in masks.items():
        cov = downsample(mask, ss)
        sel = cov > 0
        if not sel.any():
            continue
        aim = cb.solve_aim(L, C, h, anchor, palette[name], spread=spread)
        tinted = cb.render(L[sel], C[sel], h[sel], anchor, aim, spread)
        out[sel] += tinted * cov[sel][:, None]
        alpha += cov

        solid = cov[sel] > 0.99
        if solid.sum() > 50:
            sL, sC, sh = cb.to_lch(cb.linear_to_oklab(cb.srgb_to_linear(tinted[solid])))
            plateau = cb.lch_to_hex(*cb.source_anchor(sL, sC, sh))
            report.append((name, palette[name], plateau,
                           cb.ciede2000(cb.cielab(plateau), cb.cielab(palette[name]))))

    alpha = np.clip(alpha, 0, 1)
    safe = np.maximum(alpha, 1e-6)[..., None]
    out = np.clip(out / safe, 0, 1)
    rgba = np.dstack([out, np.clip(downsample(disc, ss), 0, 1)])
    return (rgba * 255 + 0.5).astype(np.uint8), report


def svg(p, png_bytes):
    """Raster fill inside, vector strokes on top -- linework stays resolution-free."""
    cw = load('color_wheel')
    d = cw.circle_defs(p.rim, p.alpha, p.reach)
    d['tick'] = f'M0,{-p.rim:g} L0,{-p.outer:g}'
    defs = ''.join(f'<path id="{k}" d="{v}"/>' for k, v in d.items())

    strokes = [f'<circle r="{p.outer:g}"/>', f'<circle r="{p.rim:g}"/>']
    for i in range(6):
        strokes.append(f'<use href="#tick"{f" transform=\"rotate({60 * i})\"" if i else ""}/>')
    strokes.append('<use href="#spokes"/>')
    for i in range(3):
        strokes.append(f'<use href="#outline"{f" transform=\"rotate({120 * i})\"" if i else ""}/>')

    half = p.outer + 8
    b64 = base64.b64encode(png_bytes).decode()
    return ('<?xml version="1.0" encoding="UTF-8"?>\n'
            '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" '
            f'viewBox="{-half:g} {-half:g} {2 * half:g} {2 * half:g}">'
            f'<defs>{defs}</defs>'
            f'<image x="{-half:g}" y="{-half:g}" width="{2 * half:g}" height="{2 * half:g}" '
            f'href="data:image/png;base64,{b64}"/>'
            f'<g fill="none" stroke="black" stroke-width="{p.stroke:g}">'
            + ''.join(strokes) + '</g></svg>\n')


def generate_source(size):
    """Fetch a square texture. The wheel is square; the cards are not, so this
    gets its own 1:1 generation rather than reusing the portrait card source."""
    import os
    from io import BytesIO
    try:
        from google import genai
        from google.genai import types
    except ImportError:
        sys.exit('google-genai is not installed for this interpreter; use the '
                 'colori-art venv: ../colori-art/venv/bin/python')

    cb = load('card_backgrounds')
    key = os.environ.get('GOOGLE_API_KEY')
    if not key:
        sys.exit('GOOGLE_API_KEY is not set')

    print(f'requesting square source texture from {cb.MODEL} ...')
    client = genai.Client(api_key=key)   # must outlive the call; it owns the http pool
    resp = client.models.generate_content(
        model=cb.MODEL, contents=[cb.PROMPT],
        config=types.GenerateContentConfig(
            response_modalities=['IMAGE', 'TEXT'],
            image_config=types.ImageConfig(aspect_ratio='1:1', image_size='4K')))

    for part in resp.candidates[0].content.parts:
        if part.inline_data is not None and part.inline_data.data:
            img = Image.open(BytesIO(part.inline_data.data)).convert('RGB')
            print(f'  received {img.size[0]}x{img.size[1]}')
            w, h = img.size
            dx, dy = int(w * cb.EDGE_INSET), int(h * cb.EDGE_INSET)
            img = img.crop((dx, dy, w - dx, h - dy))
            SOURCE.parent.mkdir(exist_ok=True)
            img.save(SOURCE.with_name(SOURCE.stem + '-raw.png'))
            cb.cover_resize(img, size, size).save(SOURCE)
            print(f'  wrote {SOURCE.relative_to(REPO)} at {size}x{size}')
            return
    sys.exit('no image came back from the model')


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--generate', action='store_true', help='fetch a square source texture')
    ap.add_argument('--outer', type=float, default=192)
    ap.add_argument('--rim', type=float, default=128)
    ap.add_argument('--alpha', type=float, default=30)
    ap.add_argument('--reach', type=float, default=55)
    ap.add_argument('--stroke', type=float, default=1.25)
    ap.add_argument('--size', type=int, default=2400, help='raster edge in px')
    ap.add_argument('--ss', type=int, default=3, help='supersampling factor for mask edges')
    ap.add_argument('--embed', type=int, default=1400,
                    help='edge of the raster embedded in the SVG (PNG stays --size)')
    ap.add_argument('--swing', type=float, default=0.10,
                    help='texture contrast in OKLab L (cards use 0.12)')
    args = ap.parse_args()

    if args.generate:
        generate_source(args.size)
        return
    if not SOURCE.exists():
        sys.exit(f'no source texture at {SOURCE.relative_to(REPO)}; run --generate first')

    palette = load('color_check').load_palette()
    texture = Image.open(SOURCE).convert('RGB')

    rgba, report = build(args, palette, texture, args.size, args.ss, args.swing)
    img = Image.fromarray(rgba, 'RGBA')
    img.save(OUT_FILL)

    # Bake the linework in, so the PNG stands on its own. Alpha takes the union
    # of disc and stroke: the outer circle straddles r=outer, and clipping it to
    # the fill would shave off its outer half.
    ink = stroke_coverage(args, args.size)[..., None]
    flat = rgba.astype(np.float64)
    flat[..., :3] *= 1 - ink
    flat[..., 3:] = np.maximum(flat[..., 3:], ink * 255)
    Image.fromarray((flat + 0.5).astype(np.uint8), 'RGBA').save(OUT_PNG)

    # The SVG carries its own smaller copy of the fill: the strokes stay vector
    # either way, and a full-resolution embed balloons the file for no gain at
    # the sizes a wheel actually gets printed or displayed at.
    buf = io.BytesIO()
    embed = img if args.embed >= args.size else img.resize((args.embed, args.embed),
                                                           Image.LANCZOS)
    embed.save(buf, 'PNG', optimize=True)
    OUT_SVG.write_text(svg(args, buf.getvalue()))

    print(f"{'region':<11} {'official':<9} {'plateau':<9} {'dE00':>5}")
    for name, official, plateau, d in sorted(report, key=lambda t: -t[3]):
        print(f'{name:<11} {official:<9} {plateau:<9} {d:5.2f}')
    print(f'\nworst {max(d for *_, d in report):.2f}   texture swing {args.swing} OKLab L')
    print(f'{OUT_PNG.relative_to(REPO)}  {args.size}x{args.size} RGBA, strokes baked at {args.stroke:g}')
    print(f'{OUT_FILL.relative_to(REPO)}  {args.size}x{args.size} RGBA, fill only')
    print(f'{OUT_SVG.relative_to(REPO)}  vector strokes over the embedded fill '
          f'({OUT_SVG.stat().st_size / 1024:.0f} KB)')


if __name__ == '__main__':
    main()
