#!/usr/bin/env python3
"""Generate the animated README demo GIF.

This is a docs-only asset generator. It avoids adding video tooling or
runtime dependencies to the app; Pillow is enough to draw a compact,
repeatable walkthrough of the current UI.
"""

from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont


ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "docs" / "media" / "rusty-requester-demo.gif"
ICON = ROOT / "assets" / "icon.png"
FONT = ROOT / "assets" / "Inter-Light.ttf"

W, H = 960, 540

BG = "#0f1116"
PANEL = "#171a21"
PANEL_2 = "#1f2430"
BORDER = "#333946"
TEXT = "#eef2f8"
MUTED = "#9aa4b2"
DIM = "#6f7787"
ACCENT = "#e9563d"
ACCENT_DARK = "#81352a"
GREEN = "#78b16c"
GREEN_BG = "#243326"
AMBER = "#f0a31a"
AMBER_BG = "#392b12"
BLUE = "#7aa2f7"
RED = "#f15b5b"


def font(size: int) -> ImageFont.FreeTypeFont | ImageFont.ImageFont:
    try:
        return ImageFont.truetype(str(FONT), size=size)
    except OSError:
        return ImageFont.load_default()


F10 = font(10)
F11 = font(11)
F12 = font(12)
F13 = font(13)
F14 = font(14)
F16 = font(16)
F18 = font(18)
F20 = font(20)
F24 = font(24)
F30 = font(30)


def rounded(draw: ImageDraw.ImageDraw, box, radius, fill, outline=None, width=1):
    draw.rounded_rectangle(box, radius=radius, fill=fill, outline=outline, width=width)


def text(draw: ImageDraw.ImageDraw, xy, value, fill=TEXT, f=F14, anchor=None):
    draw.text(xy, value, fill=fill, font=f, anchor=anchor)


def pill(draw: ImageDraw.ImageDraw, x, y, label, fill, fg, w=None):
    width = w or max(54, int(draw.textlength(label, font=F12)) + 24)
    rounded(draw, (x, y, x + width, y + 24), 7, fill, None)
    text(draw, (x + width / 2, y + 12), label, fg, F12, "mm")
    return width


def progress(a: int, b: int, t: float) -> int:
    return int(a + (b - a) * t)


def truncate(draw: ImageDraw.ImageDraw, value: str, max_width: int, f=F14) -> str:
    if draw.textlength(value, font=f) <= max_width:
        return value
    suffix = "..."
    while value and draw.textlength(value + suffix, font=f) > max_width:
        value = value[:-1]
    return value + suffix


def header(draw: ImageDraw.ImageDraw, active: int = 1):
    rounded(draw, (0, 0, W, H), 18, BG, BORDER, 2)
    draw.rectangle((0, 0, W, 34), fill="#11141a")
    for i, color in enumerate(["#ff5f57", "#febc2e", "#28c840"]):
        draw.ellipse((17 + i * 20, 12, 29 + i * 20, 24), fill=color)

    tabs = [
        ("GET", "Reachable Travel...", GREEN_BG, GREEN),
        ("GET", "Summary", GREEN_BG, GREEN),
        ("POST", "Newsletter cron ...", AMBER_BG, AMBER),
    ]
    x = 292
    for idx, (method, title, fill, fg) in enumerate(tabs):
        if idx == active:
            draw.rectangle((x - 10, 34, x + 202, 66), fill="#151922")
            draw.line((x - 10, 65, x + 202, 65), fill=ACCENT, width=2)
        pill(draw, x, 43, method, fill, fg)
        text(draw, (x + 70, 48), title, MUTED if idx != active else TEXT, F14)
        x += 218
    text(draw, (920, 50), "+", MUTED, F24, "mm")


def sidebar(im: Image.Image, draw: ImageDraw.ImageDraw):
    draw.rectangle((0, 34, 250, H), fill=PANEL)
    draw.line((250, 34, 250, H), fill=BORDER, width=1)
    icon = Image.open(ICON).convert("RGBA").resize((34, 34))
    im.paste(icon, (18, 48), icon)
    text(draw, (60, 55), "Rusty Requester", TEXT, F18)
    text(draw, (60, 79), "v0.24.0", DIM, F10)
    text(draw, (20, 116), "ENVIRONMENT", MUTED, F11)
    rounded(draw, (20, 133, 230, 160), 9, PANEL_2, BORDER)
    text(draw, (34, 140), "No environment", TEXT, F13)
    text(draw, (210, 139), "v", MUTED, F16)
    rounded(draw, (20, 178, 115, 204), 6, ACCENT_DARK, None)
    text(draw, (67, 191), "Collections", TEXT, F12, "mm")
    text(draw, (148, 187), "History", MUTED, F12)
    rounded(draw, (20, 222, 230, 252), 7, ACCENT, None)
    text(draw, (125, 237), "+ New Collection", TEXT, F13, "mm")
    rounded(draw, (20, 268, 230, 294), 8, "#151922", BORDER)
    text(draw, (35, 274), "Search requests", DIM, F13)

    y = 326
    for section in ["My Requests", "Personal", "Slack"]:
        text(draw, (24, y), "v", MUTED, F13)
        text(draw, (42, y - 1), section, TEXT, F13)
        text(draw, (215, y - 1), "+", MUTED, F16)
        y += 30
        for method, label in [
            ("GET", "subscription featured agents get"),
            ("POST", "Post featured agents"),
            ("DELETE", "Flush featured agents"),
        ][: 2 if section != "Personal" else 3]:
            fill = GREEN if method == "GET" else AMBER if method == "POST" else RED
            text(draw, (48, y), method, fill, F10)
            text(draw, (92, y - 3), truncate(draw, label, 126, F12), TEXT, F12)
            y += 28
        y += 10


def request_editor(draw: ImageDraw.ImageDraw, send_progress: float = 0.0):
    rounded(draw, (270, 92, 934, 142), 12, "#10131a", BORDER, 1)
    pill(draw, 287, 105, "GET", GREEN_BG, GREEN, 68)
    url = "https://api.example.com/v1/mobile/version-check?platform=android&version=5.2.1"
    text(draw, (370, 111), truncate(draw, url, 410, F14), TEXT, F14)
    rounded(draw, (816, 103, 875, 132), 9, ACCENT, None)
    text(draw, (846, 118), "Send", TEXT, F13, "mm")
    rounded(draw, (884, 103, 920, 132), 9, PANEL_2, BORDER)
    text(draw, (902, 118), "</>", TEXT, F11, "mm")
    if send_progress:
        draw.line((287, 139, progress(287, 816, send_progress), 139), fill=ACCENT, width=2)

    tabs = ["Params (2)", "Headers", "Cookies", "Body", "Auth", "Tests"]
    x = 280
    for idx, tab in enumerate(tabs):
        if idx == 0:
            rounded(draw, (x - 8, 160, x + 66, 188), 6, ACCENT_DARK, None)
            text(draw, (x + 28, 174), tab, TEXT, F12, "mm")
            draw.line((x + 2, 187, x + 58, 187), fill=ACCENT, width=2)
            x += 94
        else:
            text(draw, (x, 166), tab, MUTED, F12)
            x += 84

    rounded(draw, (270, 200, 934, 360), 9, "#10131a", BORDER)
    text(draw, (287, 218), "Query Params", MUTED, F11)
    text(draw, (380, 248), "KEY", MUTED, F10)
    text(draw, (585, 248), "VALUE", MUTED, F10)
    text(draw, (785, 248), "DESCRIPTION", MUTED, F10)
    for row, key, value in [(0, "platform", "android"), (1, "version", "5.2.1")]:
        y = 270 + row * 38
        draw.line((287, y - 10, 916, y - 10), fill=BORDER, width=1)
        rounded(draw, (292, y - 1, 318, y + 25), 6, ACCENT_DARK, ACCENT)
        text(draw, (305, y + 12), "✓", TEXT, F12, "mm")
        text(draw, (340, y + 4), key, TEXT, F13)
        text(draw, (520, y + 4), value, TEXT, F13)
    draw.line((287, 346, 916, 346), fill=BORDER, width=1)


def response_panel(draw: ImageDraw.ImageDraw, mode: str = "empty"):
    draw.line((270, 374, 934, 374), fill=BORDER, width=1)
    text(draw, (272, 394), "Response", MUTED, F12)
    if mode == "empty":
        text(draw, (600, 456), "No response yet", TEXT, F20, "mm")
        text(draw, (600, 485), "Send a request to see status, headers, and body.", MUTED, F12, "mm")
        return

    pill(draw, 366, 388, "200 OK", GREEN_BG, GREEN, 64)
    text(draw, (440, 394), "216 ms", MUTED, F12)
    text(draw, (500, 394), "7.6 KB", MUTED, F12)
    rounded(draw, (270, 424, 934, 520), 9, PANEL, BORDER)
    lines = [
        '{',
        '  "ok": true,',
        '  "platform": "android",',
        '  "latest_version": "5.2.1",',
        '  "force_update": false',
        '}',
    ]
    y = 442
    for line in lines:
        color = BLUE if '"' in line else TEXT
        text(draw, (292, y), line, color, F13)
        y += 14


def runner_view(draw: ImageDraw.ImageDraw):
    rounded(draw, (270, 92, 934, 508), 12, "#10131a", BORDER)
    text(draw, (294, 120), "Collection Runner", TEXT, F24)
    text(draw, (294, 150), "Run folders with saved presets and inspect every result.", MUTED, F13)
    rounded(draw, (730, 113, 914, 148), 9, ACCENT, None)
    text(draw, (822, 130), "Run preset: Smoke API", TEXT, F13, "mm")
    for i, (name, status, dur) in enumerate(
        [
            ("GET /version-check", "200", "216 ms"),
            ("GET /search/listings", "200", "342 ms"),
            ("POST /newsletter/manual", "201", "505 ms"),
            ("GET /widgets", "200", "184 ms"),
        ]
    ):
        y = 190 + i * 52
        fill = PANEL if i != 1 else "#202633"
        rounded(draw, (294, y, 914, y + 40), 8, fill, BORDER)
        pill(draw, 310, y + 8, "GET" if i != 2 else "POST", GREEN_BG if i != 2 else AMBER_BG, GREEN if i != 2 else AMBER, 58)
        text(draw, (382, y + 11), name, TEXT, F13)
        pill(draw, 774, y + 8, status, GREEN_BG, GREEN, 52)
        text(draw, (848, y + 12), dur, MUTED, F12)

    rounded(draw, (294, 414, 914, 488), 8, PANEL_2, BORDER)
    text(draw, (314, 432), "Result detail", TEXT, F14)
    text(draw, (314, 456), "Assertions passed, extractor values redacted, safe summary ready to copy.", MUTED, F12)


def compare_view(draw: ImageDraw.ImageDraw):
    rounded(draw, (270, 92, 934, 508), 12, "#10131a", BORDER)
    text(draw, (294, 120), "Environment Compare", TEXT, F24)
    text(draw, (294, 150), "Review variable drift without leaking secrets.", MUTED, F13)
    for i, (title, count, color) in enumerate(
        [("Added", "3", GREEN), ("Changed", "2", AMBER), ("Missing", "1", RED), ("Unchanged", "12", MUTED)]
    ):
        x = 294 + i * 152
        rounded(draw, (x, 188, x + 128, 244), 8, PANEL_2, BORDER)
        text(draw, (x + 16, 204), title, MUTED, F12)
        text(draw, (x + 16, 220), count, color, F20)

    rows = [
        ("API_BASE_URL", "https://staging.example.com", "https://api.example.com"),
        ("ACCESS_TOKEN", "••••••••••••", "••••••••••••"),
        ("TIMEOUT_SECONDS", "30", "60"),
    ]
    y = 282
    text(draw, (310, y), "KEY", MUTED, F10)
    text(draw, (500, y), "SOURCE", MUTED, F10)
    text(draw, (704, y), "TARGET", MUTED, F10)
    y += 24
    for key, source, target in rows:
        rounded(draw, (294, y, 914, y + 38), 7, PANEL, BORDER)
        text(draw, (310, y + 10), key, TEXT, F12)
        text(draw, (500, y + 10), source, MUTED, F12)
        text(draw, (704, y + 10), target, MUTED, F12)
        y += 48
    rounded(draw, (714, 452, 914, 486), 8, ACCENT, None)
    text(draw, (814, 469), "Copy safe summary", TEXT, F13, "mm")


def share_view(draw: ImageDraw.ImageDraw):
    rounded(draw, (270, 92, 934, 508), 12, "#10131a", BORDER)
    text(draw, (294, 120), "Safe Sharing", TEXT, F24)
    text(draw, (294, 150), "Redacted snippets and local export scanning are free.", MUTED, F13)
    rounded(draw, (294, 190, 914, 362), 9, PANEL, BORDER)
    code = [
        "curl -X POST https://api.example.com/v1/users \\",
        "  -H 'Authorization: Bearer [REDACTED]' \\",
        "  -H 'Cookie: session=[REDACTED]' \\",
        "  -d '{\"email\":\"team@example.com\",\"api_key\":\"[REDACTED]\"}'",
    ]
    y = 214
    for line in code:
        text(draw, (318, y), line, BLUE if "[REDACTED]" in line else TEXT, F13)
        y += 28
    rounded(draw, (294, 394, 914, 452), 9, "#211917", "#654036")
    text(draw, (318, 414), "Secret scanner found likely sensitive values before export.", TEXT, F13)
    text(draw, (318, 434), "Cancel, export original, or write a redacted copy.", MUTED, F12)
    rounded(draw, (742, 466, 914, 496), 8, ACCENT, None)
    text(draw, (828, 481), "Copy redacted", TEXT, F13, "mm")


def frame(scene: str, step: int) -> Image.Image:
    im = Image.new("RGB", (W, H), BG)
    draw = ImageDraw.Draw(im)
    header(draw)
    sidebar(im, draw)

    if scene == "request":
        request_editor(draw, step / 8)
        response_panel(draw, "empty" if step < 5 else "json")
        if step < 4:
            text(draw, (602, 74), "Build and send HTTP requests", TEXT, F18, "mm")
        else:
            text(draw, (602, 74), "Inspect response status, timing, and body", TEXT, F18, "mm")
    elif scene == "runner":
        runner_view(draw)
        text(draw, (602, 74), "Run collections with reusable presets", TEXT, F18, "mm")
    elif scene == "compare":
        compare_view(draw)
        text(draw, (602, 74), "Compare environments safely", TEXT, F18, "mm")
    else:
        share_view(draw)
        text(draw, (602, 74), "Share without leaking secrets", TEXT, F18, "mm")

    return im


def main() -> None:
    OUT.parent.mkdir(parents=True, exist_ok=True)
    frames = []
    durations = []
    for scene, count in [("request", 12), ("runner", 9), ("compare", 9), ("share", 10)]:
        for step in range(count):
            frames.append(frame(scene, step).convert("P", palette=Image.Palette.ADAPTIVE, colors=128))
            durations.append(130)
    frames[0].save(
        OUT,
        save_all=True,
        append_images=frames[1:],
        duration=durations,
        loop=0,
        optimize=True,
        disposal=2,
    )
    print(f"wrote {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
