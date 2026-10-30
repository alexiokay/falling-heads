"""Render the real headless world exporter with original textures, without launching a window."""
from pathlib import Path
from PIL import Image, ImageDraw
import subprocess,json,collections

ROOT=Path(__file__).resolve().parents[1]
PACK=Path(r'C:\Users\alexispace\Downloads\Faithful 64x - Release 15\assets\minecraft\textures\block')
OUT=ROOT/'design/biome-expansion';OUT.mkdir(parents=True,exist_ok=True)
BIOMES=['Plains','Forest','Desert','Mountains','Underground','Nether','End','Iceland']
NAMES=dict(Stone='stone',Dirt='dirt',Grass='grass_block_side',Sand='sand',Sandstone='sandstone',Snow='snow',Ice='ice',Wood='oak_log',Leaves='oak_leaves',Coal='coal_ore',Iron='iron_ore',Gold='gold_ore',Diamond='diamond_ore',Emerald='emerald_ore',Redstone='redstone_ore',Obsidian='obsidian',Netherrack='netherrack',Netherite='netherite_block',AncientDebris='ancient_debris_side',SoulSand='soul_sand',Glowstone='glowstone',EndStone='end_stone',ChorusFruit='chorus_plant',Purpur='purpur_block',DeadBush='dead_bush',Mushroom='red_mushroom',Vines='vine',Cobweb='cobweb',Pumpkin='pumpkin_side',Melon='melon_side',Cactus='cactus_side')
FINISH=['oak_planks','dark_oak_planks','stone_bricks','nether_bricks','sandstone','cut_sandstone','chiseled_sandstone']
cache={}
def texture(kind,grass,override):
    name=NAMES.get(kind,'stone');path=PACK/(name+'.png')
    if grass:path=PACK/'grass_block_side.png'
    if override:
        if override.startswith('Biome_'):path=ROOT/'assets/textures/biome_content'/(override[6:]+'.png')
        elif override.startswith('Finish_'):path=PACK/(FINISH[int(override[7:])]+'.png')
        elif '_' in override:path=ROOT/'assets/textures/blocks/forest/species'/(override.lower()+'.png')
    key=str(path)
    if key not in cache:
        if not path.exists():raise FileNotFoundError(path)
        im=Image.open(path).convert('RGBA');im=im.crop((0,0,im.width,min(im.width,im.height)))
        if name in ['grass_block_side','oak_leaves','vine'] or (override and override in ['Biome_short_grass','Biome_fern']):
            # The pack supplies biome-tintable grayscale foliage.
            from PIL import ImageChops
            im=ImageChops.multiply(im,Image.new('RGBA',im.size,(130,185,80,255)))
        cache[key]=im.resize((12,12),Image.Resampling.NEAREST)
    return cache[key]
stats={};thumbs=[]
for biome in BIOMES:
    path=OUT/(biome.lower()+'.json')
    subprocess.run([str(ROOT/'target/debug/falling-heads.exe'),'--export-world-preview','--biome',biome,'--seed','921','--width','192','--height','112','--output',str(path)],cwd=ROOT,check=True,creationflags=subprocess.CREATE_NO_WINDOW)
    data=json.loads(path.read_text());w,h=data['width'],data['height'];mx,my=data['min']
    im=Image.new('RGB',(w*12,h*12+44),(22,26,30));draw=ImageDraw.Draw(im)
    draw.text((12,10),f'{biome} | actual generator, seed 921 | 2D texture preview; lighting/actors/fluids not simulated',fill='white')
    for x,y,kind,grass,override in data['cells']:
        tex=texture(kind,grass,override);im.paste(tex,((x-mx)*12,(h-1-y+my)*12+44),tex)
    im.save(OUT/(biome.lower()+'.png'))
    stats[biome]=dict(generation_ms=data['generation_ms'],tiles=len(data['cells']),new_materials=dict(collections.Counter(c[4] for c in data['cells'] if c[4] and c[4].startswith('Biome_'))))
    im.thumbnail((768,464),Image.Resampling.LANCZOS);thumbs.append(im)
contact=Image.new('RGB',(1536,464*4),(22,26,30))
for i,im in enumerate(thumbs):contact.paste(im,((i%2)*768,(i//2)*464))
contact.save(OUT/'all-biomes.png');(OUT/'generation-stats.json').write_text(json.dumps(stats,indent=2))
print(json.dumps(stats,indent=2))
