"""Cut the sticker seals out of the 2 x 2 states clip (blue grid backdrop)."""
import numpy as np
from scipy import ndimage

BG = np.array([137, 207, 244], np.float32)       # backdrop blue; the grid lines are a lighter blue

# state -> quadrant of the 1280 x 720 clip (x0, y0, x1, y1)
QUAD = {
    'loading': (0, 0, 640, 360),
    'working': (620, 0, 1280, 440),     # the swimmer dips below the middle line
    'asking': (0, 340, 640, 720),
    'completed': (640, 360, 1280, 720),
}


def backdrop(f):
    """Pixels that look like the blue backdrop (grid lines included)."""
    r, g, b = f[..., 0], f[..., 1], f[..., 2]
    return (r > 95) & (r < 200) & (b - r > 45) & (g - r > 25)


def cut(rgb, keep_extra=None, hug=0):
    """rgb: HxWx3 uint8 of a region around one seal -> (RGBA uint8, mask of the seal).

    The seal is the biggest blob that is not backdrop once thin bits (splash lines, motion
    marks) are pared away; everything it encloses (ice, outline) is kept. keep_extra(lab, sizes)
    may name more blobs to keep (the sleeping seal's zZ). hug > 0 keeps only what lies within
    that many px of the dark outline (and what that encloses), which drops water rings and
    splashes stuck to the sticker.
    """
    f = rgb.astype(np.float32)
    bgish = backdrop(f)
    lab, _ = ndimage.label(bgish)
    edge = np.unique(np.concatenate([lab[0], lab[-1], lab[:, 0], lab[:, -1]]))
    outside = np.isin(lab, edge[edge > 0])
    fg = ~outside
    core = ndimage.binary_erosion(fg, iterations=3)
    clab, n = ndimage.label(core)
    if n == 0:
        return np.zeros(rgb.shape[:2] + (4,), np.uint8), np.zeros(rgb.shape[:2], bool)
    sizes = ndimage.sum(core, clab, range(1, n + 1))
    keep = [int(np.argmax(sizes)) + 1]
    if keep_extra:
        keep += keep_extra(clab, sizes, keep[0])
    main = ndimage.binary_dilation(clab == keep[0], iterations=4) & fg
    if hug:
        r, b = f[..., 0], f[..., 2]
        water = (b - r > 50) & (r > 110)           # light-blue drops and rings, never in the sticker
        dark = main & (f.mean(2) < 110) & (b - r < 70)   # the navy outline, not the water's blue edge
        main = main & ~water & (ndimage.distance_transform_edt(~dark) <= hug)
        main = ndimage.binary_opening(ndimage.binary_fill_holes(main), iterations=2)
        mlab, m = ndimage.label(main)
        if m > 1:
            main = mlab == np.argmax(ndimage.sum(main, mlab, range(1, m + 1))) + 1
    extra = ndimage.binary_dilation(np.isin(clab, keep[1:]), iterations=4) & fg if len(keep) > 1 else False
    mask = ndimage.binary_fill_holes(main | extra)
    # soft edge: the sticker's white border fades into the blue over a pixel or two
    inner = ndimage.binary_erosion(mask, iterations=1)
    band = mask & ~inner
    alpha = mask.astype(np.float32)
    whiteness = np.clip((f[..., 0] - BG[0]) / (250 - BG[0]), 0, 1)
    alpha[band] = np.maximum(whiteness[band], 0.35)
    a = alpha[..., None]
    col = np.clip(np.where(a > 0.01, (f - BG * (1 - a)) / np.maximum(a, 0.01), 0), 0, 255)
    return np.dstack([col, alpha * 255]).round().astype(np.uint8), mask
