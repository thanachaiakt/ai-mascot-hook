"""Render each seal state's animation into one horizontal strip of frames.

The motions are the ones the canvas and the mod's SVGs run in CSS, played here
frame by frame: positions are the canvas card's 240 x 260 area (plus a 10 px
margin), and frames are drawn at full size, then halved for a smooth edge.

    python tools/render_frames.py        (needs Pillow)

writes assets/<state>.png (256-color, with alpha) and prints each strip's frame count and interval.
"""
import math
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
ART, OUT = ROOT / 'art', ROOT / 'assets'
MARGIN = 10
W, H = 240 + 2 * MARGIN, 260 + 2 * MARGIN
SCALE = 1.0                     # frames keep the canvas size, 260 x 280; the pet scales them at run time


def art(name):
    return Image.open(ART / name).convert('RGBA')


def frame_of(strip, index, fw, fh):
    return strip.crop((index * fw, 0, index * fw + fw, fh))


# ---- timing --------------------------------------------------------------

def bezier(p1x, p1y, p2x, p2y):
    """CSS cubic-bezier as a function of progress."""
    def solve(x):
        lo, hi = 0.0, 1.0
        for _ in range(30):
            t = (lo + hi) / 2
            bx = 3 * (1 - t) ** 2 * t * p1x + 3 * (1 - t) * t * t * p2x + t ** 3
            lo, hi = (t, hi) if bx < x else (lo, t)
        t = (lo + hi) / 2
        return 3 * (1 - t) ** 2 * t * p1y + 3 * (1 - t) * t * t * p2y + t ** 3
    return solve


EASE_IN_OUT = bezier(.42, 0, .58, 1)
LINEAR = lambda x: x


def keyframes(t, period, frames, ease=LINEAR):
    """Value at time t of keyframes [(percent, (values...)), ...]; the easing runs per segment."""
    pct = (t % period) / period * 100
    for (p0, v0), (p1, v1) in zip(frames, frames[1:]):
        if p0 <= pct <= p1:
            u = ease((pct - p0) / (p1 - p0)) if p1 > p0 else 0
            return tuple(a + (b - a) * u for a, b in zip(v0, v1))
    return frames[-1][1]


def alternate(t, half, delay=0.0):
    """0..1..0 over 2 * half seconds, eased both ways (animation-direction: alternate)."""
    x = ((t - delay) % (2 * half)) / half
    return EASE_IN_OUT(x) if x <= 1 else EASE_IN_OUT(2 - x)


# ---- drawing -------------------------------------------------------------

def place(canvas, img, x, y, opacity=1.0):
    if opacity < 1:
        img = img.copy()
        a = img.getchannel('A').point(lambda v: round(v * opacity))
        img.putalpha(a)
    canvas.alpha_composite(img, (round(x + MARGIN), round(y + MARGIN)))


def transformed(layer, origin, tx=0.0, ty=0.0, deg=0.0, sx=1.0, sy=1.0):
    """CSS `transform: translate(tx,ty) rotate(deg) scale(sx,sy)` about `origin` (area coords)."""
    ox, oy = origin[0] + MARGIN, origin[1] + MARGIN
    r = math.radians(deg)
    c, s = math.cos(r), math.sin(r)
    # forward: p' = O + T + R S (p - O); PIL wants the inverse map p' -> p
    a, b = c / sx, s / sx
    d, e = -s / sy, c / sy

    def inv(px, py):
        qx, qy = px - ox - tx, py - oy - ty
        return ox + a * qx + b * qy, oy + d * qx + e * qy

    x0, y0 = inv(0, 0)
    coeffs = (a, b, x0, d, e, y0)
    return layer.transform(layer.size, Image.AFFINE, coeffs, resample=Image.BICUBIC)


def blank():
    return Image.new('RGBA', (W, H), (0, 0, 0, 0))


def sparkle(canvas, kind, x, y, u):
    """A twinkling sparkle: u = 0 full, 1 small and faint."""
    img = SPARK[kind]
    s = 1 - 0.45 * u
    w = max(1, round(img.width * s))
    small = img.resize((w, w), Image.NEAREST)
    place(canvas, small, x + (img.width - w) / 2, y + (img.width - w) / 2, 1 - 0.5 * u)


def bubble(canvas, cx, cy, r, tx, ty, s, opacity):
    if opacity <= 0.01:
        return
    k = 4                                  # drawn 4x and shrunk, for a smooth ring
    rr = r * s
    size = int(2 * rr + 6)
    big = Image.new('RGBA', (size * k, size * k), (0, 0, 0, 0))
    g = ImageDraw.Draw(big)
    c = size * k / 2
    g.ellipse([c - rr * k, c - rr * k, c + rr * k, c + rr * k], fill=(222, 241, 255, 115), outline=(74, 152, 216, 255), width=2 * k)
    g.arc([c - rr * .62 * k, c - rr * .62 * k, c + rr * .62 * k, c + rr * .62 * k], 190, 265, fill=(255, 255, 255, 255), width=max(1, round(rr * .22 * k)))
    gr = max(1, rr * .14) * k
    g.ellipse([c + rr * .45 * k - gr, c + rr * .45 * k - gr, c + rr * .45 * k + gr, c + rr * .45 * k + gr], fill=(255, 255, 255, 255))
    small = big.resize((size, size), Image.LANCZOS)
    place(canvas, small, cx + tx - size / 2, cy + ty - size / 2, opacity)


SPARK = {'big': art('sparkle-big.png'), 'small': art('sparkle-small.png')}


# ---- states --------------------------------------------------------------

def loading(t):
    open_ = (t % 0.4) < 0.2
    body = blank()
    place(body, CLAP_BODY, 10, 42)
    body.alpha_composite(transformed(LAYER(CLAP_L, 10, 42), (126, 184), deg=-10 if open_ else 0))
    body.alpha_composite(transformed(LAYER(CLAP_R, 10, 42), (210, 176), deg=10 if open_ else 0))
    frame = transformed(body, (120, 260), ty=0 if open_ else -4, sx=1 if open_ else 1.03, sy=1 if open_ else .97)
    sparkle(frame, 'big', 38, 46, alternate(t, 1.2))
    sparkle(frame, 'small', 29, 87, alternate(t, 1.2, -.5))
    sparkle(frame, 'small', 223, 87, alternate(t, 1.2, -.25))
    if open_:
        place(frame, ARCS_L, 40, 116)
        place(frame, ARCS_R, 225, 130)
    return frame


def working(t):
    body = blank()
    place(body, SWIM, 15, 40)
    place(body, frame_of(TAIL, int((t % .8) / .8 * 24) % 24, 110, 72), 127, 28)
    tx, ty, deg = keyframes(t, 2.4, [(0, (0, 0, 0)), (25, (-8, -6, -4)), (50, (-12, 0, 0)), (75, (-6, 6, 4)), (100, (0, 0, 0))], EASE_IN_OUT)
    frame = transformed(body, (120, 140), tx, ty, deg)
    sparkle(frame, 'big', 114, 29, alternate(t, 1.2))
    return frame


QPOP = [(0, (10, .3, 0)), (12, (0, 1.12, 1)), (20, (0, 1, 1)), (45, (-6, 1, 1)), (70, (0, 1, 1)), (88, (-4, 1, 1)), (100, (-10, .9, 0))]


def asking(t):
    frame = blank()
    place(frame, SIT_BASE, 20, 73)
    upper = blank()
    place(upper, SIT_UPPER, 20, 73)
    place(upper, frame_of(FLIP, int((t % 1.5) / 1.5 * 45) % 45, 52, 72), 180, 99)
    tilt = -6 + 11 * alternate(t, 1.5)
    frame.alpha_composite(transformed(upper, (132, 177), deg=tilt))
    sparkle(frame, 'big', 72, 83, alternate(t, 1.5))
    sparkle(frame, 'small', 73, 112, alternate(t, 1.5, -.5))
    sparkle(frame, 'big', 206, 40, alternate(t, 1.5, -.3))
    sparkle(frame, 'big', 194, 170, alternate(t, 1.5, -.6))
    for img, x, y, delay in ((Q_BIG, 150, 0, 0), (Q_SMALL, 98, 24, -2.6)):
        ty, s, op = keyframes(t - delay, 3.0, QPOP, EASE_IN_OUT)
        if op > 0.01:
            layer = blank()
            place(layer, img, x, y, op)
            frame.alpha_composite(transformed(layer, (x + img.width / 2, y + img.height), ty=ty, sx=s, sy=s))
    return frame


FLOAT = [(0, (0, 0, .5, 0)), (12, (1.4, -9.6, .57, 1)), (35, (4, -28, .7, .97)), (60, (-3, -56, .85, .93)), (85, (-.4, -76, .94, .9)), (100, (2, -90, 1, 0))]


def completed(t):
    frame = blank()
    seg = (t % 2.4) / 1.2
    step = math.floor((seg % 1) * 4) / 4
    k = step if seg < 1 else 1 - step
    nap = blank()
    place(nap, NAP, 0, 136)
    frame.alpha_composite(transformed(nap, (120, 260), sx=1 + .02 * k, sy=1 + .05 * k))
    for cx, cy, r, delay in ((93, 111, 15, 0), (94, 106, 10, -1.6), (99, 103, 7, -3.2)):
        tx, ty, s, op = keyframes(t - delay, 4.8, FLOAT)
        bubble(frame, cx, cy, r, tx, ty, s, op)
    return frame


def LAYER(img, x, y):
    layer = blank()
    place(layer, img, x, y)
    return layer


CLAP_BODY, CLAP_L, CLAP_R = art('clap-body.png'), art('clap-left.png'), art('clap-right.png')
ARCS_L, ARCS_R = art('arcs-left.png'), art('arcs-right.png')
SWIM, TAIL = art('swim-body210.png'), art('tail-strip.png')
SIT_BASE, SIT_UPPER, FLIP = art('sit-base2.png'), art('sit-upper2.png'), art('flip-strip.png')
Q_BIG, Q_SMALL = art('q-big.png'), art('q-small.png')
NAP = art('seal-nap.png')

# state: (draw, loop seconds, frames)
STATES = {
    'loading': (loading, 2.4, 48),
    'working': (working, 2.4, 48),
    'asking': (asking, 3.0, 60),
    'completed': (completed, 4.8, 60),
}

if __name__ == '__main__':
    OUT.mkdir(exist_ok=True)
    fw, fh = round(W * SCALE), round(H * SCALE)
    for name, (draw, loop, n) in STATES.items():
        strip = Image.new('RGBA', (fw * n, fh), (0, 0, 0, 0))
        for i in range(n):
            f = draw(loop * i / n)
            if SCALE != 1.0:
                f = f.resize((fw, fh), Image.LANCZOS)
            strip.alpha_composite(f, (i * fw, 0))
        # one 256-color palette per strip keeps it small, in the exe and in memory
        strip.quantize(colors=256, method=Image.Quantize.FASTOCTREE).save(OUT / f'{name}.png', optimize=True)
        print(f'{name}: {n} frames of {fw}x{fh}, {round(loop * 1000 / n)} ms each, {(OUT / f"{name}.png").stat().st_size // 1024} KB')
