"""Builds assets/social-preview.png (1280x640), the image GitHub shows when the repo link is shared.

Upload it by hand: repo Settings > General > Social preview. GitHub has no API for it.
"""
from PIL import Image, ImageDraw, ImageFont

W, H = 1280, 640
FRAME = {"loading": 20, "working": 30, "asking": 20, "completed": 30}

bg = Image.new("RGBA", (W, H), (22, 27, 34, 255))
d = ImageDraw.Draw(bg)

def font(size, bold=False):
    for name in ("segoeuib.ttf" if bold else "segoeui.ttf", "arial.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            pass
    return ImageFont.load_default()

d.text((W // 2, 120), "ai-mascot-hook", font=font(84, True), fill="white", anchor="mm")
d.text((W // 2, 195), "A desktop seal that reacts to your AI coding agent", font=font(34), fill=(160, 170, 185), anchor="mm")

size, gap = 240, 40
x = (W - (4 * size + 3 * gap)) // 2
for name, idx in FRAME.items():
    gif = Image.open(f"assets/gif/{name}.gif")
    gif.seek(idx)
    img = gif.convert("RGBA").resize((size, size), Image.NEAREST)
    bg.alpha_composite(img, (x, 280))
    d.text((x + size // 2, 560), name, font=font(28), fill=(160, 170, 185), anchor="mm")
    x += size + gap

bg.convert("RGB").save("assets/social-preview.png", optimize=True)
