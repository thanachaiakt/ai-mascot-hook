"""Cut the top seven poses out of the 4 x 3 seal sticker clip into transparent, looping GIFs
and the pet's frame strips.

    python tools/video_poses.py <clip.mp4>            (needs ffmpeg on PATH, Pillow, NumPy and SciPy)
    python tools/video_poses.py <clip.mp4> --search   (print loop points to pick from instead)

The clip is 1280 x 720 at 24 fps, a blue grid with a label under each pose. Each pose is cut
away from the grid with its props and effects (clock, car, ball, sparkles, "BREAK!")
and lightened toward white, then trimmed to a stretch whose end all but matches its start, so it loops without a jump; the
last few frames fade into the ones before the start to hide what is left of the seam.
Thinking keeps the seal alone: its "?" and zZ marks go, and where they cross the head, the
seal there is taken from other frames of the same look. Its head rises past the top of the
clip, cutting off the white sticker edge there, so that edge is drawn back in. Error's tears,
the backdrop's own blue, stay whole.

    assets/gif/<pose>.gif     300 x 300, 25 fps, transparent
    assets/<pose>.png               the pet's strip: 260 x 260 frames, every 2nd frame, 256 colors with alpha
"""
import subprocess
import sys
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage
from scipy.spatial import Delaunay

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / 'assets' / 'gif'
W, H = 1280, 512                # only the top two rows of cells are read
CELL = 320
# y spans of each row of cells. The ball flies up past the first row's labels, so the second
# row starts above where they end (LABEL_END) and their letters are taken out by hand. The
# second row stops above its own labels' Thai vowel marks.
ROWS = [(0, 212), (232, 496)]
LABEL_END = 246
GIF, STRIP = 300, 260           # frame sides of the GIF and of the pet's strip
STEP = 2                        # the strip keeps every 2nd frame, as the states' strips do
PAD = 6                        # px of clear space around the pose in the square
FADE = 4                        # frames blended across the seam
MS = 40                         # GIF delays are in 10 ms steps; 24 fps rounds to 25

BG = np.array([85, 180, 230], np.float32)   # backdrop blue; the grid lines are a lighter blue
WHITEN = 0.55                   # how much of the gap to white the seal's grey-blue keeps
SHUT = 300                      # px: grid shut in by the pose larger than this is backdrop

# pose -> (cell, frames the loop may use). Break Time shows the seal only before frame 24
# and after 180, so its loop keeps to the ringing clock. Eat Salmon (cell 7) is drawn by
# sheet_pose.py instead.
POSES = {
    'thinking': (0, (0, 240)),
    'active': (1, (0, 240)),
    'error': (2, (0, 240), 1500),     # its big tear is backdrop blue
    'slide': (3, (0, 240)),
    'break-time': (4, (24, 180)),
    'toy-car': (5, (0, 240)),
    'ball': (6, (0, 240)),
}
# poses cut down to the seal alone: thinking's "?" and zZ would read as napping
SOLO = {'thinking'}
MARK_ZONE = (85, 205)           # px: the marks cross the head above this row, right of this column
                                # (the far eye, the near one starts left of it)
MARK_LOOKS = 7                  # frames of the same look the head is taken from
MARK_APART = 3                  # frames between any two of them and this one, so a mark moves on
MARK_DARK = 25                  # how much darker than nearly all of them a pixel is to be a mark
MARK_MOVE = 12                  # px the seal may have moved by in a frame a pixel is taken from
MARK_SHIFT = 4                  # px more each patch of it may have moved by, as the seal leans
MARK_WIN = 15                   # px: side of the patch each pixel is lined up by
# poses whose head rises past the top of the clip, cutting off the sticker's white edge
TOPPED = {'thinking'}
TOP = 6                         # rows added above the clip to draw that edge in
RIM = 4                         # px: the white edge's width, its blur into the blue included
RIM_WHITE = np.array([234, 236, 240], np.float32)
# poses with a prop the blue of the backdrop: error's big tear. Only its darker teal outline
# tells it apart, and where the tear crosses the seal's edge the backdrop would flood in
# through that outline, leaving a hole in the face.
WALLED = {'error'}
DROP = 300                      # px: the most a tear drop covers; the seal is far bigger

# pose -> (first frame, frame that matches it), picked by eye from what --search lists.
LOOP = {
    'thinking': (40, 187),
    'active': (11, 109),
    'error': (86, 209),
    'slide': (50, 238),
    'break-time': (51, 103),
    'toy-car': (4, 232),
    'ball': (141, 229),
}


def frames(src):
    raw = subprocess.run(['ffmpeg', '-v', 'error', '-i', src, '-vf', f'crop={W}:{H}:0:0',
                          '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-'],
                         capture_output=True, check=True).stdout
    return np.frombuffer(raw, np.uint8).reshape(-1, H, W, 3)


def cell(v, k):
    r, c = divmod(k, 4)
    y0, y1 = ROWS[r]
    return v[:, y0:y1, c * CELL:(c + 1) * CELL]


def search(v):
    for name, (k, (lo, hi), *_) in POSES.items():
        a = cell(v, k)[lo:hi, ::2, ::2].astype(np.float32).reshape(hi - lo, -1)
        sq = (a ** 2).sum(1)
        d = np.sqrt(np.maximum(sq[:, None] + sq[None] - 2 * a @ a.T, 0) / a.shape[1])
        step = np.median(np.diag(d, 1))
        n = len(d)
        # The frames around each end must match too, so things move the same way across
        # the seam and do not just pass the same spot.
        best = []
        for i in range(FADE, n - 2):
            for j in range(i + 24, n - 2):
                s = np.mean([d[i + t, j + t] for t in range(-2, 3)])
                if s < step:
                    best.append((s, i + lo, j + lo))
        best.sort(key=lambda t: (-(t[2] - t[1]) // 24, t[0]))     # longest first, by the second
        shown = []
        for s, i, j in best:
            if all(abs(i - p) > 8 or abs(j - q) > 8 for _, p, q in shown):
                shown.append((s, i, j))
            if len(shown) == 5:
                break
        print(f'{name}: step {step:.1f}  ' + '  '.join(f'{i}->{j} ({j - i}) {s:.1f}' for s, i, j in shown))


def whiten(c, keep=WHITEN):
    """Lift the seal's grey-blue fur toward white, keeping keep of its gap to white. Only
    pale, low-chroma colors with more blue than red move, so the navy outline, sunglasses,
    cheeks and props keep their colors."""
    mx, mn = c.max(-1), c.min(-1)
    sat = (mx - mn) / np.maximum(mx, 1)
    w = (np.clip((0.45 - sat) / 0.1, 0, 1) * np.clip((c.mean(-1) - 100) / 25, 0, 1)
         * np.clip((c[..., 2] - c[..., 0]) / 10 + 1, 0, 1))[..., None]
    lum = c.mean(-1, keepdims=True)
    lifted = 255 - (255 - (0.7 * c + 0.3 * lum)) * keep
    return c + (lifted - c) * w


def clear_marks(c, lo, hi):
    """c: the frames of a cell -> {t: float frame} for frames lo..hi-1 with the marks taken
    off the seal's head.

    Where a mark crosses the head it is drawn over the seal, which no cut can bring back. But
    the seal blinks, nods off and leans the same way more than once while the marks drift by,
    so other frames show the seal as it is here with the marks elsewhere. The frames of the
    same look are found by the whole head, with the pixels that differ most (marks among
    them) left out of the count, and lined up with this one patch by patch, as the seal
    leans and squashes rather than slides. A mark is navy, darker than whatever it covers,
    so a pixel is a mark where it is clearly darker than most of those frames; there it
    takes the color of the frame closest to their median, and the art stays the seal's own,
    pixel steps and all. The "?" drifts slowly, so frames near in time may share it: every
    frame is cleared once, and the frames asked are then the cleared ones.
    """
    n, h, w = c.shape[:3]
    zone = np.zeros((h, w), bool)
    zone[:MARK_ZONE[0], MARK_ZONE[1]:] = True
    head = np.zeros((h, w), bool)
    head[:160, 40:] = True
    f32 = c.astype(np.float32)
    keep = int(head.sum() * 0.85)
    # the zone's box, with room for the patches and the shift
    y1 = MARK_ZONE[0] + MARK_WIN + MARK_SHIFT
    x0 = MARK_ZONE[1] - MARK_WIN - MARK_SHIFT
    box = (slice(0, y1), slice(x0, w))
    z = zone[box]
    pad = MARK_MOVE + MARK_SHIFT

    def gap(a, b):
        # the largest channel's difference; max(-1) over three channels is many times slower
        d = np.abs(a - b)
        return np.maximum(np.maximum(d[..., 0], d[..., 1]), d[..., 2])

    def clear(t, frames, looks, ref, most):
        e = np.abs(looks - looks[t]).max(-1)
        dist = np.partition(e, keep, axis=1)[:, :keep].mean(1)
        picked = []
        for u in np.argsort(dist):
            if all(abs(u - p) >= MARK_APART for p in picked + [t]):
                picked.append(u)
            if len(picked) == MARK_LOOKS:
                break
        f = f32[t][box]
        ref = ref[box]
        bh, bw = f.shape[:2]
        stack = []
        for u in picked:
            src = np.pad(frames[u], ((pad, pad), (pad, pad), (0, 0)), mode='edge')[:y1 + 2 * pad, x0:]
            def at(dy, dx):
                return src[pad - dy:pad - dy + bh, pad - dx:pad - dx + bw]
            # the whole box first, coarse then fine, then each patch around that
            def off(m):
                return np.minimum(gap(at(*m)[::2, ::2], ref[::2, ::2]), 60).mean()
            span = range(-MARK_MOVE, MARK_MOVE + 1, 3)
            gy, gx = min(((dy, dx) for dy in span for dx in span), key=off)
            near = [(gy + dy, gx + dx) for dy in (-1, 0, 1) for dx in (-1, 0, 1)
                    if max(abs(gy + dy), abs(gx + dx)) <= MARK_MOVE]
            gy, gx = min(near, key=off)
            best_e = np.full((bh, bw), np.inf, np.float32)
            best = np.zeros_like(f)
            for dy in range(gy - MARK_SHIFT, gy + MARK_SHIFT + 1):
                for dx in range(gx - MARK_SHIFT, gx + MARK_SHIFT + 1):
                    moved = at(dy, dx)
                    # capped, so a mark in either frame does not sway the match
                    err = ndimage.uniform_filter(np.minimum(gap(moved, ref), 60), MARK_WIN, mode='nearest')
                    better = err < best_e
                    np.copyto(best_e, err, where=better)
                    np.copyto(best, moved, where=better[..., None])
            stack.append(best)
        stack = np.stack(stack)
        # not the median: where the seal's edges sit a pixel apart in some of the frames
        # it would take a pupil or outline here for a mark
        bad = f.mean(-1) < np.quantile(stack.mean(-1), 1 - most, axis=0) - MARK_DARK
        med = np.median(stack, 0)
        # A mark's blur over the backdrop is hardly darker than it, but where the frames
        # all show backdrop, anything else is a mark: the backdrop never blinks.
        r, g, b = med[..., 0], med[..., 1], med[..., 2]
        blue = (r < 135) & (b - r > 95) & (g - r > 60) & (np.ptp(stack.mean(-1), axis=0) < 30)
        bad |= blue & (gap(f, med) > MARK_DARK)
        bad = ndimage.binary_dilation(bad & z, iterations=1) & z
        k = np.abs(stack - med).sum(-1).argmin(0)
        pick = np.take_along_axis(stack, k[None, ..., None], 0)[0]
        g = f32[t].copy()
        g[box][bad] = pick[bad]
        return g

    first = np.stack([clear(t, f32, f32[:, head], f32[t], 0.75) for t in range(n)])
    # Lined up by the cleared frame too, where a mark no longer pulls the match aside. The
    # cleared frames hold no marks to outvote, so a mark is darker than all of them: when
    # the seal looks like few other frames (it leans once) and some of them are a poor
    # match, one that shows its eye where this one does keeps the eye.
    return {t: clear(t, first, first[:, head], first[t], 1.0) for t in range(lo, hi)}


def top_rim(f, still, shut=SHUT):
    """f: HxWx3 float frame of a TOPPED cell -> the frame with TOP rows of backdrop added above
    and the sticker's white edge drawn into them where the clip cut it off.

    The edge runs RIM px out from the seal's navy outline. It is drawn from the outline's
    top row in each column, not grown from the cut, so the specks of blur and of marks the
    top row of the clip holds give it no bumps; above it the top rows are backdrop again.
    """
    f = np.concatenate([np.broadcast_to(BG, (TOP,) + f.shape[1:]), f])
    still = np.pad(still, ((TOP, 0), (0, 0)))
    h = f.shape[0]
    navy = (cut(f, still, shut, solo=True)[..., 3] > 0.5) & (f.mean(-1) < 120)
    line = np.where(navy.any(0), navy.argmax(0), h)
    edge = ndimage.minimum_filter1d(line, 2 * RIM + 1, mode='nearest') - RIM
    # only the rows the clip cut short, and the two below them where the edge was left thin
    for x in np.flatnonzero(edge < TOP + 2):
        f[:max(edge[x], 0), x] = BG
        f[max(edge[x], 0):min(line[x], TOP + 2), x] = RIM_WHITE
    return f, still


def backdrop(f, still, shut=SHUT, walled=False):
    """f: HxWx3 float frame of one cell -> the mask of everything that is not grid and not
    cut off from the cell's edge by the pose. still marks pixels of the first row's labels,
    which are never kept. walled counts only bright blue as grid, so a blue prop's darker
    outline keeps the backdrop out of it.
    """
    r, g, b = f[..., 0], f[..., 1], f[..., 2]
    grid = (r < 135) & (b - r > 95) & (g - r > 60)
    if walled:
        grid &= g > 160
    lab, n = ndimage.label(grid | still)
    sizes = ndimage.sum(np.ones_like(r), lab, range(1, n + 1))
    edge = np.unique(np.concatenate([lab[0], lab[-1], lab[:, 0], lab[:, -1]]))
    # Grid shut in between a flipper and the body is still backdrop; small blue bits
    # (tears, drops) inside the outline are not.
    out = np.isin(lab, edge[edge > 0]) | np.isin(lab, np.flatnonzero(sizes > shut) + 1)
    fg = ndimage.binary_opening(~out, iterations=1)
    if walled:
        # A drop's outline is thin and breaks here and there, letting its blue inside out
        # with the backdrop. On the face, bridging the outline shuts the inside in again:
        # small holes so made are drops (the backdrop shut in by a flipper is bigger).
        shut_in = ndimage.binary_closing(fg, iterations=3)
        holes, k = ndimage.label(ndimage.binary_fill_holes(shut_in) & ~shut_in)
        small = ndimage.sum(np.ones_like(r), holes, range(1, k + 1)) < DROP
        fg |= np.isin(holes, np.flatnonzero(small) + 1)
        # A flying drop just forming is only an arc or two: drops are round, so each small
        # cluster of pieces gets back all its hull covers.
        groups, k = ndimage.label(ndimage.binary_dilation(fg, iterations=2))
        for i, sl in enumerate(ndimage.find_objects(groups), 1):
            px = np.argwhere(fg[sl] & (groups[sl] == i))
            if len(px) >= DROP or len(px) < 3 or np.linalg.matrix_rank(px - px[0]) < 2:
                continue
            box = np.argwhere(np.ones(fg[sl].shape, bool))
            fg[sl] |= (Delaunay(px).find_simplex(box) >= 0).reshape(fg[sl].shape)
    return fg


def unmarked(f, fg):
    """The part of fg that is the seal, without the marks floating by its head.

    The seal's navy outline lies between its pale fur and its white sticker edge, which
    rings it whole, so the seal is what the pale parts enclose; the marks are bare navy,
    blurred into the blue, with nothing pale beside them. Where a mark crosses the edge it
    breaks the ring, and the eye beside it would leak out with the mark: a wider closing
    mends the ring, and its inside, away from the edge, comes back.
    """
    r, b = f[..., 0], f[..., 2]
    pale = fg & (f.mean(-1) > 140) & (b - r < 90)
    within = lambda k: ndimage.binary_fill_holes(ndimage.binary_closing(pale, iterations=k))
    return fg & (ndimage.binary_dilation(within(3), iterations=1) | ndimage.binary_erosion(within(6), iterations=4))


def cut(f, still, shut=SHUT, solo=False, walled=False):
    """f: HxWx3 float frame of one cell -> HxWx4 float RGBA with the grid taken away.

    Everything backdrop() keeps stays, so props and effects stay, unless solo keeps only
    the seal: unmarked() and then the biggest piece.
    """
    r = f[..., 0]
    fg = backdrop(f, still, shut, walled)
    if solo:
        fg = unmarked(f, fg)
        # crumbs of a mark still cling to the rim; the head is round, so up by it whatever
        # sticks out past its smoothed outline goes
        zone = np.zeros_like(fg)
        zone[:MARK_ZONE[0], MARK_ZONE[1]:] = True
        fg &= ndimage.binary_dilation(ndimage.binary_opening(fg, iterations=5), iterations=1) | ~zone
    flab, m = ndimage.label(fg)
    fsizes = ndimage.sum(fg, flab, range(1, m + 1))
    # Drops encoder speckles, and bits cut off at the side edges: the toy car's dust cloud
    # drifts across into the Break Time cell.
    sides = np.unique(np.concatenate([flab[:, 0], flab[:, -1]]))
    small_side = np.isin(np.arange(1, m + 1), sides) & (fsizes < 3000)
    keep = (fsizes >= 20) & ~small_side
    if solo:
        keep &= fsizes == fsizes.max()
    fg = np.isin(flab, np.flatnonzero(keep) + 1)
    # Soft edge: the sticker's white border fades into the blue over a pixel or two
    band = fg & ~ndimage.binary_erosion(fg, iterations=1)
    alpha = fg.astype(np.float32)
    whiteness = np.clip((r - BG[0]) / (250 - BG[0]), 0, 1)
    alpha[band] = np.maximum(whiteness[band], 0.35)
    a = alpha[..., None]
    col = np.clip(np.where(a > 0.01, (f - BG * (1 - a)) / np.maximum(a, 0.01), 0), 0, 255)
    col = whiten(col)
    return np.dstack([col * a, alpha])                           # premultiplied, for the fade


def save_gif(frames, path, ms=MS):
    """Premultiplied RGBA frames -> one transparent GIF with a palette shared by every frame."""
    w = frames[0].width
    sheet = np.concatenate([np.asarray(f) for f in frames], axis=1)
    solid = sheet[..., 3] >= 128
    rgb = sheet[..., :3].copy()
    rgb[~solid] = 255                                  # keep the hidden pixels out of the palette
    pal_img = Image.fromarray(rgb).quantize(colors=255, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
    idx = np.asarray(pal_img).copy()
    idx[~solid] = 255
    pal = pal_img.getpalette()[:255 * 3] + [0, 0, 0]
    out = []
    for i in range(len(frames)):
        im = Image.fromarray(idx[:, i * w:(i + 1) * w], 'P')
        im.putpalette(pal)
        im.info['transparency'] = 255
        out.append(im)
    out[0].save(path, save_all=True, append_images=out[1:], duration=ms, loop=0,
                disposal=2, transparency=255, optimize=False)


def build(v):
    OUT.mkdir(parents=True, exist_ok=True)
    for name, (k, _, *shut) in POSES.items():
        i, j = LOOP[name]
        c = cell(v, k)
        still = np.zeros(c.shape[1:3], bool)
        if k >= 4:
            # The label letters never move; the ball is over them for a few frames only
            label = slice(0, LABEL_END - ROWS[1][0])
            med = np.median(c[::4, label], axis=0)
        keyed = {}
        cleared = clear_marks(c, i - FADE, j) if name in SOLO else {}
        for t in range(i - FADE, j):
            f = cleared[t] if name in SOLO else c[t].astype(np.float32)
            s = still.copy()
            if k >= 4:
                s[label] = np.abs(f[label] - med).max(-1) < 40
            if name in TOPPED:
                f, s = top_rim(f, s, *shut)
            keyed[t] = cut(f, s, *shut, solo=name in SOLO, walled=name in WALLED)
        loop = [keyed[t] for t in range(i, j)]
        # The end fades into the frames just before the start, so it arrives on frame i
        for t in range(FADE):
            w = (t + 1) / (FADE + 1)
            loop[-FADE + t] = (1 - w) * keyed[j - FADE + t] + w * keyed[i - FADE + t]
        # One square around everything the pose covers in the loop
        ys, xs = np.nonzero(np.any([f[..., 3] > 0.5 for f in loop], axis=0))
        side = max(xs.max() - xs.min(), ys.max() - ys.min()) + 1 + 2 * PAD
        x0 = (xs.min() + xs.max() + 1 - side) // 2
        y0 = (ys.min() + ys.max() + 1 - side) // 2
        squares = []
        for f in loop:
            px = np.dstack([f[..., :3], f[..., 3:] * 255]).round().clip(0, 255).astype(np.uint8)
            sq = Image.new('RGBa', (side, side), (0, 0, 0, 0))
            sq.paste(Image.fromarray(px, 'RGBa'), (-x0, -y0))
            squares.append(sq)
        path = OUT / f'{name}.gif'
        save_gif([q.resize((GIF, GIF), Image.LANCZOS).convert('RGBA') for q in squares], path)
        frames = [q.resize((STRIP, STRIP), Image.LANCZOS).convert('RGBA') for q in squares[::STEP]]
        strip = Image.new('RGBA', (STRIP * len(frames), STRIP), (0, 0, 0, 0))
        for n, fr in enumerate(frames):
            strip.paste(fr, (n * STRIP, 0))
        png = ROOT / 'assets' / f'{name}.png'
        strip.quantize(colors=256, method=Image.Quantize.FASTOCTREE).save(png, optimize=True)
        print(f'{path.relative_to(ROOT).as_posix()}: clip frames {i}-{j - 1}, {side}px square, '
              f'{len(squares)} frames, {path.stat().st_size // 1024} KB; '
              f'strip {len(frames)} frames, {png.stat().st_size // 1024} KB')


if __name__ == '__main__':
    v = frames(sys.argv[1])
    search(v) if '--search' in sys.argv else build(v)
