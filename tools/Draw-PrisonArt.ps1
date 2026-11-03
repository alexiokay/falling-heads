# Draws original pixel art for the space prison event into the free pack
# (other packs fall back to it):
#   textures/prison/bedrock.png        16x16 bedrock at 4x (64x64)
#   textures/prison/trapdoor.png       16x16 iron trapdoor at 4x
#   textures/prison/button.png, button_pressed.png  16x16 buttons at 4x
#   textures/prison/guard.png          16x16 guard face with police cap at 4x
#   textures/prison/captain.png        16x16 guard captain: red cap with a gold band at 4x
#   textures/prison/stars.png          512x512 starfield for the space backdrop
#   textures/prison/earth.png          1024x512 Earth map (equirectangular) for the planet
#   textures/prison/hazard.png         16x16 yellow-black hazard stripes at 4x
#   textures/prison/solar.png          16x16 solar panel cells at 4x
# Fixed random seeds, so reruns give the same pictures.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Split-Path -Parent $PSScriptRoot
$out = Join-Path $root 'assets/packs/free/textures/prison'
New-Item -ItemType Directory -Force $out | Out-Null

function New-Pixels([int]$size = 16) {
    New-Object System.Drawing.Bitmap ($size * 4), ($size * 4), ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
}
function Set-Pixel($bitmap, [int]$x, [int]$y, [System.Drawing.Color]$c) {
    $g = [System.Drawing.Graphics]::FromImage($bitmap)
    $brush = New-Object System.Drawing.SolidBrush $c
    $g.FillRectangle($brush, $x * 4, $y * 4, 4, 4)
    $brush.Dispose(); $g.Dispose()
}
function Rgb([int]$r, [int]$g, [int]$b, [int]$a = 255) { [System.Drawing.Color]::FromArgb($a, $r, $g, $b) }
function Save($bitmap, [string]$name) {
    $bitmap.Save((Join-Path $out $name), [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
}

# Bedrock: blotchy near-black and grey stone.
$rng = New-Object System.Random 41
$b = New-Pixels
for ($y = 0; $y -lt 16; $y++) {
    for ($x = 0; $x -lt 16; $x++) {
        $n = $rng.Next(0, 10)
        $v = if ($n -lt 3) { 28 } elseif ($n -lt 6) { 58 } elseif ($n -lt 9) { 92 } else { 130 }
        Set-Pixel $b $x $y (Rgb $v $v ($v + 4))
    }
}
Save $b 'bedrock.png'

# Iron trapdoor: light frame, four dark window holes, rivets.
$t = New-Pixels
for ($y = 0; $y -lt 16; $y++) {
    for ($x = 0; $x -lt 16; $x++) {
        $edge = $x -eq 0 -or $y -eq 0 -or $x -eq 15 -or $y -eq 15
        $hole = (($x -in 3..6) -or ($x -in 9..12)) -and (($y -in 3..6) -or ($y -in 9..12))
        $c = if ($edge) { Rgb 110 110 116 } elseif ($hole) { Rgb 40 40 46 } else { Rgb 200 200 206 }
        Set-Pixel $t $x $y $c
    }
}
foreach ($p in @(@(1, 1), @(14, 1), @(1, 14), @(14, 14))) { Set-Pixel $t $p[0] $p[1] (Rgb 150 150 156) }
Save $t 'trapdoor.png'

# Buttons: a raised stone button with a red core; pressed is flat and lit green.
foreach ($pressed in $false, $true) {
    $bt = New-Pixels
    for ($y = 0; $y -lt 16; $y++) {
        for ($x = 0; $x -lt 16; $x++) {
            $edge = $x -lt 2 -or $y -lt 2 -or $x -gt 13 -or $y -gt 13
            $core = ($x -in 5..10) -and ($y -in 5..10)
            $c = if ($edge) { Rgb 70 70 74 }
                 elseif ($core -and $pressed) { Rgb 80 230 90 }
                 elseif ($core) { Rgb 220 50 45 }
                 elseif ($pressed) { Rgb 120 120 124 }
                 else { Rgb 160 160 164 }
            Set-Pixel $bt $x $y $c
        }
    }
    if (-not $pressed) { foreach ($x in 5..10) { Set-Pixel $bt $x 5 (Rgb 255 120 110) } }
    Save $bt $(if ($pressed) { 'button_pressed.png' } else { 'button.png' })
}

# Guard: police cap with a gold badge, skin, eyes, moustache.
$face = @(
    'CCCCCCCCCCCCCCCC',
    'CCCCCCCCCCCCCCCC',
    'CCCCCCCYYCCCCCCC',
    'CCCCCCCYYCCCCCCC',
    'VVVVVVVVVVVVVVVV',
    'SSSSSSSSSSSSSSSS',
    'SSSSSSSSSSSSSSSS',
    'SSWWBSSSSSSBWWSS',
    'SSWWBSSSSSSBWWSS',
    'SSSSSSSNNSSSSSSS',
    'SSSSSSSNNSSSSSSS',
    'SSSHHHHHHHHHHSSS',
    'SSSHSSSSSSSSHSSS',
    'SSSSSMMMMMMSSSSS',
    'SSSSSSSSSSSSSSSS',
    'SSSSSSSSSSSSSSSS'
)
$pal = New-Object 'System.Collections.Generic.Dictionary[char,System.Drawing.Color]'
$pal['C'] = Rgb 30 50 120   # cap
$pal['Y'] = Rgb 240 200 60  # badge
$pal['V'] = Rgb 15 20 40    # visor
$pal['S'] = Rgb 214 160 120 # skin
$pal['W'] = Rgb 245 245 245 # eye white
$pal['B'] = Rgb 40 60 140   # iris
$pal['N'] = Rgb 180 120 90  # nose
$pal['H'] = Rgb 70 45 30    # moustache
$pal['M'] = Rgb 140 70 60   # mouth
$gd = New-Pixels
for ($y = 0; $y -lt 16; $y++) { for ($x = 0; $x -lt 16; $x++) { Set-Pixel $gd $x $y $pal[$face[$y][$x]] } }
Save $gd 'guard.png'

# Captain: the guard face under a red cap with a gold band and star.
$captainFace = $face.Clone()
$captainFace[1] = 'GGGGGGGGGGGGGGGG'
$pal['C'] = Rgb 150 22 30
$pal['G'] = Rgb 240 190 50
$cp = New-Pixels
for ($y = 0; $y -lt 16; $y++) { for ($x = 0; $x -lt 16; $x++) { Set-Pixel $cp $x $y $pal[$captainFace[$y][$x]] } }
Save $cp 'captain.png'

# Starfield: black-blue space with stars of a few sizes and a faint nebula.
$s = New-Object System.Drawing.Bitmap 512, 512, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$g = [System.Drawing.Graphics]::FromImage($s)
$g.Clear((Rgb 4 6 16))
$rng = New-Object System.Random 77
for ($i = 0; $i -lt 6; $i++) {
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $cx = $rng.Next(0, 512); $cy = $rng.Next(0, 512); $r = $rng.Next(60, 140)
    $path.AddEllipse($cx - $r, $cy - $r, 2 * $r, 2 * $r)
    $brush = New-Object System.Drawing.Drawing2D.PathGradientBrush $path
    $brush.CenterColor = Rgb 60 40 120 40
    $brush.SurroundColors = @((Rgb 4 6 16 0))
    $g.FillPath($brush, $path); $brush.Dispose(); $path.Dispose()
}
for ($i = 0; $i -lt 260; $i++) {
    $x = $rng.Next(0, 512); $y = $rng.Next(0, 512)
    $size = if ($rng.Next(0, 12) -eq 0) { 3 } elseif ($rng.Next(0, 4) -eq 0) { 2 } else { 1 }
    $v = $rng.Next(150, 256)
    $brush = New-Object System.Drawing.SolidBrush (Rgb $v $v ([math]::Min(255, $v + 20)))
    $g.FillRectangle($brush, $x, $y, $size, $size); $brush.Dispose()
}
$g.Dispose()
$s.Save((Join-Path $out 'stars.png'), [System.Drawing.Imaging.ImageFormat]::Png)
$s.Dispose()
# Earth: deep ocean, blobby green and brown continents, polar ice, soft clouds.
$e = New-Object System.Drawing.Bitmap 1024, 512, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$g = [System.Drawing.Graphics]::FromImage($e)
$g.SmoothingMode = 'AntiAlias'
$ocean = New-Object System.Drawing.Drawing2D.LinearGradientBrush ([System.Drawing.Rectangle]::new(0, 0, 1024, 512)), (Rgb 20 60 140), (Rgb 30 90 170), 90.0
$g.FillRectangle($ocean, 0, 0, 1024, 512); $ocean.Dispose()
$rng = New-Object System.Random 2024
function Blob($g, $rng, [int]$cx, [int]$cy, [int]$r, [System.Drawing.Color]$c, [int]$parts) {
    $brush = New-Object System.Drawing.SolidBrush $c
    for ($i = 0; $i -lt $parts; $i++) {
        $x = $cx + $rng.Next(-$r, $r); $y = $cy + [int]($rng.Next(-$r, $r) * 0.6); $s = $rng.Next([int]($r * 0.4), $r)
        $g.FillEllipse($brush, $x - $s, $y - [int]($s * 0.7), 2 * $s, [int](1.4 * $s))
    }
    $brush.Dispose()
}
foreach ($c in @(@(180, 170, 90), @(260, 330, 80), @(520, 160, 110), @(560, 300, 70), @(780, 180, 100), @(860, 340, 60))) {
    Blob $g $rng $c[0] $c[1] $c[2] (Rgb 60 140 60) 14
    Blob $g $rng $c[0] $c[1] ([int]($c[2] * 0.5)) (Rgb 150 130 80) 5
    Blob $g $rng $c[0] $c[1] ([int]($c[2] * 0.3)) (Rgb 40 110 50) 4
}
$ice = New-Object System.Drawing.SolidBrush (Rgb 235 240 250)
$g.FillRectangle($ice, 0, 0, 1024, 26); $g.FillRectangle($ice, 0, 488, 1024, 24); $ice.Dispose()
for ($i = 0; $i -lt 40; $i++) {
    $cloud = New-Object System.Drawing.SolidBrush (Rgb 255 255 255 120)
    $x = $rng.Next(0, 1024); $y = $rng.Next(40, 470); $w = $rng.Next(40, 160)
    $g.FillEllipse($cloud, $x, $y, $w, [int]($w * 0.25)); $cloud.Dispose()
}
$g.Dispose()
$e.Save((Join-Path $out 'earth.png'), [System.Drawing.Imaging.ImageFormat]::Png)
$e.Dispose()
# Hazard stripes: diagonal yellow and black bands.
$hz = New-Pixels
for ($y = 0; $y -lt 16; $y++) {
    for ($x = 0; $x -lt 16; $x++) {
        $c = if ((($x + $y) % 8) -lt 4) { Rgb 235 190 30 } else { Rgb 25 25 28 }
        Set-Pixel $hz $x $y $c
    }
}
Save $hz 'hazard.png'

# Solar panel: dark blue cells in a silver grid, a light sheen on each cell.
$sp = New-Pixels
for ($y = 0; $y -lt 16; $y++) {
    for ($x = 0; $x -lt 16; $x++) {
        $grid = ($x % 4 -eq 0) -or ($y % 4 -eq 0)
        $sheen = (($x % 4) -eq 1) -and (($y % 4) -eq 1)
        $c = if ($grid) { Rgb 170 175 185 } elseif ($sheen) { Rgb 70 110 190 } else { Rgb 25 45 110 }
        Set-Pixel $sp $x $y $c
    }
}
Save $sp 'solar.png'
"Prison art written to $out"
