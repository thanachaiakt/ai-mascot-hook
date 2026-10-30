"""Cut a pose drawn as a 4 x 4 sprite sheet into a transparent GIF and the pet's frame strip.

    python tools/sheet_pose.py <sheet.png> <pose> <ms per frame>   (needs Pillow, NumPy and SciPy)
    python tools/sheet_pose.py image.png salmon 440

The sheet is square, a blue grid behind sixteen stickers of the seal, read left to right and
top to bottom. Each is cut away from the grid with its props and effects (salmon, crumbs,
sparkles, heart, bubbles) and lightened toward white as video_poses.py does. The stickers sit
a little apart from cell to cell, so each is moved to stand where the first does: its bottom
on the same row, the middle of its base on the same column.

    assets/gif/<pose>.gif     300 x 300, transparent
    assets/<pose>.png               the pet's strip: 260 x 260 frames, 256 colors with alpha
"""
import sys
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

from video_poses import GIF, PAD, STRIP, WHITEN, save_gif, whiten

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / 'assets' / 'gif'
SIDE = 4                        # cells across and down
BG = np.array([90, 183, 230], np.float32)   # backdrop blue; the grid lines are a lighter blue
SHUT = 300                      # px: grid shut in by the pose larger than this is backdrop
BASE = 25                       # rows at the seal's bottom whose middle lines the stickers up


def cut(f, keep=WHITEN):
    """f: HxWx3 float cell -> (HxWx4 float premultiplied RGBA, mask of the seal itself), the
    fur whitened keeping keep of its gap to white."""
    r, g, b = f[..., 0], f[..., 1], f[..., 2]
    # Both the backdrop and its lighter lines are far bluer than the seal's lavender fur,
    # the pale bubbles and the white sticker edge.
    grid = (b - r > 70) & (g - r > 45)
    lab, n = ndimage.label(grid)
    sizes = ndimage.sum(grid, lab, range(1, n + 1))
    edge = np.unique(np.concatenate([lab[0], lab[-1], lab[:, 0], lab[:, -1]]))
    out = np.isin(lab, edge[edge > 0]) | np.isin(lab, np.flatnonzero(sizes > SHUT) + 1)
    fg = ndimage.binary_opening(~out, iterations=1)
    flab, m = ndimage.label(fg)
    fsizes = ndimage.sum(fg, flab, range(1, m + 1))
    seal = flab == fsizes.argmax() + 1
    fg = np.isin(flab, np.flatnonzero(fsizes >= 20) + 1)
    # Soft edge: the sticker's white border fades into the blue over a pixel or two
    band = fg & ~ndimage.binary_erosion(fg, iterations=1)
    alpha = fg.astype(np.float32)
    alpha[band] = np.maximum(np.clip((r[band] - BG[0]) / (250 - BG[0]), 0, 1), 0.35)
    a = alpha[..., None]
    col = np.clip(np.where(a > 0.01, (f - BG * (1 - a)) / np.maximum(a, 0.01), 0), 0, 255)
    return np.dstack([whiten(col, keep) * a, alpha]), seal


def foot(seal):
    """The seal's bottom row and the middle column of its base."""
    ys, xs = np.nonzero(seal)
    bottom = ys.max()
    return bottom, xs[ys > bottom - BASE].mean()


def build(sheet, name, ms):
    v = np.asarray(Image.open(sheet).convert('RGB')).astype(np.float32)
    c = v.shape[0] // SIDE
    cells = [cut(v[r * c:(r + 1) * c, k * c:(k + 1) * c]) for r in range(SIDE) for k in range(SIDE)]
    # Every sticker moved onto the first one's foot, on a canvas with room for the moves
    y0, x0 = foot(cells[0][1])
    room = c // 4
    frames = []
    for f, seal in cells:
        y, x = foot(seal)
        dy, dx = round(y0 - y), round(x0 - x)
        big = np.zeros((c + 2 * room, c + 2 * room, 4), np.float32)
        big[room + dy:room + dy + c, room + dx:room + dx + c] = f
        frames.append(big)
    # One square around everything the pose covers
    ys, xs = np.nonzero(np.any([f[..., 3] > 0.5 for f in frames], axis=0))
    side = max(xs.max() - xs.min(), ys.max() - ys.min()) + 1 + 2 * PAD
    sx = (xs.min() + xs.max() + 1 - side) // 2
    sy = (ys.min() + ys.max() + 1 - side) // 2
    squares = []
    for f in frames:
        px = np.dstack([f[..., :3], f[..., 3:] * 255]).round().clip(0, 255).astype(np.uint8)
        sq = Image.new('RGBa', (side, side), (0, 0, 0, 0))
        sq.paste(Image.fromarray(px, 'RGBa'), (-sx, -sy))
        squares.append(sq)
    OUT.mkdir(parents=True, exist_ok=True)
    path = OUT / f'{name}.gif'
    save_gif([q.resize((GIF, GIF), Image.LANCZOS).convert('RGBA') for q in squares], path, ms)
    strip = Image.new('RGBA', (STRIP * len(squares), STRIP), (0, 0, 0, 0))
    for n, q in enumerate(squares):
        strip.paste(q.resize((STRIP, STRIP), Image.LANCZOS).convert('RGBA'), (n * STRIP, 0))
    png = ROOT / 'assets' / f'{name}.png'
    strip.quantize(colors=256, method=Image.Quantize.FASTOCTREE).save(png, optimize=True)
    print(f'{path.relative_to(ROOT).as_posix()}: {side}px square, {len(squares)} frames of {ms} ms, '
          f'{path.stat().st_size // 1024} KB; strip {png.stat().st_size // 1024} KB')


if __name__ == '__main__':
    build(sys.argv[1], sys.argv[2], int(sys.argv[3]))
