#!/usr/bin/env python3
"""Render the Rusty Requester app icon from ``assets/icon.svg``.

Writes the canonical ``assets/icon.png`` at 1024×1024. The SVG is the
single source of truth — editing the icon means editing the SVG. This
script just rasterises it (with transparency preserved) so the Rust
binary has the PNG bytes to embed and the Makefile bundle step has the
input `sips` downscales for the iconset.

Dependencies: ``pip install --user resvg-py`` (pure-Rust renderer, no
libcairo / ImageMagick). Falls back to ``sips`` on macOS via CLI if
resvg-py isn't available.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SVG = ROOT / "assets" / "icon.svg"
PNG = ROOT / "assets" / "icon.png"
SIZE = 1024


def render_with_resvg() -> bool:
    try:
        import resvg_py  # type: ignore
    except ImportError:
        return False
    data = resvg_py.svg_to_bytes(svg_path=str(SVG), width=SIZE, height=SIZE)
    PNG.write_bytes(bytes(data))
    return True


def render_with_qlmanage() -> bool:
    # `qlmanage` rasterises onto a white backdrop (no alpha) — acceptable
    # as a last resort but not ideal. Printed as a warning so the user
    # installs resvg-py for the transparent render.
    tmp = ROOT / "assets" / "_ql_thumb"
    tmp.mkdir(exist_ok=True)
    try:
        subprocess.run(
            ["qlmanage", "-t", "-s", str(SIZE), str(SVG), "-o", str(tmp)],
            check=True,
            capture_output=True,
        )
        thumb = tmp / f"{SVG.name}.png"
        if not thumb.exists():
            return False
        thumb.replace(PNG)
        return True
    except Exception:
        return False
    finally:
        for leftover in tmp.glob("*"):
            leftover.unlink(missing_ok=True)
        tmp.rmdir()


def main() -> int:
    if not SVG.exists():
        print(f"error: {SVG} not found", file=sys.stderr)
        return 1
    if render_with_resvg():
        print(f"Rendered {PNG} ({SIZE}×{SIZE}) via resvg-py — transparent PNG.")
        return 0
    if render_with_qlmanage():
        print(
            f"Rendered {PNG} ({SIZE}×{SIZE}) via qlmanage.\n"
            "warning: transparent corners were filled with white. Install\n"
            "  pip install --user resvg-py\n"
            "to get a proper transparent PNG."
        )
        return 0
    print(
        "error: no SVG renderer available.\n"
        "  pip install --user resvg-py   (pure-Rust, recommended)\n"
        "  brew install librsvg           (and pipe `rsvg-convert`)",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
