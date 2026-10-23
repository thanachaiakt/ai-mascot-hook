"""Render ai-mascot-hook's UI art: the round menu buttons.

    python tools/render_ui.py        (needs Pillow)

writes assets/menu.png: twelve 96 x 96 buttons side by side, drawn at twice their
on-screen size: status off, status off hovered, status on, status on hovered,
edit, edit hovered, resize, resize hovered, close, close hovered, settings,
settings hovered (also used while its submenu is open).
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


def bubble_icon(filled):
    def draw(d, c, ink):
        x0, y0, x1, y1 = c - 19 * K, c - 15 * K, c + 19 * K, c + 11 * K
        tail = [(c - 11 * K, y1 - 2 * K), (c - 13 * K, y1 + 10 * K), (c - 1 * K, y1 - 2 * K)]
        if filled:
            d.rounded_rectangle([x0, y0, x1, y1], radius=9 * K, fill=ink)
            d.polygon(tail, fill=ink)
            dot = WHITE
        else:
            d.rounded_rectangle([x0, y0, x1, y1], radius=9 * K, outline=ink, width=5 * K)
            d.polygon(tail, fill=ink)
            dot = ink
        for dx in (-9, 0, 9):
            r = 3.2 * K
            cx, cy = c + dx * K, (y0 + y1) / 2
            d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=dot)
    return draw


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


def pencil_icon(d, c, ink):
    """A pencil leaning right, writing on a short line."""
    import math
    ang = math.radians(-45)
    ux, uy = math.cos(ang), math.sin(ang)          # along the pencil, toward its end
    nx, ny = -uy, ux                               # across it
    tip = (c - 13 * K, c + 11 * K)
    half, length, cone = 5.5 * K, 30 * K, 9 * K
    p = lambda t, s: (tip[0] + ux * t + nx * s, tip[1] + uy * t + ny * s)
    d.polygon([p(cone, -half), p(length, -half), p(length, half), p(cone, half)], fill=ink)   # body
    d.polygon([p(0, 0), p(cone - K, -half + K), p(cone - K, half - K)], fill=ink)             # point
    d.polygon([p(length + 2 * K, -half), p(length + 7 * K, -half), p(length + 7 * K, half), p(length + 2 * K, half)], fill=ink)  # eraser
    line(d, (c - 4 * K, c + 19 * K), (c + 16 * K, c + 19 * K), ink, 4 * K)


if __name__ == '__main__':
    OUT.mkdir(exist_ok=True)
    tiles = [
        button(bubble_icon(False), WHITE, WHITE, NAVY), button(bubble_icon(False), HOVER, BLUE, NAVY),
        button(bubble_icon(True), WHITE, WHITE, BLUE), button(bubble_icon(True), HOVER, BLUE, BLUE),
        button(pencil_icon, WHITE, WHITE, NAVY), button(pencil_icon, HOVER, BLUE, NAVY),
        button(resize_icon, WHITE, WHITE, NAVY), button(resize_icon, HOVER, BLUE, NAVY),
        button(close_icon, WHITE, WHITE, NAVY), button(close_icon, DANGER, RED, RED),
        button(gear_icon(WHITE), WHITE, WHITE, NAVY), button(gear_icon(HOVER), HOVER, BLUE, NAVY),
    ]
    strip = Image.new('RGBA', (S * len(tiles), S), (0, 0, 0, 0))
    for i, t in enumerate(tiles):
        strip.alpha_composite(t, (i * S, 0))
    strip.save(OUT / 'menu.png', optimize=True)
    print('menu.png', strip.size)
