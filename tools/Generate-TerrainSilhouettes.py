"""Regenerate projectile silhouettes after replacing the bundled Faithful sprites.

Requires Pillow. The 0.4 alpha threshold matches biome_content/art.rs.
Plants remain pass-through; only solid decorative hazards need these masks.
"""
from pathlib import Path
from PIL import Image

root = Path(__file__).resolve().parent.parent
names = ["pointed_dripstone_up_tip", "pointed_dripstone_down_tip", "amethyst_cluster"]
out = [
    "// Generated from bundled Faithful alpha >= 102 (the rendering cutoff).",
    "// Row runs preserve transparent corners instead of using a square hitbox.",
]
for name in names:
    im = Image.open(root / f"assets/textures/biome_content/{name}.png").convert("RGBA")
    width, height = im.size
    out.append(f"pub(super) const {name.upper()}: &[[f32;4]] = &[")
    for y in range(height):
        x = 0
        while x < width:
            if im.getpixel((x, y))[3] < 102:
                x += 1
                continue
            start = x
            while x < width and im.getpixel((x, y))[3] >= 102:
                x += 1
            rectangle = (start * 64 / width - 32, 32 - (y + 1) * 64 / height,
                         x * 64 / width - 32, 32 - y * 64 / height)
            out.append("    [" + ", ".join(f"{v:.1f}" for v in rectangle) + "],")
    out.append("];")
(root / "src/systems/terrain_silhouettes.rs").write_text("\n".join(out) + "\n", encoding="utf8")
