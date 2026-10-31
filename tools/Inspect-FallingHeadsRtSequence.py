"""Read-only screen-space flicker heuristic; moving actors still need visual review."""
from pathlib import Path
import json,re
from datetime import datetime
import numpy as np
from PIL import Image
r=Path(__file__).resolve().parents[1]
log=(r/'target/rt-sequence-err.log').read_text(encoding='utf-8',errors='replace')
stamp=re.search(r'\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d+Z',log)
assert stamp,'Runtime log must contain its start timestamp'
cutoff=datetime.fromisoformat(stamp[0]).timestamp()-2
report=[]
for stage in range(5):
 files=sorted(p for p in (r/'target').glob(f'visual-flicker-{stage}-*.png') if p.stat().st_mtime>=cutoff)
 if not files:continue
 frames=np.stack([np.asarray(Image.open(p).convert('RGB'),dtype=np.float32).mean(axis=2)[80:-80,180:-180] for p in files])
 median=np.median(frames,axis=0)
 gy,gx=np.gradient(median)
 mask=(median>45)&(np.abs(gx)+np.abs(gy)<12)
 counts=(((frames<median*.35)&(median-frames>30))&mask).sum(axis=(1,2))
 top=sorted(zip(counts.tolist(),files),reverse=True)[:3]
 report.append({'stage':stage,'frames':len(files),'median_dark_outlier_pixels':float(np.median(counts)),
                'max_dark_outlier_pixels':int(counts.max()),'worst':[{'pixels':n,'file':str(p)} for n,p in top]})
print(json.dumps(report,indent=2))
(r/'target/rt-sequence-analysis.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
