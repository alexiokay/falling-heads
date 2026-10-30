"""Reproducible Faithful R15 biome import. Never modifies the source pack."""
from pathlib import Path
from PIL import Image, ImageDraw
import json, hashlib, shutil

ROOT = Path(__file__).resolve().parents[1]
PACK = Path(r'C:\Users\alexispace\Downloads\Faithful 64x - Release 15')
SRC = PACK / 'assets/minecraft/textures'
OUT = ROOT / 'assets/textures/biome_content'
OUT.mkdir(parents=True, exist_ok=True)
records = []

def save(name, src, crop=None, overlay=None):
    path = SRC / (src + '.png')
    im = Image.open(path).convert('RGBA')
    source_size = im.size
    if crop:
        # Java model UV coordinates are expressed in the vanilla atlas units.
        box = tuple(v * 4 for v in crop)
        im = im.crop(box)
        if overlay:
            im.alpha_composite(Image.open(SRC / (overlay + '.png')).convert('RGBA').crop(box))
    im.save(OUT / (name + '.png'))
    records.append(dict(output=name+'.png', source=src+'.png', source_size=source_size,
                        crop_vanilla=crop, overlay=overlay,
                        sha256=hashlib.sha256(path.read_bytes()).hexdigest(), size=im.size))

blocks = '''crimson_nylium_side warped_nylium_side dirt wheat_stage7 farmland_moist hay_block_side hay_block_top moss_block rooted_dirt mossy_stone_bricks mossy_cobblestone short_grass fern sweet_berry_bush_stage3 flowering_azalea_leaves red_sand terracotta bone_block_side bone_block_top tuff calcite deepslate deepslate_top cracked_deepslate_bricks pointed_dripstone_up_tip pointed_dripstone_down_tip amethyst_block amethyst_cluster basalt_side basalt_top blackstone magma crimson_fungus warped_fungus crimson_nylium warped_nylium packed_ice blue_ice powder_snow end_stone_bricks purpur_pillar_side purpur_pillar_top'''.split()
for name in blocks:
    save(name, 'block/'+name)

faces = {
    'slime': ('slime/slime', (8,8,16,16), None),
    'creeper': ('creeper/creeper', (8,8,16,16), None),
    'bogged': ('skeleton/bogged', (8,8,16,16), 'skeleton/bogged_overlay'),
    'husk': ('zombie/husk', (8,8,16,16), None),
    'breeze': ('breeze/breeze', (8,8,16,16), 'breeze/breeze_eyes'),
    'silverfish': ('silverfish/silverfish', (2,2,5,4), None),
    'blaze': ('blaze/blaze', (8,8,16,16), None),
    'magma_cube': ('slime/magmacube', (8,24,16,32), None),
    'stray': ('skeleton/stray', (8,8,16,16), 'skeleton/stray_overlay'),
    'shulker': ('shulker/shulker', (6,58,12,64), None),
}
for name, (path, crop, overlay) in faces.items():
    save(name, 'entity/'+path, crop, 'entity/'+overlay if overlay else None)
# Slime eyes and mouth are separate model boxes in the source atlas.
slime=Image.open(SRC/'entity/slime/slime.png').convert('RGBA')
face=Image.open(OUT/'slime.png').convert('RGBA')
for box, xy in [((34,2,36,4),(4,8)),((34,2,36,4),(20,8)),((33,5,34,6),(16,20))]:
    part=slime.crop(tuple(v*4 for v in box));face.alpha_composite(part,xy)
face.save(OUT/'slime.png')
for name in ['amethyst_shard', 'wind_charge', 'fire_charge', 'sweet_berries', 'blaze_rod', 'slime_ball', 'shulker_shell', 'tipped_arrow_base', 'tipped_arrow_head']:
    save(name, 'item/'+name)
save('shulker_spark', 'entity/shulker/spark')
# Keep the shaft neutral: only the potion-coated arrowhead receives substance color.
for name,color in [('poison_arrow',(100,190,36)),('frost_arrow',(100,200,255))]:
    base=Image.open(OUT/'tipped_arrow_base.png').convert('RGBA')
    tip=Image.open(OUT/'tipped_arrow_head.png').convert('RGBA')
    colored=Image.new('RGBA',tip.size,color+(255,));colored.putalpha(tip.getchannel('A'))
    from PIL import ImageChops
    tint=ImageChops.multiply(tip,colored);base.alpha_composite(tint);base.save(OUT/(name+'.png'))

# Magma uses explicit frame files, so animation does not stretch the strip.
magma = Image.open(OUT / 'magma.png')
for i in range(magma.height//magma.width):
    magma.crop((0,i*64,64,(i+1)*64)).save(OUT / f'magma_{i}.png')
# Directional block atlases: top | bottom | side | front, same contract as the game.
for name in blocks + ['magma_0','magma_1','magma_2']:
    source=Image.open(OUT/(name+'.png')).convert('RGBA').crop((0,0,64,64))
    top_name={'hay_block_side':'hay_block_top','bone_block_side':'bone_block_top','basalt_side':'basalt_top','deepslate':'deepslate_top','purpur_pillar_side':'purpur_pillar_top'}.get(name,name)
    top=Image.open(OUT/(top_name+'.png')).convert('RGBA').crop((0,0,64,64))
    if name in ('crimson_nylium','warped_nylium'): source=Image.open(OUT/(name+'_side.png')).convert('RGBA')
    if name=='farmland_moist': source=Image.open(OUT/'dirt.png').convert('RGBA')
    atlas=Image.new('RGBA',(256,64));
    for i,face in enumerate([top,top,source,source]): atlas.paste(face,(i*64,0))
    atlas.save(OUT/(name+'_atlas.png'))
# Sound originals use the game's shared effects bus; no synthesized substitutes.
sounds=Path(r'F:/Users/alexispace/Downloads/mc_sounds/minecraft/sounds')
audio=ROOT/'assets/audio/biome_content';audio.mkdir(parents=True,exist_ok=True)
audio_files={'blaze_shoot':'fire/ignite.ogg','blaze_hurt':'mob/blaze/hit1.ogg','blaze_death':'mob/blaze/death.ogg','shulker_shoot':'entity/shulker/shoot1.ogg','creeper_fuse':'random/fuse.ogg','slime_jump':'mob/slime/big1.ogg','magma_jump':'mob/magmacube/jump1.ogg'}
audio_records=[]
for name,path in audio_files.items():
    original=sounds/path
    if original.exists():
        shutil.copy2(original,audio/(name+'.ogg'))
        audio_records.append({'file':name+'.ogg','source':str(original),'sha256':hashlib.sha256(original.read_bytes()).hexdigest()})
(audio/'source.json').write_text(json.dumps(audio_records,indent=2),encoding='utf-8')
(OUT/'source.json').write_text(json.dumps(dict(pack=str(PACK), license='assets/licenses/Faithful-64x-Release-15.txt', files=records), indent=2), encoding='utf-8')
canvas=Image.new('RGB',(800,((len(records)+9)//10)*96),(38,42,47))
draw=ImageDraw.Draw(canvas)
for i,r in enumerate(records):
    im=Image.open(OUT/r['output']).convert('RGBA');im=im.crop((0,0,im.width,min(im.height,im.width)))
    im.thumbnail((64,64),Image.Resampling.NEAREST)
    x=(i%10)*80;y=(i//10)*96;canvas.paste(im,(x,y),im)
    draw.text((x,y+65),r['output'][:12],fill='white')
gallery=ROOT/'design/biome-content-assets.png';gallery.parent.mkdir(exist_ok=True)
canvas.save(gallery)
print(f'Imported {len(records)} assets; gallery: {gallery}')

