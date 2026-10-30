"""Cut a clip of one seal sticker on the blue grid into a transparent GIF and the pet's frame
strip, the whole clip through.

    python tools/clip_pose.py <clip.mp4> <pose> [white]   (needs ffmpeg on PATH, Pillow, NumPy and SciPy)
    python tools/clip_pose.py salmon.mp4 salmon 0.3

The clip is 1280 x 720 at 24 fps. Each frame is cut away from the grid as sheet_pose.py cuts
a sticker, with its props (the salmon, crumbs) and lightened toward white: white is how much
of the fur's gap to white stays (video_poses.py's 0.55 when left out), so less is whiter.
Bubbles drifting off across the grid, away from the seal, are left out.

    assets/gif-poses/<pose>.gif     300 x 300, the clip's own length, transparent
    assets/<pose>.png               the pet's strip: 260 x 260 frames, every 2nd frame, 256 colors with alpha
"""
import subprocess
import sys
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

from sheet_pose import cut
from video_poses import GIF, PAD, STEP, STRIP, WHITEN, save_gif

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / 'assets' / 'gif-poses'
W, H = 1280, 720
FPS = 24
NEAR = 40                       # px from the seal a prop may be and still be kept


def frames(src):
    """The clip's frames one at a time, so the whole clip is never held at full size."""
    p = subprocess.Popen(['ffmpeg', '-v', 'error', '-i', src, '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-'],
                         stdout=subprocess.PIPE)
    while raw := p.stdout.read(W * H * 3):
        yield np.frombuffer(raw, np.uint8).reshape(H, W, 3)
    p.wait()


def build(src, name, keep):
    # each frame kept as uint8 premultiplied RGBA, cut down to what it covers
    kept = []
    for f in frames(src):
        rgba, seal = cut(f.astype(np.float32), keep)
        near = ndimage.binary_dilation(seal, iterations=NEAR)
        lab, m = ndimage.label(rgba[..., 3] > 0)
        touch = np.unique(lab[near & (lab > 0)])
        rgba[~np.isin(lab, touch)] = 0
        ys, xs = np.nonzero(rgba[..., 3] > 0)
        y0, x0 = ys.min(), xs.min()
        px = np.dstack([rgba[..., :3], rgba[..., 3:] * 255]).round().clip(0, 255).astype(np.uint8)
        kept.append((y0, x0, px[y0:ys.max() + 1, x0:xs.max() + 1]))
    # One square around everything the pose covers
    top = min(y for y, _, _ in kept)
    left = min(x for _, x, _ in kept)
    bottom = max(y + p.shape[0] for y, _, p in kept)
    right = max(x + p.shape[1] for _, x, p in kept)
    side = max(right - left, bottom - top) + 2 * PAD
    sx = (left + right - side) // 2
    sy = (top + bottom - side) // 2
    squares = []
    for y, x, p in kept:
        sq = Image.new('RGBa', (side, side), (0, 0, 0, 0))
        sq.paste(Image.fromarray(p, 'RGBa'), (x - sx, y - sy))
        squares.append(sq)
    OUT.mkdir(parents=True, exist_ok=True)
    path = OUT / f'{name}.gif'
    # GIF delays are in 10 ms steps: each frame ends on the step nearest its time in the
    # clip, so the GIF runs as long as the clip does
    ends = [round((n + 1) * 100 / FPS) * 10 for n in range(len(squares))]
    ms = [b - a for a, b in zip([0] + ends, ends)]
    save_gif([q.resize((GIF, GIF), Image.LANCZOS).convert('RGBA') for q in squares], path, ms)
    picked = squares[::STEP]
    strip = Image.new('RGBA', (STRIP * len(picked), STRIP), (0, 0, 0, 0))
    for n, q in enumerate(picked):
        strip.paste(q.resize((STRIP, STRIP), Image.LANCZOS).convert('RGBA'), (n * STRIP, 0))
    png = ROOT / 'assets' / f'{name}.png'
    strip.quantize(colors=256, method=Image.Quantize.FASTOCTREE).save(png, optimize=True)
    print(f'{path.relative_to(ROOT).as_posix()}: {side}px square, {len(squares)} frames, '
          f'{ends[-1] / 1000:.2f} s, {path.stat().st_size // 1024} KB; '
          f'strip {len(picked)} frames, {png.stat().st_size // 1024} KB')


if __name__ == '__main__':
    build(sys.argv[1], sys.argv[2], float(sys.argv[3]) if len(sys.argv) > 3 else WHITEN)
