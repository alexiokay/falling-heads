"""Exercise the real exporter across biomes and seeds; save reusable layouts."""
from pathlib import Path
import subprocess,json,time,statistics
ROOT=Path(__file__).resolve().parents[2]
EXE=ROOT/'target/debug/falling-heads.exe'
OUT=ROOT/'design/world-previews'
BIOMES=['Plains','Forest','Desert','Mountains','Underground','Nether','End','Iceland']
def generate(biome,seed,path):
    start=time.perf_counter()
    subprocess.run([str(EXE),'--export-world-preview','--biome',biome,'--seed',str(seed),'--x','96','--width','160','--height','112','--output',str(path)],cwd=ROOT,check=True,capture_output=True,timeout=60,creationflags=subprocess.CREATE_NO_WINDOW)
    data=json.loads(path.read_text());elapsed=(time.perf_counter()-start)*1000
    return data,elapsed
def identity(data):return {k:v for k,v in data.items() if k not in ('generation_ms','total_ms')}
if __name__=='__main__':
    OUT.mkdir(parents=True,exist_ok=True);report=[]
    for biome in BIOMES:
        previous=None
        for seed in [7,81,921]:
            data,elapsed=generate(biome,seed,OUT/f'{biome.lower()}-{seed}.json')
            assert len(data['cells'])==len({tuple(c[:2]) for c in data['cells']})
            assert 0.08<len(data['cells'])/(data['width']*data['height'])<0.95,(biome,seed,'bad air balance')
            if previous is not None:assert data['cells']!=previous,'Seed did not change terrain'
            previous=data['cells']
            if seed==921:
                repeated,_=generate(biome,seed,ROOT/'target/preview-repeat.json')
                assert identity(data)==identity(repeated),'Non-deterministic static world'
            report.append({'biome':biome,'seed':seed,'cells':len(data['cells']),'landmarks':len(data['landmarks']),'tree_cells':sum(c[4] is not None and ('_log' in c[4] or '_leaves' in c[4]) for c in data['cells']),'generation_ms':round(data['generation_ms'],2),'including_startup_ms':round(elapsed,2)})
    summary={'samples':len(report),'repeat_checks':8,'median_generation_ms':round(statistics.median(r['generation_ms'] for r in report),2),'max_generation_ms':max(r['generation_ms'] for r in report),'median_including_startup_ms':round(statistics.median(r['including_startup_ms'] for r in report),2),'results':report}
    (OUT/'verification.json').write_text(json.dumps(summary,indent=2),encoding='utf-8')
    print(json.dumps({k:v for k,v in summary.items() if k!='results'},indent=2))
