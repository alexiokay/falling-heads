"""Local, read-only world/asset workbench. Uses the game's actual headless exporter."""
from http.server import ThreadingHTTPServer, SimpleHTTPRequestHandler
from pathlib import Path
from urllib.parse import urlparse, parse_qs, unquote
import json, subprocess, tempfile, threading, re, time, base64

ROOT = Path(__file__).resolve().parents[2]
PACK = Path(r'C:\Users\alexispace\Downloads\Faithful 64x - Release 15\assets\minecraft\textures')
LOCK = threading.Lock()
CACHE = {}
BIOMES = ['Plains','Forest','Desert','Mountains','Underground','Nether','End','Iceland']
def manifest():
    source=(ROOT/'src/plugins/asset_loader_plugin.rs').read_text(encoding='utf-8')
    handles=dict(re.findall(r'(\w+): asset_server.load\("([^"\n]+\.png)"\)',source))
    names={'Dirt':'dirt','Stone':'stone','Coal':'coal_ore','Iron':'iron_ore','Gold':'gold_ore','Diamond':'diamond_ore','Emerald':'emerald_ore','Redstone':'redstone_ore','Wood':'oak_log','Leaves':'oak_leaves','Sand':'sand','Sandstone':'sandstone','Netherrack':'netherrack','Obsidian':'obsidian','Glowstone':'glowstone','AncientDebris':'ancient_debris_side','EndStone':'end_stone','Snow':'snow','Ice':'ice','PackedIce':'packed_ice','SoulSand':'soul_sand','Pumpkin':'pumpkin_front','Melon':'melon_side','Cactus':'cactus_side','Cobweb':'cobweb','NetherQuartz':'nether_quartz_ore'}
    mapping={k:'/assets/'+handles[v] for k,v in names.items() if v in handles}
    # Direct material bindings override friendly names (e.g. Gold uses a block,
    # not ore). Reading them avoids a second set of drifting texture choices.
    material_source=(ROOT/'src/main.rs').read_text(encoding='utf-8')
    for kind,body in re.findall(r'BlockType::(\w+)\s*=>((?:(?!BlockType::).)*)',material_source,re.S):
        match=re.search(r'base_color_texture:\s*Some\(block_textures\.(\w+)\.clone\(\)\)',body)
        if match and match[1] in handles:mapping[kind]='/assets/'+handles[match[1]]
    mapping.update({'Grass':'/assets/textures/blocks/forest/species/grass_block_side.png','Mushroom':'/assets/textures/blocks/general/red_mushroom.png','Purpur':'/assets/textures/blocks/end/purpur_block.png'})
    finishes=['textures/structures/oak_planks.png','textures/structures/dark_oak_planks.png','textures/structures/stone_bricks.png','textures/structures/nether_bricks.png','textures/blocks/desert/sandstone/sandstone.png','textures/structures/cut_sandstone.png','textures/structures/chiseled_sandstone.png']
    mapping.update({f'Finish_{i}':'/assets/'+p for i,p in enumerate(finishes)})
    mapping['Netherite']='/assets/textures/blocks/nether/netherite_block.png'
    for species in ['Oak','Birch','Spruce']:
        for part in ['log','leaves']:
            mapping[species+'_'+part]='/assets/textures/blocks/forest/species/'+species.lower()+'_'+part+'.png'
    for path in (ROOT/'assets/textures/biome_content').glob('*.png'):
        if not path.stem.endswith('_atlas'): mapping['Biome_'+path.stem]='/assets/textures/biome_content/'+path.name
    return {'textures':mapping,'biomes':BIOMES,'pack':str(PACK),'assets':[{'name':str(p.relative_to(PACK)).replace('\\','/'),'url':'/pack/'+str(p.relative_to(PACK)).replace('\\','/')} for p in sorted(PACK.rglob('*.png'))] if PACK.exists() else []}
class Handler(SimpleHTTPRequestHandler):
    def log_message(self,*args): pass
    def send_json(self,data,status=200):
        raw=json.dumps(data).encode();self.send_response(status);self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(raw)));self.end_headers();self.wfile.write(raw)
    def do_POST(self):
        if self.path!='/api/save-preview':return self.send_error(404)
        try:
            if self.headers.get('Origin','http://127.0.0.1:48119')!='http://127.0.0.1:48119':raise ValueError('Use the local workbench to save previews')
            size=int(self.headers.get('Content-Length','0'))
            if not 0<size<12000000:raise ValueError('Invalid preview size')
            data=json.loads(self.rfile.read(size));prefix='data:image/png;base64,'
            if not data['image'].startswith(prefix):raise ValueError('Expected PNG')
            raw=base64.b64decode(data['image'][len(prefix):],validate=True)
            if not raw.startswith(b'\x89PNG\r\n\x1a\n'):raise ValueError('Invalid PNG')
            name='preview-'+re.sub('[^a-zA-Z0-9_-]','',data.get('name','world'))[:80]+'.png'
            folder=ROOT/'design/world-previews';folder.mkdir(exist_ok=True,parents=True)
            (folder/name).write_bytes(raw)
            return self.send_json({'path':str(folder/name),'url':'/exports/'+name})
        except Exception as e:return self.send_json({'error':str(e)},400)
    def do_GET(self):
        url=urlparse(self.path)
        if url.path=='/api/manifest': return self.send_json(manifest())
        if url.path=='/api/generate':
            try:
                q=parse_qs(url.query); get=lambda k,d:q.get(k,[d])[0]
                biome=get('biome','Plains')
                if biome not in BIOMES: raise ValueError('Unknown biome')
                args=['--export-world-preview','--biome',biome]
                params={'seed':max(0,min(4294967295,int(get('seed','921')))),'x':max(-1000000,min(1000000,int(get('x','96')))),'width':max(32,min(384,int(get('width','160')))),'height':max(32,min(256,int(get('height','112'))))}
                if get('y','').strip(): params['y']=max(-1000000,min(1000000,int(get('y','0'))))
                for k,v in params.items(): args += ['--'+k,str(v)]
                exe=ROOT/'target/debug/falling-heads.exe'
                key=(exe.stat().st_mtime_ns,*args)
                with LOCK:
                    if key not in CACHE:
                        start=time.perf_counter()
                        with tempfile.TemporaryDirectory(prefix='mineheads-preview-') as d:
                            out=Path(d)/'world.json'
                            result=subprocess.run([str(exe),*args,'--output',str(out)],cwd=ROOT,capture_output=True,timeout=90,creationflags=subprocess.CREATE_NO_WINDOW)
                            if result.returncode: raise RuntimeError(result.stderr.decode(errors='replace')[-2000:])
                            data=json.loads(out.read_text());data['total_ms']=(time.perf_counter()-start)*1000
                        if len(CACHE)>64:CACHE.clear()
                        CACHE[key]=data
                    return self.send_json(CACHE[key])
            except Exception as e:return self.send_json({'error':str(e)},500)
        roots={'/assets/':ROOT/'assets','/pack/':PACK,'/exports/':ROOT/'design/world-previews'}
        for prefix,root in roots.items():
            if url.path.startswith(prefix):
                file=(root/unquote(url.path[len(prefix):])).resolve()
                if not file.is_relative_to(root.resolve()) or not file.is_file():return self.send_error(404)
                return self.send_file(file)
        if url.path in ['/','/index.html']:return self.send_file(Path(__file__).with_name('index.html'))
        return self.send_error(404)
    def send_file(self,file):
        raw=file.read_bytes();self.send_response(200);self.send_header('Content-Type',self.guess_type(str(file)));self.send_header('Content-Length',str(len(raw)));self.end_headers();self.wfile.write(raw)
if __name__=='__main__':
    print('MineHeads world workbench: http://127.0.0.1:48119',flush=True)
    ThreadingHTTPServer(('127.0.0.1',48119),Handler).serve_forever()
