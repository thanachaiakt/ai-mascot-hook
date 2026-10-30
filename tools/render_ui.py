"""Render ai-mascot-hook's UI art: the round menu buttons.

    python tools/render_ui.py        (needs Pillow)

writes assets/menu.png: eight 96 x 96 buttons side by side, drawn at twice their
on-screen size: resize, resize hovered, close, close hovered, settings, settings
hovered, feed, feed hovered.
"""
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

OUT = Path(__file__).resolve().parent.parent / 'assets'
K = 4                                    # supersampling
S = 96                                   # button size, 2x of the 48 px it shows at

NAVY = (24, 52, 110, 255)
BLUE = (74, 152, 216, 255)
RED = (212, 61, 98, 255)
WHITE = (255, 255, 255, 255)
HOVER = (227, 242, 253, 255)
DANGER = (255, 227, 232, 255)
SALMON = (236, 112, 72, 255)


def button(icon, fill, ring, ink):
    big = Image.new('RGBA', (S * K, S * K), (0, 0, 0, 0))
    c = S * K / 2
    r = 38 * K
    shadow = Image.new('RGBA', big.size, (0, 0, 0, 0))
    ImageDraw.Draw(shadow).ellipse([c - r, c - r + 4 * K, c + r, c + r + 4 * K], fill=(16, 32, 70, 90))
    big.alpha_composite(shadow.filter(ImageFilter.GaussianBlur(5 * K)))
    d = ImageDraw.Draw(big)
    d.ellipse([c - r, c - r, c + r, c + r], fill=fill, outline=ring, width=3 * K if ring != fill else 0)
    icon(d, c, ink)
    return big.resize((S, S), Image.LANCZOS)


def line(d, a, b, ink, w):
    d.line([a, b], fill=ink, width=w)
    for p in (a, b):
        d.ellipse([p[0] - w / 2, p[1] - w / 2, p[0] + w / 2, p[1] + w / 2], fill=ink)


def close_icon(d, c, ink):
    o, w = 14 * K, 6 * K
    line(d, (c - o, c - o), (c + o, c + o), ink, w)
    line(d, (c + o, c - o), (c - o, c + o), ink, w)


def resize_icon(d, c, ink):
    o, w, h = 15 * K, 5 * K, 9 * K
    line(d, (c - o + 3 * K, c + o - 3 * K), (c + o - 3 * K, c - o + 3 * K), ink, w)
    d.polygon([(c + o + K, c - o - K), (c + o + K, c - o + h + K), (c + o - h - K, c - o - K)], fill=ink)
    d.polygon([(c - o - K, c + o + K), (c - o - K, c + o - h - K), (c - o + h + K, c + o + K)], fill=ink)


def gear_icon(hole):
    """A cog: eight teeth around a ring, its hole painted in the button's own fill."""
    import math

    def draw(d, c, ink):
        teeth, r_out, r_body, r_hole, half = 8, 19 * K, 14 * K, 6 * K, 4.6 * K
        for i in range(teeth):
            a = 2 * math.pi * (i + 0.5) / teeth
            ux, uy = math.cos(a), math.sin(a)
            nx, ny = -uy, ux
            p = lambda t, s: (c + ux * t + nx * s, c + uy * t + ny * s)
            d.polygon([p(r_body - 2 * K, -half), p(r_out, -half * 0.75), p(r_out, half * 0.75), p(r_body - 2 * K, half)], fill=ink)
        d.ellipse([c - r_body, c - r_body, c + r_body, c + r_body], fill=ink)
        d.ellipse([c - r_hole, c - r_hole, c + r_hole, c + r_hole], fill=hole)
    return draw


def fish_icon(hole):
    """A salmon swimming left, its eye painted in the button's own fill."""
    def draw(d, c, ink):
        d.ellipse([c - 18 * K, c - 11 * K, c + 10 * K, c + 11 * K], fill=ink)
        d.polygon([(c + 6 * K, c), (c + 20 * K, c - 11 * K), (c + 17 * K, c), (c + 20 * K, c + 11 * K)], fill=ink)
        r = 2.6 * K
        d.ellipse([c - 10 * K - r, c - 3 * K - r, c - 10 * K + r, c - 3 * K + r], fill=hole)
        # a stripe of fat, as on a salmon fillet
        d.line([(c - 2 * K, c - 8 * K), (c + 2 * K, c + 8 * K)], fill=hole, width=2 * K)
    return draw


if __name__ == '__main__':
    OUT.mkdir(exist_ok=True)
    tiles = [
        button(resize_icon, WHITE, WHITE, NAVY), button(resize_icon, HOVER, BLUE, NAVY),
        button(close_icon, WHITE, WHITE, NAVY), button(close_icon, DANGER, RED, RED),
        button(gear_icon(WHITE), WHITE, WHITE, NAVY), button(gear_icon(HOVER), HOVER, BLUE, NAVY),
        button(fish_icon(WHITE), WHITE, WHITE, SALMON), button(fish_icon(HOVER), HOVER, BLUE, SALMON),
    ]
    strip = Image.new('RGBA', (S * len(tiles), S), (0, 0, 0, 0))
    for i, t in enumerate(tiles):
        strip.alpha_composite(t, (i * S, 0))
    strip.save(OUT / 'menu.png', optimize=True)
    print('menu.png', strip.size)
