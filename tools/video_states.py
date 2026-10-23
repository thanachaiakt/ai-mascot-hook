"""Cut the four seal states out of the 2 x 2 clip art/states.mp4.

    python tools/video_states.py        (needs ffmpeg on PATH, Pillow, NumPy and SciPy)

Each quadrant's seal is cut away from the blue grid (bubbles, sparkles and splashes go with
it; the sleeping seal keeps its zZ; the swimmer gets back the tail the clip loses mid-spin),
trimmed to a stretch that loops without a jump, cropped to a tight 1:1 square and written:

    assets/gif/<state>.gif       300 x 300, 12.5 fps, transparent
    assets/<state>.png           the pet's strip: 260 x 260 frames, 256 colors with alpha
    assets/previews/<state>.png  the strip's first frame
"""
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage

sys.path.insert(0, str(Path(__file__).resolve().parent))
from seg import QUAD, cut  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / 'art' / 'states.mp4'
STRIP, GIF = 260, 300           # frame sides of the pet's strip and of the GIF
STEP = 2                        # keep every 2nd of the clip's 24 fps
PAD = 6                         # px of clear space around the seal in the square

# The clip frames each state loops over: frame END all but matches frame START, closer than
# two neighbouring frames are, so START follows END - STEP with no jump. Asking waves with
# its left flipper in frames 0-77 and draws the resting left flipper too (three flippers),
# so its loop keeps to the right-flipper waves after that.
LOOP = {'loading': (53, 201), 'working': (6, 195), 'asking': (88, 208), 'completed': (53, 225)}


def zz(clab, sizes, main):
    """The sleeping seal's floating zZ: blobs in the space above its head."""
    out = []
    for i, sl in enumerate(ndimage.find_objects(clab)):
        if sl is None or i + 1 == main:
            continue
        cy, cx = (sl[0].start + sl[0].stop) / 2, (sl[1].start + sl[1].stop) / 2
        if 340 < cx < 470 and 35 < cy < 150 and sizes[i] >= 10:
            out.append(i + 1)
    return out


OPTS = {'completed': dict(keep_extra=zz), 'working': dict(hug=11)}

# The swimmer's tail all but vanishes in clip frames 150-151, upside down mid-spin (it melts
# into the shovel). Frame 152's tail, cut along TAIL_POLY, goes back in behind the body:
# frame -> (where its base sits, degrees clockwise, boxes of leftover spikes to clear).
TAIL_FROM, TAIL_POLY, TAIL_BASE = 152, [(200, 88), (312, 88), (312, 150), (290, 182), (232, 182), (200, 150)], (262, 175)
TAIL_FIX = {
    150: ((300, 162), 14, [(270, 120, 290, 129), (302, 104, 322, 118)]),
    151: ((281, 168), 7, [(244, 84, 272, 104)]),
}


def mend_tail(keyed):
    src = Image.fromarray(keyed[TAIL_FROM], 'RGBA')
    m = Image.new('L', src.size, 0)
    ImageDraw.Draw(m).polygon(TAIL_POLY, fill=255)
    tail = Image.new('RGBA', src.size, (0, 0, 0, 0))
    tail.paste(src, (0, 0), m)
    for i, ((ax, ay), deg, boxes) in TAIL_FIX.items():
        if i not in keyed:
            continue
        a = keyed[i].copy()
        for x0, y0, x1, y1 in boxes:
            a[y0:y1, x0:x1] = 0
        out = tail.rotate(-deg, resample=Image.BICUBIC, center=TAIL_BASE, translate=(ax - TAIL_BASE[0], ay - TAIL_BASE[1]))
        out.alpha_composite(Image.fromarray(a, 'RGBA'))
        keyed[i] = np.asarray(out)


def save_gif(frames, path):
    """RGBA frames -> one transparent GIF with a palette shared by every frame."""
    n = len(frames)
    sheet = np.concatenate([np.asarray(f) for f in frames], axis=1)
    solid = sheet[..., 3] >= 128
    rgb = sheet[..., :3].copy()
    rgb[~solid] = 255                                  # keep the hidden pixels out of the palette
    pal_img = Image.fromarray(rgb).quantize(colors=255, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
    idx = np.asarray(pal_img).copy()
    idx[~solid] = 255
    pal = pal_img.getpalette()[:255 * 3] + [0, 0, 0]
    w = frames[0].width
    out = []
    for i in range(n):
        im = Image.fromarray(idx[:, i * w:(i + 1) * w], 'P')
        im.putpalette(pal)
        im.info['transparency'] = 255
        out.append(im)
    out[0].save(path, save_all=True, append_images=out[1:], duration=round(1000 * STEP / 24, -1),
                loop=0, disposal=2, transparency=255, optimize=False)


def main():
    (ROOT / 'assets' / 'gif').mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        subprocess.run(['ffmpeg', '-v', 'error', '-i', str(SRC), '-an', str(tmp / 'f%04d.png')], check=True)
        paths = sorted(tmp.glob('f*.png'))
        for state, quad in QUAD.items():
            start, end = LOOP[state]
            used = list(range(start, end, STEP))
            need = set(used) | ({TAIL_FROM} if state == 'working' else set())
            keyed = {i: cut(np.asarray(Image.open(paths[i]).convert('RGB').crop(quad)), **OPTS.get(state, {}))[0]
                     for i in sorted(need)}
            if state == 'working':
                mend_tail(keyed)
            loop = [keyed[i] for i in used]
            # one square around everything the seal covers in the loop
            ys, xs = np.nonzero(np.any([f[..., 3] > 0 for f in loop], axis=0))
            side = max(xs.max() - xs.min(), ys.max() - ys.min()) + 1 + 2 * PAD
            x0 = (xs.min() + xs.max() + 1 - side) // 2
            y0 = (ys.min() + ys.max() + 1 - side) // 2
            squares = []
            for f in loop:
                sq = Image.new('RGBA', (side, side), (0, 0, 0, 0))
                sq.paste(Image.fromarray(f, 'RGBA'), (-x0, -y0))
                squares.append(sq.convert('RGBa'))
            def sized(s):
                return [q.resize((s, s), Image.LANCZOS).convert('RGBA') for q in squares]
            save_gif(sized(GIF), ROOT / 'assets' / 'gif' / f'{state}.gif')
            frames = sized(STRIP)
            strip = Image.new('RGBA', (STRIP * len(frames), STRIP), (0, 0, 0, 0))
            for i, fr in enumerate(frames):
                strip.paste(fr, (i * STRIP, 0))
            strip.quantize(colors=256, method=Image.Quantize.FASTOCTREE).save(ROOT / 'assets' / f'{state}.png', optimize=True)
            frames[0].save(ROOT / 'assets' / 'previews' / f'{state}.png', optimize=True)
            kb = lambda p: p.stat().st_size // 1024
            print(f'{state}: clip frames {start}-{end}, {side}px square, '
                  f'{len(frames)} frames, {round(1000 * STEP / 24)} ms each; '
                  f'strip {kb(ROOT / "assets" / f"{state}.png")} KB, gif {kb(ROOT / "assets" / "gif" / f"{state}.gif")} KB')


if __name__ == '__main__':
    main()
