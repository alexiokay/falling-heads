"""Inventory the user-supplied pack; copy selected originals with provenance."""
from pathlib import Path
from PIL import Image
from collections import Counter
import csv,json,hashlib,shutil
root=Path(__file__).resolve().parents[1]
pack=Path(r'C:\Users\alexispace\Downloads\Faithful 64x - Release 15')
textures=pack/'assets/minecraft/textures'
out=root/'design';out.mkdir(exist_ok=True)
rows=[]
for p in sorted(textures.rglob('*.png')):
    with Image.open(p) as im:w,h=im.size
    rows.append({'asset':p.relative_to(textures).as_posix(),'width':w,'height':h,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
with (out/'faithful64-release15-assets.csv').open('w',newline='',encoding='utf-8') as f:
    writer=csv.DictWriter(f,fieldnames=rows[0].keys());writer.writeheader();writer.writerows(rows)
copies=[]
for folder in ['assets/textures/blocks/forest/species','assets/textures/structures']:
    target=root/folder
    for dst in target.glob('*.png'):
        src=textures/'block'/dst.name
        if not src.exists():continue
        backup=root/'target/pre-faithful15'/folder/dst.name;backup.parent.mkdir(parents=True,exist_ok=True)
        if not backup.exists():shutil.copy2(dst,backup)
        shutil.copy2(src,dst)
        copies.append({'source':src.relative_to(pack).as_posix(),'destination':dst.relative_to(root).as_posix(),'sha256':hashlib.sha256(src.read_bytes()).hexdigest()})
    (target/'source.json').write_text(json.dumps({'source':'User-supplied Faithful 64x - Release 15','path':str(pack),'license':'See assets/licenses/Faithful-64x-Release-15.txt','files':[c for c in copies if c['destination'].startswith(folder)]},indent=2),encoding='utf-8')
(root/'assets/licenses').mkdir(exist_ok=True)
shutil.copy2(pack/'LICENSE.txt',root/'assets/licenses/Faithful-64x-Release-15.txt')
sounds=Path(r'F:\Users\alexispace\Downloads\mc_sounds\minecraft\sounds')
audio=[{'file':p.relative_to(sounds).as_posix(),'bytes':p.stat().st_size} for p in sorted(sounds.rglob('*')) if p.suffix.lower() in ('.ogg','.wav','.mp3')]
(out/'minecraft-audio-inventory.json').write_text(json.dumps({'source':str(sounds),'files':audio},indent=2),encoding='utf-8')
summary={'pack':str(pack),'textures':len(rows),'groups':dict(Counter(r['asset'].split('/')[0] for r in rows)),'sizes':dict(Counter(f"{r['width']}x{r['height']}" for r in rows)),'upgraded_textures':copies,'audio_source':str(sounds),'audio_count':len(audio)}
(out/'faithful64-release15-summary.json').write_text(json.dumps(summary,indent=2),encoding='utf-8')
print(json.dumps({k:v for k,v in summary.items() if k not in ('sizes','upgraded_textures')},indent=2));print('Updated textures:',len(copies))
