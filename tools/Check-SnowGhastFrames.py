"""Check the real rendered ghast face around native mouth transitions."""
import json
from pathlib import Path
from PIL import Image
root = Path(__file__).resolve().parents[1]
records = json.loads((root / 'target/snowghast-captures.json').read_text())
results = []
for record in records:
    image = Image.open(root / f"target/visual-snowghast-{record['name']}.png").convert('RGB')
    x0, y0, x1, y1 = [round(v) for v in record['box']]
    assert 0 <= x0 < x1 <= image.width and 0 <= y0 < y1 <= image.height
    pixels = [image.getpixel((x,y)) for y in range(y0,y1) for x in range(x0,x1)]
    brightness = [sum(pixel)/3 for pixel in pixels]
    bright = sum(value > 110 for value in brightness)/len(pixels)
    results.append(dict(name=record['name'],open=record['open'],bright_fraction=round(bright,4),mean=round(sum(brightness)/len(brightness),2)))
(root / 'target/snowghast-pixels.json').write_text(json.dumps(results,indent=2))
print(json.dumps(results,indent=2))
assert len(results) == 24
assert all(r['bright_fraction'] > 0.30 for r in results), 'A sampled ghast transition lost the light face'
print('PASS: all 24 sampled transition frames retain the rendered light face')
