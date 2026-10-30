"""Summarize repeated depth-only A/B windows; raw samples remain in JSON."""
import json
import statistics as stats
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
data = json.loads((ROOT / "target/depth-benchmark.json").read_text())
assert len(data) == 16, "Benchmark must complete all 16 phases"


def percentile(values, fraction):
    return sorted(values)[min(len(values) - 1, int(len(values) * fraction))]


def compare(rows, series):
    medians = [stats.median(series(row)) for row in rows]
    before = stats.median([v for v, row in zip(medians, rows) if not row["after"]])
    after = stats.median([v for v, row in zip(medians, rows) if row["after"]])
    pairs = []
    for i in range(0, len(rows), 2):
        a, b = medians[i:i+2]
        pairs.append(a - b if rows[i]["after"] else b - a)
    return dict(before_ms=before, after_ms=after, delta_ms=after-before,
                change_percent=(after/before-1)*100 if before else None,
                paired_deltas_ms=pairs, phase_medians_ms=medians,
                counts=[len(series(row)) for row in rows])


summary = {}
for mode in ("raster", "rt"):
    rows = [row for row in data if row["mode"] == mode]
    paths = set.intersection(*(set(row["gpu_ms"]) for row in rows))
    summary[mode] = {
        "frame": compare(rows, lambda row: row["frame_ms"]),
        "phase_p95_frame_ms": [percentile(row["frame_ms"], .95) for row in rows],
        "gpu_passes": {path: compare(rows, lambda row: row["gpu_ms"][path]) for path in sorted(paths)},
    }
    print(mode, json.dumps(summary[mode]["frame"], indent=2))
    for path, result in sorted(summary[mode]["gpu_passes"].items(), key=lambda item: -item[1]["before_ms"]):
        if result["before_ms"] > .04:
            print(f'  {path}: {result["before_ms"]:.4f} -> {result["after_ms"]:.4f} ms ({result["change_percent"]:+.2f}%)')
(ROOT / "target/depth-summary.json").write_text(json.dumps(summary, indent=2))
