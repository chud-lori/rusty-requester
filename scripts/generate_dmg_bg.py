#!/usr/bin/env python3
"""Generate the DMG installer background art.

Produces a 600x400 PNG at assets/dmg_background.png. The background is a
Tokyo-Night-styled gradient with the app name, a subtitle, and a dashed
accent arrow between where Finder will place the .app and the Applications
shortcut.

Layout the AppleScript in scripts/make_dmg.sh expects:
  window bounds : 600 x 400
  app icon      : centred at (175, 200)
  Applications  : centred at (425, 200)
  icon size     : 128
"""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

W, H = 600, 400
# Rust-forge palette — warm coppers + rust orange.
GRAD_TOP = (48, 28, 17, 255)       # #301C11 warm copper top
GRAD_BOT = (18, 11, 7, 255)        # #120B07 deep warm bottom
ACCENT = (206, 66, 43, 255)        # #CE422B rust orange
AMBER = (245, 158, 11, 255)        # #F59E0B amber arrow tip
TEXT = (245, 230, 208, 255)        # #F5E6D0 warm cream title
MUTED = (170, 140, 115, 255)       # #AA8C73 warm muted subtitle

FONT_CANDIDATES = [
    "/System/Library/Fonts/Helvetica.ttc",
    "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
    "/Library/Fonts/Arial Bold.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
]


def load_font(size: int) -> ImageFont.ImageFont:
    for p in FONT_CANDIDATES:
        try:
            return ImageFont.truetype(p, size)
        except Exception:
            continue
    return ImageFont.load_default()


def vertical_gradient(img: Image.Image, top, bot) -> None:
    w, h = img.size
    grad = Image.new("RGBA", (1, h))
    for y in range(h):
        t = y / max(h - 1, 1)
        r = round(top[0] * (1 - t) + bot[0] * t)
        g = round(top[1] * (1 - t) + bot[1] * t)
        b = round(top[2] * (1 - t) + bot[2] * t)
        a = round(top[3] * (1 - t) + bot[3] * t)
        grad.putpixel((0, y), (r, g, b, a))
    img.paste(grad.resize((w, h)), (0, 0))


def draw_centered(draw: ImageDraw.ImageDraw, text: str, y: int,
                  font: ImageFont.ImageFont, fill) -> None:
    bbox = draw.textbbox((0, 0), text, font=font)
    tw = bbox[2] - bbox[0]
    draw.text(((W - tw) // 2, y), text, fill=fill, font=font)


def main() -> None:
    out_dir = Path(__file__).resolve().parent.parent / "assets"
    out_dir.mkdir(parents=True, exist_ok=True)
    out = out_dir / "dmg_background.png"

    img = Image.new("RGBA", (W, H), GRAD_BOT)
    vertical_gradient(img, GRAD_TOP, GRAD_BOT)
    draw = ImageDraw.Draw(img)

    # Title
    draw_centered(draw, "Rusty Requester", 28, load_font(30), TEXT)
    # Subtitle
    draw_centered(draw, "Drag the app onto the Applications folder",
                  78, load_font(13), MUTED)

    # Dashed arrow between icon slots — starts rust orange, tip amber.
    arrow_y = 200
    arrow_start = 250
    arrow_end = 348
    dash_w, gap = 10, 6
    x = arrow_start
    idx = 0
    n_dashes = max(1, (arrow_end - arrow_start) // (dash_w + gap))
    while x + dash_w <= arrow_end - 4:
        # lerp from rust orange → amber across the dashes
        t = idx / max(n_dashes - 1, 1)
        r = round(ACCENT[0] * (1 - t) + AMBER[0] * t)
        g = round(ACCENT[1] * (1 - t) + AMBER[1] * t)
        b = round(ACCENT[2] * (1 - t) + AMBER[2] * t)
        draw.line([(x, arrow_y), (x + dash_w, arrow_y)],
                  fill=(r, g, b, 255), width=3)
        x += dash_w + gap
        idx += 1
    head = 10
    draw.polygon([
        (arrow_end + head, arrow_y),
        (arrow_end - head + 4, arrow_y - head),
        (arrow_end - head + 4, arrow_y + head),
    ], fill=AMBER)

    # Bottom hint
    draw_centered(draw, "Then eject this disk image.",
                  H - 48, load_font(11), MUTED)

    img.save(out, format="PNG")
    print(f"wrote {out}  ({W}x{H})")


if __name__ == "__main__":
    main()
