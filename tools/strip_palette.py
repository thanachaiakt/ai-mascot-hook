"""Save the pet's frame strips with a 48-color palette, or shrink strips already saved.

    python tools/strip_palette.py assets/happy.png ...   (needs Pillow and NumPy)

A strip is first cut to 256 colors, then its palette is merged down to COLORS by k-means over
the palette entries, each weighted by the square root of how many pixels use it. Quantizing
straight to fewer colors flattens the pale shading that covers most of the seal; merging the
palette keeps it, because the rare colors are the ones that get folded together. Weighting by
the plain count folded small props into their surroundings (the shovel's wooden handle went
grey); the square root keeps them. Against 256 colors the frames file is about a quarter
smaller and the pixels move by one to five steps in 255 on average.
"""
import sys
from pathlib import Path

import numpy as np
from PIL import Image

COLORS = 48
ROUNDS = 50                     # k-means rounds; 256 points settle long before this


def shrink(img, colors=COLORS):
    """8-bit palette image -> the same image on a palette of at most `colors` entries."""
    idx = np.asarray(img)
    pal = np.array(img.getpalette()[:768], np.float64).reshape(-1, 3)
    pal = np.pad(pal, ((0, 256 - len(pal)), (0, 0)))
    alpha = np.full(256, 255.0)
    trns = img.info.get('transparency')
    if isinstance(trns, bytes):
        alpha[:len(trns)] = list(trns)
    elif trns is not None:
        alpha[trns] = 0
    weight = np.sqrt(np.bincount(idx.ravel(), minlength=256))
    used = np.flatnonzero(weight)
    if len(used) <= colors:
        return img
    # premultiplied, so every clear entry is the same point whatever its color
    pts = np.c_[pal * alpha[:, None] / 255, alpha][used]
    w = weight[used]
    # the first center stays fully clear, or the clear background would pick up a faint tint;
    # the rest start from the most used entries, then refine
    seen = pts[:, 3] > 0
    centers = np.vstack([np.zeros(4), pts[seen][np.argsort(-w[seen])[:colors - 1]]])
    for _ in range(ROUNDS):
        label = ((pts[:, None] - centers[None]) ** 2).sum(-1).argmin(1)
        moved = centers.copy()
        for k in range(1, colors):
            m = label == k
            if m.any():
                moved[k] = (pts[m] * w[m, None]).sum(0) / w[m].sum()
        if np.allclose(moved, centers):
            break
        centers = moved
    label = ((pts[:, None] - centers[None]) ** 2).sum(-1).argmin(1)
    a = centers[:, 3].clip(0, 255)
    rgb = np.where(a[:, None] > 0, centers[:, :3] * 255 / np.maximum(a[:, None], 1), 0).clip(0, 255)
    remap = np.zeros(256, np.uint8)
    remap[used] = label
    out = Image.fromarray(remap[idx], 'P')
    out.putpalette(rgb.round().astype(np.uint8).ravel().tolist())
    out.info['transparency'] = bytes(a.round().astype(np.uint8))
    return out


def save_strip(strip, path):
    """RGBA strip -> the pet's 8-bit palette PNG at `path`."""
    write(shrink(strip.quantize(colors=256, method=Image.Quantize.FASTOCTREE)), path)


def write(img, path):
    extra = {'transparency': img.info['transparency']} if 'transparency' in img.info else {}
    img.save(path, optimize=True, **extra)


if __name__ == '__main__':
    for arg in sys.argv[1:]:
        path = Path(arg)
        before = path.stat().st_size
        write(shrink(Image.open(path)), path)
        print(f'{path}: {before // 1024} KB -> {path.stat().st_size // 1024} KB')
