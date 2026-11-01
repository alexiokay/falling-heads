# Draws original 16x16 pixel-art faces for every head in the free pack and
# scales them (nearest neighbour) to the size and format the game loads.
# Each face is a grid: '.' is the face's mottled background colours, any other
# character comes from its palette. Bee and iron golem are skins: the face is
# painted into the rectangle their face mesh reads (see extra_powers.rs).
# Needs ffmpeg/ffprobe on PATH. Fixed random seed: reruns give the same art.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Split-Path -Parent $PSScriptRoot
$tex = Join-Path $root 'assets/packs/free/textures'
$tmp = Join-Path ([IO.Path]::GetTempPath()) 'falling-heads-faces'
New-Item -ItemType Directory -Force -Path $tmp | Out-Null

function C([int]$rgb, [int]$a = 255) { [System.Drawing.Color]::FromArgb($a, ($rgb -shr 16) -band 255, ($rgb -shr 8) -band 255, $rgb -band 255) }
# Case-sensitive palette (PowerShell hash literals treat 'S' and 's' as one key).
function Pal {
    $d = [System.Collections.Generic.Dictionary[string, System.Drawing.Color]]::new([StringComparer]::Ordinal)
    for ($i = 0; $i -lt $args.Count; $i += 2) { $d[[string]$args[$i]] = $args[$i + 1] }
    ,$d
}

# ---------------------------------------------------------------- shared grids
$humanRows = @(
    'HHHHHHHHHHHHHHHH',
    'HHHHHHHHHHHHHHHH',
    'HHHHHHDDHHHHHHHH',
    'HHSSSSSSSSSSSSHH',
    'HSSSSSSSSSSSSSSH',
    'SSSSSSSSSSSSSSSS',
    'SSDDDSSSSSSDDDSS',
    'SSSSSSSSSSSSSSSS',
    'SSWWIISSSSIIWWSS',
    'SSWWIISSSSIIWWSS',
    'SSSSSSSnnSSSSSSS',
    'SSSSSSSnnSSSSSSS',
    'SSSSSMMMMMMSSSSS',
    'SSSSSSMMMMSSSSSS',
    'sSSSSSSSSSSSSSSs',
    'ssSSSSSSSSSSSSss'
)
$villagerRows = @(
    'HHHHHHHHHHHHHHHH',
    'HSSSSSSSSSSSSSSH',
    'SSSSSSSSSSSSSSSS',
    'SSSSSSSSSSSSSSSS',
    'SSSSSSSSSSSSSSSS',
    'SDDDDDDDDDDDDDDS',
    'SSSSSSSSSSSSSSSS',
    'SSWIISSNNSSIIWSS',
    'SSWIISSNNSSIIWSS',
    'SSSSSSNNNNSSSSSS',
    'SSSSSSNNNNSSSSSS',
    'SSSSSSNNNNSSSSSS',
    'SSSSSSnNNnSSSSSS',
    'SSSSSMMnnMMSSSSS',
    'sSSSSSSSSSSSSSSs',
    'ssSSSSSSSSSSSSss'
)
$zombieRows = @(
    'zzZZZZZzZZZZZZzz',
    'zZZZZZZZZZZzZZZz',
    'ZZZZzZZZZZZZZZZZ',
    'ZZZZZZZZZZZZZzZZ',
    'ZZZZZZZZZZZZZZZZ',
    'ZZeeeZZZZZZeeeZZ',
    'ZZeeeZZZZZZeeeZZ',
    'ZZZZZZZZZZZZZZZZ',
    'ZZZZZZzzzZZZZZZZ',
    'ZZZZZZzzzZZZZZZZ',
    'ZZZZZZZZZZZZZZZZ',
    'ZZZkkkkkkkkkZZZZ',
    'ZZZkzkzkzkzkZZZZ',
    'ZZZZZZZZZZZZZZZZ',
    'zZZZZZZZZZZZZZZz',
    'zzZZZZZZZZZZZZzz'
)
$skullRows = @(
    'bBBBBBBBBBBBBBBb',
    'BBBBBBBBBBBBBBBB',
    'BBBBBBBBBBBBBBBB',
    'BBBBBBBBBBBBBBBB',
    'BBBBBBBBBBBBBBBB',
    'BBkkkBBBBBBkkkBB',
    'BBkkkBBBBBBkkkBB',
    'BBkkkBBBBBBkkkBB',
    'BBBBBBBBBBBBBBBB',
    'BBBBBBBkkBBBBBBB',
    'BBBBBBBBBBBBBBBB',
    'BBkkkkkkkkkkkkBB',
    'BBkBkBkBkBkBkkBB',
    'BBkkkkkkkkkkkkBB',
    'bBBBBBBBBBBBBBBb',
    'bbBBBBBBBBBBBBbb'
)
function Human($hair, $dark, $skin, $shade, $iris) {
    @{ rows = $humanRows; pal = (Pal H (C $hair) D (C $dark) S (C $skin) s (C $shade) n (C $shade) W (C 0xf4f4f4) I (C $iris) M (C 0x7a3e33)) }
}
function Zombie($skin, $shade, $dark, $eyes) {
    @{ rows = $zombieRows; pal = (Pal Z (C $skin) z (C $shade) k (C $dark) e (C $eyes)) }
}
function Skull($bone, $shade) {
    @{ rows = $skullRows; pal = (Pal B (C $bone) b (C $shade) k (C 0x262626)) }
}
function Over([string[]]$rows, [hashtable]$marks) {
    # Start from a blank '.' grid and stamp feature rows by index.
    $grid = @('................') * 16
    foreach ($k in $marks.Keys) { $grid[$k] = $marks[$k] }
    $grid
}

# ---------------------------------------------------------------- the faces
$faces = @{
    hero  = Human 0x5a3a22 0x3e2716 0xe0a882 0xc98c66 0x3b5bd6
    hero2 = Human 0x24201e 0x141110 0xb98563 0x9c6c4c 0x5a3d2a
    hero3 = Human 0xb5462a 0x8a3018 0xf0c0a0 0xd8a080 0x3a9a4a
    hero4 = Human 0xe8c860 0xc8a440 0xe8b896 0xcc9a78 0x8a4ad0
    villager = @{ rows = $villagerRows; pal = (Pal H (C 0x5a3d2b) S (C 0xbd8b72) s (C 0xa0705a) D (C 0x3d2a1e) W (C 0xf4f4f4) I (C 0x2f8f3a) N (C 0xa8775f) n (C 0x8c5f4a) M (C 0x6b3f30)) }
    pillager = @{ rows = $villagerRows; pal = (Pal H (C 0x4a4d4e) S (C 0x8f9293) s (C 0x7a7d7e) D (C 0x1e1f20) W (C 0xd8d8d8) I (C 0x2a2a2a) N (C 0x7f8283) n (C 0x666869) M (C 0x3a3b3c)) }
    witch = @{
        rows = @(
            'PPPPPPPPPPPPPPPP',
            'PPPPPPPPPPPPPPPP',
            'PPPPPPYYYPPPPPPP',
            'bbbbbbYkYbbbbbbb',
            'SSSSSSSSSSSSSSSS',
            'SDDDDDDSSDDDDDDS',
            'SSSSSSSSSSSSSSSS',
            'SSWLLSSNNSSLLWSS',
            'SSWLLSSNNSSLLWSS',
            'SSSSSSNNNNSSSSSS',
            'SSSSSSNNgNSSSSSS',
            'SSSSSSNNNNSSSSSS',
            'SSSSSSnNNnSSSSSS',
            'SSSSSMMnnMMSSSSS',
            'sSSSSSSSSSSSSSSs',
            'ssSSSSSSSSSSSSss'
        )
        pal = (Pal P (C 0x3b1f4f) b (C 0x3f7a2a) Y (C 0xd8b23a) k (C 0x2a2010) S (C 0x9fae7c) s (C 0x86945f) D (C 0x2e3320) W (C 0xf0f0e0) L (C 0x8a3ab0) N (C 0x8f9e6c) n (C 0x77854f) g (C 0x3f6a2a) M (C 0x4a3a2a))
    }
    zombie  = Zombie 0x5f9e4a 0x4a7d39 0x1e2a18 0x1a2214
    drowned = Zombie 0x4f9a96 0x3c7a77 0x0f2e30 0x7ff6ff
    husk    = Zombie 0xc2a36b 0xa3874f 0x3d2e17 0x2a1f10
    skeleton = Skull 0xd8d8d0 0xb5b5ab
    stray    = Skull 0xb8c8cc 0x93a6ab
    bogged   = Skull 0xb7c29a 0x8e9b6e
    creeper = @{
        bg = @(0x5dbb3f, 0x4a9a30, 0x7fd35e, 0x53a838); pal = (Pal k (C 0x16210f))
        rows = Over $null @{ 4 = '...kkk....kkk...'; 5 = '...kkk....kkk...'; 6 = '....kk....kk....'; 9 = '......kkkk......'; 10 = '.....kk..kk.....'; 11 = '....kk....kk....'; 12 = '....k......k....' }
    }
    slime = @{
        bg = @(0x7ecb5e, 0x6cb84d, 0x8fd66f); pal = (Pal k (C 0x1f3d18) w (C 0xd8ffd0) g (C 0x4f9a3a))
        rows = Over $null @{ 1 = '.ww.............'; 2 = '.w..............'; 3 = '..gggggggggggg..'; 4 = '..g..........g..'; 5 = '..g.kk....kk.g..'; 6 = '..g.kk....kk.g..'; 7 = '..g..........g..'; 8 = '..g..........g..'; 9 = '..g....kk....g..'; 10 = '..g..........g..'; 11 = '..g..........g..'; 12 = '..gggggggggggg..' }
    }
    magma_cube = @{
        bg = @(0x3a1010, 0x521a12, 0x2a0c0c); pal = (Pal o (C 0xff8a1e) y (C 0xffe066) r (C 0xd8261e))
        rows = Over $null @{ 2 = 'oo.....ooo....oo'; 3 = '..oo.....oo.....'; 5 = '...yy......yy...'; 6 = '...rr......rr...'; 9 = '.ooo..oo...ooo..'; 11 = '.....oooooo.....'; 13 = 'oo..ooo.....oo.o' }
    }
    blaze = @{
        bg = @(0xf2c13a, 0xe8a82a, 0xffd65a); pal = (Pal k (C 0x3a2408) o (C 0x8a4a10))
        rows = Over $null @{ 4 = '..kkkk....kkkk..'; 6 = '...kk......kk...'; 7 = '...kk......kk...'; 11 = '.....oooooo.....'; 12 = '......oooo......' }
    }
    breeze = @{
        bg = @(0xbfe3ff, 0xa8d4f5, 0xd6efff); pal = (Pal k (C 0x1c2c44) c (C 0xffffff) w (C 0xf4fbff))
        rows = Over $null @{ 2 = '..wwwww.........'; 3 = '.w.....ww.......'; 6 = '...kk......kk...'; 7 = '...kc......kc...'; 10 = '.....wwwwww.....'; 11 = '....w......w....'; 13 = '........wwwww...' }
    }
    enderman = @{
        bg = @(0x161616, 0x1d1d1d, 0x111111); pal = (Pal p (C 0xb14fe6) P (C 0xf2b8ff))
        rows = Over $null @{ 6 = '..ppp......ppp..'; 7 = '..pPp......pPp..'; 8 = '..ppp......ppp..' }
    }
    cave_spider = @{
        bg = @(0x1f3b4a, 0x26485a, 0x183140); pal = (Pal r (C 0xb81e18) R (C 0xff5a4a) f (C 0x9a9a9a))
        rows = Over $null @{ 4 = '..r..r....r..r..'; 6 = '...rR......Rr...'; 7 = '...rr......rr...'; 9 = '.....r....r.....'; 12 = '......f..f......'; 13 = '.......ff.......' }
    }
    penguin = @{
        rows = @(
            'kkkkkkkkkkkkkkkk',
            'kkkkkkkkkkkkkkkk',
            'kkkkkkkkkkkkkkkk',
            'kkkkkwwwwwwkkkkk',
            'kkkwwwwwwwwwwkkk',
            'kkwwwwwwwwwwwwkk',
            'kkwwwwwwwwwwwwkk',
            'kkwwkkwwwwkkwwkk',
            'kkwwkkwwwwkkwwkk',
            'kkwwwwwoowwwwwkk',
            'kkwwwwoooowwwwkk',
            'kkwwwwwoowwwwwkk',
            'kkwwwwwwwwwwwwkk',
            'kkkwwwwwwwwwwkkk',
            'kkkkwwwwwwwwkkkk',
            'kkkkkwwwwwwkkkkk'
        )
        pal = (Pal k (C 0x1a1a22) w (C 0xf2f2f2) o (C 0xf29a1e))
    }
    chicken = @{
        bg = @(0xf4f4f0, 0xdcdcd4, 0xffffff); pal = (Pal k (C 0x1a1a1a) y (C 0xf0b62a) r (C 0xd8261e))
        rows = Over $null @{ 6 = '...kk......kk...'; 7 = '...kk......kk...'; 9 = '......yyyy......'; 10 = '......yyyy......'; 11 = '.......rr.......'; 12 = '.......rr.......' }
    }
    cow = @{
        bg = @(0x5a3a26, 0x4d311f, 0x634330); pal = (Pal h (C 0xd8d0c0) w (C 0xf0ece4) k (C 0x141010) e (C 0xffffff) p (C 0xe6a0a0) n (C 0x9a4a50))
        rows = Over $null @{ 0 = 'h..............h'; 1 = 'hh............hh'; 2 = '....wwww........'; 3 = '...wwwwww.......'; 4 = '....wwww........'; 6 = '..kk......kk....'; 7 = '..ke......ke....'; 10 = '...pppppppppp...'; 11 = '...pppppppppp...'; 12 = '...ppnppppnpp...'; 13 = '...pppppppppp...'; 14 = '....pppppppp....' }
    }
    mooshroom = @{
        bg = @(0x9c2a22, 0x8a231c, 0xa8342a); pal = (Pal h (C 0xd8d0c0) w (C 0xf0ece4) k (C 0x141010) e (C 0xffffff) p (C 0xe6a0a0) n (C 0x9a4a50))
        rows = Over $null @{ 0 = 'h..............h'; 1 = 'hh............hh'; 2 = '........wwww....'; 3 = '.......wwwwww...'; 4 = '........wwww....'; 6 = '..kk......kk....'; 7 = '..ke......ke....'; 10 = '...pppppppppp...'; 11 = '...pppppppppp...'; 12 = '...ppnppppnpp...'; 13 = '...pppppppppp...'; 14 = '....pppppppp....' }
    }
    yeti = @{
        bg = @(0xeef2f6, 0xdfe6ec, 0xf8fbff); pal = (Pal b (C 0x6f8fb3) k (C 0x1a2230) r (C 0xd8261e) w (C 0xffffff))
        rows = Over $null @{ 5 = '...bbbbbbbbbb...'; 6 = '...bkkbbbbkkb...'; 7 = '...bbrkbbkrbb...'; 8 = '...bbbbbbbbbb...'; 9 = '...bbbbkkbbbb...'; 10 = '...bbbbbbbbbb...'; 11 = '...bkkkkkkkkb...'; 12 = '...bkwkkkkwkb...'; 13 = '...bbbbbbbbbb...'; 14 = '....bbbbbbbb....' }
    }
    ghast = @{
        bg = @(0xf0f0f0, 0xe2e2e2, 0xfafafa); pal = (Pal k (C 0x2a2a2a) g (C 0xb8b8b8))
        rows = Over $null @{ 6 = '..kkk......kkk..'; 8 = '...g........g...'; 9 = '...g........g...'; 10 = '...g........g...'; 11 = '......kkkk......'; 12 = '......kkkk......' }
    }
    ghast_shooting = @{
        bg = @(0xf0f0f0, 0xe2e2e2, 0xfafafa); pal = (Pal k (C 0x2a2a2a) r (C 0xd8261e))
        rows = Over $null @{ 5 = '..kkk......kkk..'; 6 = '..krk......krk..'; 7 = '..kkk......kkk..'; 10 = '.....kkkkkk.....'; 11 = '.....krrrrk.....'; 12 = '.....kkkkkk.....' }
    }
    happy_ghast = @{
        bg = @(0xf0f0f0, 0xe2e2e2, 0xfafafa); pal = (Pal k (C 0x2a2a2a) p (C 0xf4a0b4))
        rows = Over $null @{ 5 = '....k......k....'; 6 = '...k.k....k.k...'; 8 = '..pp........pp..'; 10 = '....k......k....'; 11 = '.....kkkkkk.....' }
    }
    sentinel = @{
        bg = @(0x3f8f86, 0x347a72, 0x4a9a90); pal = (Pal y (C 0xe8c040) w (C 0xf4f4e8) o (C 0xe0602a) k (C 0x1a1a1a))
        rows = Over $null @{ 0 = 'y......yy......y'; 4 = '.....wwwwww.....'; 5 = '....wwwwwwww....'; 6 = '....wwwookww....'; 7 = '....wwwookww....'; 8 = '....wwwwwwww....'; 9 = '.....wwwwww.....'; 15 = 'y......yy......y' }
    }
    shulker = @{
        bg = @(0x8a5ca8, 0x6e4589, 0x9a6cb8); pal = (Pal Y (C 0xe8d36a) k (C 0x2a2010))
        rows = Over $null @{ 4 = '..YYYYYYYYYYYY..'; 5 = '..YYYYYYYYYYYY..'; 6 = '..YYYYYYYYYYYY..'; 7 = '..YYkkYYYYkkYY..'; 8 = '..YYkkYYYYkkYY..'; 9 = '..YYYYYYYYYYYY..'; 10 = '..YYYYkkkkYYYY..'; 11 = '..YYYYYYYYYYYY..' }
    }
    silverfish = @{
        bg = @(0x8a8f94, 0x777c80, 0x9aa0a5); pal = (Pal d (C 0x5c6064) k (C 0x1a1a1a))
        rows = Over $null @{ 0 = '.....d....d.....'; 2 = '.....d....d.....'; 4 = '.....d....d.....'; 6 = '.k...d....d.....'; 7 = '.k...d....d.....'; 9 = '.....d....d.....'; 11 = '.....d....d.....'; 13 = '.....d....d.....'; 15 = '.....d....d.....' }
    }
    bat_face = @{
        bg = @(0x4a3626, 0x3c2b1e, 0x553f2c); pal = (Pal e (C 0x2a1c12) k (C 0x111111) w (C 0xf0f0f0))
        rows = Over $null @{ 0 = 'ee............ee'; 1 = 'eee..........eee'; 2 = 'eeee........eeee'; 6 = '...kw......wk...'; 7 = '...kk......kk...'; 11 = '......w..w......'; 12 = '......w..w......' }
    }
    bat_wing = @{
        bg = @(0x2c2120, 0x3a2c28, 0x33262a); pal = (Pal b (C 0x5a463c))
        rows = Over $null @{ 1 = 'b...............'; 3 = '.bb.............'; 5 = '...bbb..........'; 7 = '......bbb.......'; 9 = '.........bbb....'; 11 = '............bbb.'; 13 = '..............bb' }
    }
    golem_face = @{
        bg = @(0xc9c4bc, 0xb8b2a8, 0xd6d1ca); pal = (Pal k (C 0x5a544c) r (C 0xc8281e) n (C 0xa59f95) v (C 0x4a8a2a))
        rows = Over $null @{ 1 = 'v.........v.....'; 3 = '..kkkkkkkkkkkk..'; 5 = '...kr......rk...'; 6 = '...kk..nn..kk...'; 7 = '.......nn.......'; 8 = '.......nn.......'; 9 = '.......nn....v..'; 10 = '.......nn.......'; 11 = '......nnnn......'; 13 = '....kkkkkkkk....' }
    }
    bee_face = @{
        bg = @(0xf2c43a, 0xe6b42a, 0xffd04a); pal = (Pal k (C 0x1a1410) w (C 0xffffff))
        rows = Over $null @{ 4 = '..kkkk....kkkk..'; 5 = '..kkwk....kwkk..'; 6 = '..kkkk....kkkk..'; 7 = '..kkkk....kkkk..'; 11 = '......kkkk......' }
    }
    bee_face_angry = @{
        bg = @(0xf2c43a, 0xe6b42a, 0xffd04a); pal = (Pal k (C 0x1a1410) r (C 0xd8261e))
        rows = Over $null @{ 3 = '..k..........k..'; 4 = '..kkkk....kkkk..'; 5 = '..kkrk....krkk..'; 6 = '..kkkk....kkkk..'; 7 = '..kkkk....kkkk..'; 11 = '.....kkkkkk.....' }
    }
}

# Which file shows which face.
$targets = [ordered]@{
    'entities/hero.jpg' = 'hero'; 'entities/player/steve.jpg' = 'hero2'
    'entities/player/skinmc-avatar (3).png' = 'hero3'; 'entities/player/skinmc-avatar (4).png' = 'hero4'
    'entities/villager.png' = 'villager'; 'entities/VillagerFace.webp' = 'villager'
    'entities/CowFace.webp' = 'cow'; 'entities/MooshroomFace.png' = 'mooshroom'
    'entities/enemy/CaveSpiderFace.png' = 'cave_spider'
    'entities/enemy/EndermanFace.png' = 'enderman'; 'entities/enemy/EndermanFace.webp' = 'enderman'
    'entities/enemy/bat_face.png' = 'bat_face'; 'entities/enemy/bat_wing.png' = 'bat_wing'
    'entities/enemy/blaze.webp' = 'blaze'; 'biome_content/blaze.png' = 'blaze'
    'entities/enemy/chicken.webp' = 'chicken'
    'entities/enemy/creeper.webp' = 'creeper'; 'biome_content/creeper.png' = 'creeper'
    'entities/enemy/drowned.jpg' = 'drowned'
    'entities/enemy/ghast/ghast.png' = 'ghast'; 'entities/enemy/ghast/ghast_shooting.png' = 'ghast_shooting'
    'entities/enemy/ghast/happy_ghast.png' = 'happy_ghast'
    'entities/enemy/penguin.png' = 'penguin'; 'entities/enemy/pillager.png' = 'pillager'
    'entities/enemy/sentinel_face.png' = 'sentinel'
    'entities/enemy/skeleton.png' = 'skeleton'; 'entities/enemy/skeleton2.png' = 'skeleton'
    'entities/enemy/slime.webp' = 'slime'; 'biome_content/slime.png' = 'slime'
    'entities/enemy/witch.png' = 'witch'; 'entities/enemy/witch.webp' = 'witch'
    'entities/enemy/yeti.png' = 'yeti'; 'entities/enemy/zombie.png' = 'zombie'
    'biome_content/bogged.png' = 'bogged'; 'biome_content/breeze.png' = 'breeze'
    'biome_content/husk.png' = 'husk'; 'biome_content/magma_cube.png' = 'magma_cube'
    'biome_content/shulker.png' = 'shulker'; 'biome_content/silverfish.png' = 'silverfish'
    'biome_content/stray.png' = 'stray'
}

$rng = New-Object System.Random 3
function Render([string]$id) {
    $face = $faces[$id]
    if (-not $face) { throw "No face '$id'" }
    $bmp = New-Object System.Drawing.Bitmap 16, 16, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    for ($y = 0; $y -lt 16; $y++) {
        $row = $face.rows[$y]
        if ($row.Length -ne 16) { throw "Face '$id' row $y has $($row.Length) pixels" }
        for ($x = 0; $x -lt 16; $x++) {
            $ch = [string]$row[$x]
            $color = if ($ch -eq '.') {
                if (-not $face.bg) { throw "Face '$id' uses '.' without background colours" }
                C $face.bg[$rng.Next(0, $face.bg.Count)]
            } elseif ($face.pal.ContainsKey($ch)) { $face.pal[$ch] } else { throw "Face '$id' has no colour for '$ch'" }
            $bmp.SetPixel($x, $y, $color)
        }
    }
    $bmp
}

function Save-Scaled([System.Drawing.Bitmap]$bmp, [string]$rel) {
    $to = Join-Path $tex $rel
    $w, $h = ((& ffprobe -v error -select_streams v:0 -show_entries stream=width,height -of csv=p=0 $to) -split ',')
    $png = Join-Path $tmp 'face.png'
    $bmp.Save($png, [System.Drawing.Imaging.ImageFormat]::Png)
    $codec = switch ([IO.Path]::GetExtension($to).ToLowerInvariant()) {
        '.jpg' { @('-pix_fmt', 'yuvj420p', '-q:v', '2') }
        '.webp' { @('-c:v', 'libwebp', '-lossless', '1') }
        default { @() }
    }
    & ffmpeg -v error -y -i $png -vf "scale=${w}:${h}:flags=neighbor" -frames:v 1 @codec $to
    if ($LASTEXITCODE -ne 0) { throw "ffmpeg failed for $rel" }
}

foreach ($rel in $targets.Keys) {
    $bmp = Render $targets[$rel]
    Save-Scaled $bmp $rel
    $bmp.Dispose()
}

# Skins: striped or mottled body, the face painted where the face mesh reads it.
function Save-Skin([string]$rel, [int]$size, [int[]]$stripes, [string]$faceId, [double[]]$uv) {
    $skin = New-Object System.Drawing.Bitmap $size, $size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($skin)
    $band = [Math]::Max(1, [int]($size / 16))
    for ($y = 0; $y -lt $size; $y += $band) {
        $brush = New-Object System.Drawing.SolidBrush (C $stripes[[int]($y / $band) % $stripes.Count])
        $g.FillRectangle($brush, 0, $y, $size, $band)
        $brush.Dispose()
    }
    $face = Render $faceId
    $g.InterpolationMode = 'NearestNeighbor'
    $g.PixelOffsetMode = 'Half'
    $rect = [System.Drawing.Rectangle]::new([int]($uv[0] * $size), [int]($uv[1] * $size),
        [int](($uv[2] - $uv[0]) * $size), [int](($uv[3] - $uv[1]) * $size))
    $g.DrawImage($face, $rect)
    $g.Dispose(); $face.Dispose()
    Save-Scaled $skin $rel
    $skin.Dispose()
}
# UV rectangles match face_quad() in src/systems/extra_powers.rs.
$golemUv = 0.0625, 0.0625, 0.125, 0.140625
$beeUv = 0.15625, 0.15625, 0.265625, 0.265625
Save-Skin 'powers/iron_golem.png' 512 @(0xc9c4bc, 0xbdb7ae) 'golem_face' $golemUv
Save-Skin 'powers/bee.png' 256 @(0xf2c43a, 0x2a2018) 'bee_face' $beeUv
Save-Skin 'powers/bee_angry.png' 256 @(0xf2c43a, 0x2a2018) 'bee_face_angry' $beeUv
"Faces drawn: $($targets.Count) heads and 3 skins"
