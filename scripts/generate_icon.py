#!/usr/bin/env python3
"""Generate the Rusty Requester app icon as a PNG.

Produces a 512x512 PNG at assets/icon.png. The icon is a rounded dark square
(Tokyo Night palette) with a stylized "R" and an accent send-arrow underline.
"""

from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

SIZE = 512

# Tokyo Night palette
PANEL = (26, 29, 41, 255)
ELEVATED = (36, 40, 59, 255)
BORDER = (47, 53, 73, 255)
ACCENT = (122, 162, 247, 255)
GREEN = (158, 206, 106, 255)
ORANGE = (224, 175, 104, 255)

FONT_CANDIDATES = [
    "/System/Library/Fonts/Helvetica.ttc",
    "/System/Library/Fonts/SFNS.ttf",
    "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
    "/Library/Fonts/Arial Bold.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    "/usr/share/fonts/dejavu/DejaVuSans-Bold.ttf",
    "C:/Windows/Fonts/arialbd.ttf",
]


def load_bold_font(size: int) -> ImageFont.ImageFont:
    for path in FONT_CANDIDATES:
        try:
            return ImageFont.truetype(path, size)
        except Exception:
            continue
    return ImageFont.load_default()


def draw_vertical_gradient(img: Image.Image, top: tuple[int, int, int, int],
                            bottom: tuple[int, int, int, int]) -> None:
    """Fill img with a vertical gradient in-place."""
    w, h = img.size
    grad = Image.new("RGBA", (1, h))
    for y in range(h):
        t = y / max(h - 1, 1)
        r = round(top[0] * (1 - t) + bottom[0] * t)
        g = round(top[1] * (1 - t) + bottom[1] * t)
        b = round(top[2] * (1 - t) + bottom[2] * t)
        a = round(top[3] * (1 - t) + bottom[3] * t)
        grad.putpixel((0, y), (r, g, b, a))
    img.paste(grad.resize((w, h)), (0, 0))


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    out_dir = root / "assets"
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / "icon.png"

    # Transparent base
    img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))

    # Build a rounded-rectangle mask
    mask = Image.new("L", (SIZE, SIZE), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        (0, 0, SIZE, SIZE), radius=SIZE // 7, fill=255
    )

    # Gradient background clipped to the rounded mask
    bg = Image.new("RGBA", (SIZE, SIZE), PANEL)
    draw_vertical_gradient(bg, (30, 34, 50, 255), (22, 25, 37, 255))
    img.paste(bg, (0, 0), mask)

    draw = ImageDraw.Draw(img)

    # Inner border for a subtle bevel
    inset = 6
    draw.rounded_rectangle(
        (inset, inset, SIZE - inset, SIZE - inset),
        radius=SIZE // 7 - inset,
        outline=BORDER,
        width=3,
    )

    # Centered bold "R"
    font = load_bold_font(int(SIZE * 0.58))
    text = "R"
    bbox = draw.textbbox((0, 0), text, font=font)
    tw = bbox[2] - bbox[0]
    th = bbox[3] - bbox[1]
    tx = (SIZE - tw) // 2 - bbox[0]
    ty = (SIZE - th) // 2 - bbox[1] - int(SIZE * 0.04)
    # Drop shadow
    draw.text((tx + 3, ty + 5), text, fill=(0, 0, 0, 120), font=font)
    # Main glyph
    draw.text((tx, ty), text, fill=ACCENT, font=font)

    # Accent "send" arrow beneath the R
    arrow_y = int(SIZE * 0.82)
    arrow_start = int(SIZE * 0.32)
    arrow_end = int(SIZE * 0.68)
    line_width = max(SIZE // 50, 6)
    draw.line(
        [(arrow_start, arrow_y), (arrow_end, arrow_y)],
        fill=GREEN,
        width=line_width,
    )
    head = SIZE // 28
    draw.polygon(
        [
            (arrow_end + head, arrow_y),
            (arrow_end - head // 2, arrow_y - head),
            (arrow_end - head // 2, arrow_y + head),
        ],
        fill=GREEN,
    )

    img.save(out_path, format="PNG")
    print(f"wrote {out_path} ({SIZE}x{SIZE})")


if __name__ == "__main__":
    main()
