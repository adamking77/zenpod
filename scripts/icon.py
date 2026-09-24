"""The Zenpod enso: 30 rays around a thin ring, the played part in the accent, drying out toward the gap.
Rebuilt from the 0.2.0 app-icon.png by measurement. Run `python3 scripts/icon.py <out dir>` for flat SVG tiles, or
`python3 scripts/icon.py --package src-tauri/icons/Zenpod.icon` for the Icon Composer package that gives macOS 26+ a
light and a dark icon. That package only becomes an icon once actool (Xcode 26+) compiles it; see .github/workflows/icon.yml."""

import json
import math
import sys
from pathlib import Path

C = (499.3, 515.5)
RING, RING_W, RAY_W, DOT = 175.1, 11, 12, 19
RAY_IN = 210
START, TURN, END = 32.0, 170.0, 350.7  # degrees clockwise from twelve o'clock
RAYS = [(32.0, 284.7), (42.8, 308.6), (53.8, 313.9), (64.8, 327.4), (75.8, 308.3), (86.8, 267.2), (97.8, 285.6),
        (108.8, 311.4), (119.8, 294.7), (130.8, 297.9), (141.8, 303.5), (152.8, 283.9), (163.8, 280.1), (174.8, 277.0),
        (185.8, 268.9), (196.8, 286.6), (207.5, 285.9), (218.2, 261.9), (229.2, 268.0), (240.2, 272.9), (251.2, 256.0),
        (262.2, 259.9), (273.2, 267.7), (284.2, 259.4), (295.2, 255.1), (306.2, 245.4), (317.2, 234.1), (328.2, 238.7),
        (339.0, 235.7), (349.8, 228.3)]

LOOKS = {
    "dark": {"tile": "#1d1c27", "accent": "#88a6e0", "ink": "#a6a8b4", "fade": (0.68, 0.27)},
    "light": {"tile": "#f4f2ef", "accent": "#355aa0", "ink": "#3f3f48", "fade": (0.62, 0.2)},
}


def at(deg, r):
    a = math.radians(deg)
    return C[0] + r * math.sin(a), C[1] - r * math.cos(a)


def arc(a0, a1, r=RING):
    (x0, y0), (x1, y1) = at(a0, r), at(a1, r)
    return f"M{x0:.2f} {y0:.2f}A{r} {r} 0 {int(a1 - a0 > 180)} 1 {x1:.2f} {y1:.2f}"


def fade(deg, look):
    hi, lo = LOOKS[look]["fade"]
    return hi + (lo - hi) * (deg - TURN) / (END - TURN)


def accent_part(look):
    col = LOOKS[look]["accent"]
    rays = "".join(
        f'<line x1="{at(a, RAY_IN)[0]:.2f}" y1="{at(a, RAY_IN)[1]:.2f}" x2="{at(a, r - 6)[0]:.2f}" y2="{at(a, r - 6)[1]:.2f}"/>'
        for a, r in RAYS if a < TURN
    )
    x, y = at(START, RING)
    return (f'<g stroke="{col}" stroke-linecap="round" fill="none"><g stroke-width="{RAY_W}">{rays}</g>'
            f'<path d="{arc(START, TURN)}" stroke-width="{RING_W}"/></g><circle cx="{x:.2f}" cy="{y:.2f}" r="{DOT}" fill="{col}"/>')


def ink_part(look):
    col = LOOKS[look]["ink"]
    rays = "".join(
        f'<line x1="{at(a, RAY_IN)[0]:.2f}" y1="{at(a, RAY_IN)[1]:.2f}" x2="{at(a, r - 6)[0]:.2f}" y2="{at(a, r - 6)[1]:.2f}" stroke-opacity="{fade(a, look):.3f}"/>'
        for a, r in RAYS if a >= TURN
    )
    # SVG has no conic gradient: the ring dries out in short butt-capped steps, round only at its far end.
    n = 60
    steps = "".join(
        f'<path d="{arc(TURN + (END - TURN) * i / n, TURN + (END - TURN) * (i + 1) / n)}" stroke-opacity="{fade(TURN + (END - TURN) * (i + 0.5) / n, look):.3f}"'
        + (' stroke-linecap="round"' if i == n - 1 else "") + "/>"
        for i in range(n)
    )
    return (f'<g stroke="{col}" fill="none"><g stroke-width="{RAY_W}" stroke-linecap="round">{rays}</g>'
            f'<g stroke-width="{RING_W}">{steps}</g></g>')


def svg(body, tile=None):
    back = f'<rect x="100" y="100" width="824" height="824" rx="185" fill="{tile}"/>' if tile else ""
    return f'<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024">{back}{body}</svg>\n'


def srgb(hex_):
    r, g, b = (int(hex_[i:i + 2], 16) / 255 for i in (1, 3, 5))
    return f"srgb:{r:.5f},{g:.5f},{b:.5f},1.00000"


def package(out: Path):
    """Icon Composer's format: the artwork fills the whole square (macOS masks it). Each layer names a drawing per
    appearance and keeps the drawing's own colours: a layer fill would flatten the fade and fill the open arcs."""
    (out / "Assets").mkdir(parents=True, exist_ok=True)
    # The tile ran 100..924 of 1024; scale the glyph so it keeps its place in a full-bleed square.
    k = 1024 / 824
    bleed = lambda body: svg(f'<g transform="matrix({k:.5f} 0 0 {k:.5f} {-100 * k:.3f} {-100 * k:.3f})">{body}</g>')
    for look in LOOKS:
        (out / "Assets" / f"ink-{look}.svg").write_text(bleed(ink_part(look)))
        (out / "Assets" / f"accent-{look}.svg").write_text(bleed(accent_part(look)))

    def layer(name):
        return {"name": name, "layers": [{"name": name, "glass": False, "image-name-specializations": [
                    {"value": f"{name}-light.svg"}, {"appearance": "dark", "value": f"{name}-dark.svg"}]}],
                "shadow": {"kind": "none", "opacity": 0.5}, "specular": False, "translucency": {"enabled": False, "value": 0.5}}

    tile = [{"value": {"solid": srgb(LOOKS["light"]["tile"])}}, {"appearance": "dark", "value": {"solid": srgb(LOOKS["dark"]["tile"])}}]
    icon = {"fill-specializations": tile, "groups": [layer("accent"), layer("ink")], "supported-platforms": {"squares": ["macOS"]}}
    (out / "icon.json").write_text(json.dumps(icon, indent=2) + "\n")


if __name__ == "__main__":
    if sys.argv[1:2] == ["--package"]:
        package(Path(sys.argv[2]))
        sys.exit()
    out = Path(sys.argv[1] if len(sys.argv) > 1 else ".")
    out.mkdir(parents=True, exist_ok=True)
    for look, c in LOOKS.items():
        (out / f"tile-{look}.svg").write_text(svg(ink_part(look) + accent_part(look), c["tile"]))
        (out / f"accent-{look}.svg").write_text(svg(accent_part(look)))
        (out / f"ink-{look}.svg").write_text(svg(ink_part(look)))
