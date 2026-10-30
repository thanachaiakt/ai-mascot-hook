"""Builds assets/icon.ico from one frame of the happy (clapping) strip.
Needs Pillow:  python tools/make_icon.py"""
from PIL import Image

FRAME, SIZE = 20, 260
strip = Image.open("assets/happy.png").convert("RGBA")
seal = strip.crop((FRAME * SIZE, 0, (FRAME + 1) * SIZE, SIZE))
seal = seal.crop(seal.getbbox())
side = max(seal.size)
square = Image.new("RGBA", (side, side), (0, 0, 0, 0))
square.alpha_composite(seal, ((side - seal.width) // 2, (side - seal.height) // 2))
square.save("assets/icon.ico", sizes=[(s, s) for s in (16, 24, 32, 48, 64, 128, 256)])
