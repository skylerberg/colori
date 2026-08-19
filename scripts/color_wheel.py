#!/usr/bin/env python3
"""Generate the Colori color wheel as an SVG.

Radial layout, measured from the wheel center out along a petal's spoke:
    0 ...... hub, where the three primaries meet
    reach .. closest approach of the petal circle (the spoke ends here)
    rim .... inner circle: petal mouths sit on it, tertiary ring starts
    outer .. outside edge of the wheel

Petal depth is rim - reach, which is also the diameter of the largest
circular token that fits inside a petal. alpha is the petal half-width in
degrees: each mouth spans its spoke angle +/- alpha along the rim, leaving
the primaries 120 - 2*alpha degrees of rim each.

The default circle mode draws each petal as a true circle clipped by the
rim. Leaf mode reproduces the older pointed-petal design, where fork is
the radius at which the two petal edges meet and edge is the radius of
those edge arcs.
"""

import argparse
import math
import subprocess
import sys
from pathlib import Path

RING = ['#FE5103', '#FFBF00', '#98F608', '#038DB3', '#3703F0', '#FC05DF']
PRIMARY = ['#E5031A', '#FDF70A', '#0159FE']
PETAL = ['#FC8820', '#01A200', '#9304FE']


def pt(r, a):
    return (r * math.cos(math.radians(a)), -r * math.sin(math.radians(a)))


def fmt(p):
    return f"{p[0]:.2f},{p[1]:.2f}"


def mirror(p):
    return (-p[0], p[1])


def circle_params(rim, alpha, reach):
    r_c = (rim ** 2 - reach ** 2) / (2 * (rim * math.cos(math.radians(alpha)) - reach))
    return r_c, r_c - reach


def circle_defs(rim, alpha, reach):
    r_c, rho = circle_params(rim, alpha, reach)
    S = pt(reach, 30)
    Pp = pt(rim, 30 + alpha)
    Pm = pt(rim, 30 - alpha)
    Cp = pt(r_c, 30)
    ang_p = math.degrees(math.atan2(-(Pp[1] - Cp[1]), Pp[0] - Cp[0]))
    large = 1 if 2 * (210 - ang_p) > 180 else 0
    return {
        'primary': f"M0,0 L{fmt(S)} A{rho:.2f},{rho:.2f} 0 0 1 {fmt(Pp)} A{rim:g},{rim:g} 0 0 0 {fmt(mirror(Pp))} A{rho:.2f},{rho:.2f} 0 0 1 {fmt(mirror(S))} Z",
        'petal': f"M{fmt(Pp)} A{rho:.2f},{rho:.2f} 0 {large} 0 {fmt(Pm)} A{rim:g},{rim:g} 0 0 0 {fmt(Pp)} Z",
        'outline': f"M{fmt(Pp)} A{rho:.2f},{rho:.2f} 0 {large} 0 {fmt(Pm)}",
        'spokes': f"M{fmt(mirror(S))} L0,0 L{fmt(S)} M0,0 L0,{reach:.2f}",
    }


def leaf_defs(rim, alpha, fork, edge):
    F = pt(fork, 30)
    Pp = pt(rim, 30 + alpha)
    Pm = pt(rim, 30 - alpha)
    return {
        'primary': f"M0,0 L{fmt(F)} A{edge:g},{edge:g} 0 0 1 {fmt(Pp)} A{rim:g},{rim:g} 0 0 0 {fmt(mirror(Pp))} A{edge:g},{edge:g} 0 0 1 {fmt(mirror(F))} Z",
        'petal': f"M{fmt(F)} A{edge:g},{edge:g} 0 0 1 {fmt(Pp)} A{rim:g},{rim:g} 0 0 1 {fmt(Pm)} A{edge:g},{edge:g} 0 0 1 {fmt(F)} Z",
        'outline': f"M{fmt(Pp)} A{edge:g},{edge:g} 0 0 0 {fmt(F)} A{edge:g},{edge:g} 0 0 0 {fmt(Pm)}",
        'spokes': f"M{fmt(mirror(F))} L0,0 L{fmt(F)} M0,0 L0,{fork:g}",
    }


def build_svg(args):
    if args.mode == 'circle':
        d = circle_defs(args.rim, args.alpha, args.reach)
    else:
        d = leaf_defs(args.rim, args.alpha, args.fork, args.edge)
    d['sector'] = (f"M0,{-args.outer:g} A{args.outer:g},{args.outer:g} 0 0 1 {fmt(pt(args.outer, 30))} "
                   f"L{fmt(pt(args.rim, 30))} A{args.rim:g},{args.rim:g} 0 0 0 0,{-args.rim:g} Z")
    d['tick'] = f"M0,{-args.rim:g} L0,{-args.outer:g}"
    defs = "".join(f'<path id="{k}" d="{v}"/>' for k, v in d.items())

    fills = []
    for i, c in enumerate(RING):
        tr = f' transform="rotate({60 * i})"' if i else ''
        fills.append(f'<use href="#sector" fill="{c}"{tr}/>')
    for group, colors in (('primary', PRIMARY), ('petal', PETAL)):
        for i, c in enumerate(colors):
            tr = f' transform="rotate({120 * i})"' if i else ''
            fills.append(f'<use href="#{group}" fill="{c}"{tr}/>')

    strokes = [f'<circle r="{args.outer:g}"/>', f'<circle r="{args.rim:g}"/>']
    for i in range(6):
        tr = f' transform="rotate({60 * i})"' if i else ''
        strokes.append(f'<use href="#tick"{tr}/>')
    strokes.append('<use href="#spokes"/>')
    for i in range(3):
        tr = f' transform="rotate({120 * i})"' if i else ''
        strokes.append(f'<use href="#outline"{tr}/>')

    half = args.outer + 8
    return ('<?xml version="1.0" encoding="UTF-8"?>\n'
            '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" '
            f'viewBox="{-half:g} {-half:g} {2 * half:g} {2 * half:g}">'
            f'<defs>{defs}</defs>'
            '<g>' + "".join(fills) + '</g>'
            f'<g fill="none" stroke="black" stroke-width="{args.stroke:g}">' + "".join(strokes) + '</g>'
            '</svg>\n')


def petal_metrics(args):
    if args.mode == 'circle':
        D, rho = circle_params(args.rim, args.alpha, args.reach)
        R = args.rim
        a1 = rho ** 2 * math.acos((D ** 2 + rho ** 2 - R ** 2) / (2 * D * rho))
        a2 = R ** 2 * math.acos((D ** 2 + R ** 2 - rho ** 2) / (2 * D * R))
        a3 = 0.5 * math.sqrt((-D + rho + R) * (D + rho - R) * (D - rho + R) * (D + rho + R))
        return a1 + a2 - a3, args.rim - args.reach

    F = pt(args.fork, 30)
    Pp = pt(args.rim, 30 + args.alpha)
    Pm = pt(args.rim, 30 - args.alpha)

    def seg(r, chord):
        phi = 2 * math.asin(chord / (2 * r))
        return r * r / 2 * (phi - math.sin(phi))

    tri = abs((Pp[0] - F[0]) * (Pm[1] - F[1]) - (Pm[0] - F[0]) * (Pp[1] - F[1])) / 2
    area = tri + seg(args.rim, math.dist(Pp, Pm)) + 2 * seg(args.edge, math.dist(F, Pp))

    mid = ((F[0] + Pp[0]) / 2, (F[1] + Pp[1]) / 2)
    dx, dy = Pp[0] - F[0], Pp[1] - F[1]
    clen = math.hypot(dx, dy)
    h = math.sqrt(args.edge ** 2 - (clen / 2) ** 2)
    n = (-dy / clen, dx / clen)
    cands = [(mid[0] + h * n[0], mid[1] + h * n[1]), (mid[0] - h * n[0], mid[1] - h * n[1])]
    sd = pt(1, 30)
    C = min(cands, key=lambda p: abs(sd[0] * p[1] - sd[1] * p[0]))
    token = 0
    for t in range(1001):
        rr = args.fork + (args.rim - args.fork) * t / 1000
        p = pt(rr, 30)
        token = max(token, 2 * min(args.edge - math.dist(p, C), args.rim - rr))
    return area, token


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--mode', choices=['circle', 'leaf'], default='circle')
    ap.add_argument('--outer', type=float, default=192)
    ap.add_argument('--rim', type=float, default=128)
    ap.add_argument('--alpha', type=float, default=30)
    ap.add_argument('--reach', type=float, default=55, help='circle mode: petal circle\'s closest approach to the hub')
    ap.add_argument('--fork', type=float, default=68, help='leaf mode: radius where the petal edges meet')
    ap.add_argument('--edge', type=float, default=96, help='leaf mode: radius of the petal edge arcs')
    ap.add_argument('--stroke', type=float, default=2.5)
    ap.add_argument('--out', type=Path, default=Path('color_wheel.svg'))
    ap.add_argument('--png', action='store_true', help='also render a PNG preview next to --out (uses qlmanage)')
    args = ap.parse_args()

    if not 0 < args.alpha < 60:
        sys.exit('alpha must be between 0 and 60 degrees')
    if not args.rim < args.outer:
        sys.exit('rim must be smaller than outer')
    if args.mode == 'circle':
        limit = args.rim * math.cos(math.radians(args.alpha))
        if not 0 < args.reach < limit:
            sys.exit(f'reach must be between 0 and rim*cos(alpha) = {limit:.1f}')
    else:
        min_edge = math.dist(pt(args.fork, 30), pt(args.rim, 30 + args.alpha)) / 2
        if not 0 < args.fork < args.rim:
            sys.exit('fork must be between 0 and rim')
        if args.edge < min_edge:
            sys.exit(f'edge must be at least half the fork-to-mouth chord = {min_edge:.1f}')

    args.out.write_text(build_svg(args))

    petal, token = petal_metrics(args)
    primary = (math.pi * args.rim ** 2 - 3 * petal) / 3
    ring = math.pi * (args.outer ** 2 - args.rim ** 2) / 6
    print(f"wrote {args.out}")
    print(f"areas: petal {petal:.0f}  primary {primary:.0f}  ring sector {ring:.0f}")
    print(f"largest token in a petal: {token:.0f} (wheel is {2 * args.outer:g} across)")

    if args.png:
        result = subprocess.run(
            ['qlmanage', '-t', '-s', '800', '-o', str(args.out.parent or Path('.')), str(args.out)],
            capture_output=True)
        png = args.out.with_name(args.out.name + '.png')
        if result.returncode == 0 and png.exists():
            print(f"wrote {png}")
        else:
            print('png render failed (qlmanage is macOS-only)', file=sys.stderr)


if __name__ == '__main__':
    main()
