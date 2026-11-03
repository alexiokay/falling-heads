# Draws original pixel-art sprites for the space prison's power-ups into the
# free pack (other packs fall back to it):
#   textures/prison/item_fuel.png    16x16 nitro bottle with a flame label, at 4x
#   textures/prison/item_shield.png  16x16 energy shield emblem, at 4x
#   textures/prison/item_key.png     16x16 key card with a chip and stripes, at 4x
#   textures/prison/aura.png         64x64 soft white glow (tinted per item in game)
#   textures/prison/sparkle.png      8x8 four-point sparkle, at 4x
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Split-Path -Parent $PSScriptRoot
$out = Join-Path $root 'assets/packs/free/textures/prison'
New-Item -ItemType Directory -Force $out | Out-Null

function Rgb([int]$r, [int]$g, [int]$b, [int]$a = 255) { [System.Drawing.Color]::FromArgb($a, $r, $g, $b) }

# Palettes are case-sensitive (PowerShell hash literals are not).
function Pal([hashtable]$colors) {
    $d = New-Object 'System.Collections.Generic.Dictionary[char,System.Drawing.Color]'
    foreach ($k in $colors.Keys) { $d[[char]$k] = $colors[$k] }
    $d
}
function Draw-Sprite([string[]]$rows, $pal, [string]$name, [int]$scale = 4) {
    $h = $rows.Count; $w = $rows[0].Length
    $b = New-Object System.Drawing.Bitmap ($w * $scale), ($h * $scale), ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($b)
    for ($y = 0; $y -lt $h; $y++) {
        if ($rows[$y].Length -ne $w) { throw "$name row $y is $($rows[$y].Length) wide, expected $w" }
        for ($x = 0; $x -lt $w; $x++) {
            $c = $rows[$y][$x]
            if ($c -eq '.') { continue }
            $brush = New-Object System.Drawing.SolidBrush $pal[$c]
            $g.FillRectangle($brush, $x * $scale, $y * $scale, $scale, $scale)
            $brush.Dispose()
        }
    }
    $g.Dispose()
    $b.Save((Join-Path $out $name), [System.Drawing.Imaging.ImageFormat]::Png)
    $b.Dispose()
}

# Nitro bottle: blue body, silver valve, white label with a flame.
$fuelPal = New-Object 'System.Collections.Generic.Dictionary[char,System.Drawing.Color]'
$fuelPal['K'] = Rgb 18 20 34
$fuelPal['S'] = Rgb 205 210 220
$fuelPal['s'] = Rgb 120 126 138
$fuelPal['B'] = Rgb 40 110 220
$fuelPal['b'] = Rgb 22 60 140
$fuelPal['L'] = Rgb 140 200 255
$fuelPal['W'] = Rgb 240 242 246
$fuelPal['O'] = Rgb 255 120 20
$fuelPal['Y'] = Rgb 255 220 60
Draw-Sprite @(
    '......KKKK......',
    '.....KSSSsK.....',
    '......KSsK......',
    '.....KKKKKK.....',
    '....KLBBBBbK....',
    '....KLBBBBbK....',
    '....KLWWWWbK....',
    '....KLWOOWbK....',
    '....KLWOYWbK....',
    '....KLWYOWbK....',
    '....KLWWWWbK....',
    '....KLBBBBbK....',
    '....KLBBBBbK....',
    '....KLBBBBbK....',
    '.....KbbbbK.....',
    '......KKKK......'
) $fuelPal 'item_fuel.png'

# Energy shield: cyan shield with a white core and rim light.
$shieldPal = New-Object 'System.Collections.Generic.Dictionary[char,System.Drawing.Color]'
$shieldPal['K'] = Rgb 10 30 50
$shieldPal['C'] = Rgb 60 200 255
$shieldPal['c'] = Rgb 20 110 170
$shieldPal['w'] = Rgb 230 250 255
$shieldPal['B'] = Rgb 30 90 220
Draw-Sprite @(
    '................',
    '...KKKKKKKKKK...',
    '..KwCCCCCCCCcK..',
    '..KwCCCCCCCCcK..',
    '..KwCCwwwwCCcK..',
    '..KCCCwBBwCCcK..',
    '..KCCCwBBwCCcK..',
    '..KCCwwwwwwCcK..',
    '...KCCCwwCCcK...',
    '...KCCCwwCCcK...',
    '....KCCwwCcK....',
    '....KCCCCCcK....',
    '.....KCCCcK.....',
    '......KCcK......',
    '.......KK.......',
    '................'
) $shieldPal 'item_shield.png'

# Key card: white card, red header, gold chip, blue magnetic stripe.
$keyPal = New-Object 'System.Collections.Generic.Dictionary[char,System.Drawing.Color]'
$keyPal['K'] = Rgb 25 25 35
$keyPal['R'] = Rgb 220 40 40
$keyPal['W'] = Rgb 240 240 245
$keyPal['G'] = Rgb 240 190 50
$keyPal['g'] = Rgb 170 120 20
$keyPal['B'] = Rgb 40 80 200
Draw-Sprite @(
    '................',
    '................',
    '................',
    '.KKKKKKKKKKKKKK.',
    '.KRRRRRRRRRRRRK.',
    '.KWWWWWWWWWWWWK.',
    '.KWGGGWWWWWWWWK.',
    '.KWGgGWWWBBBBWK.',
    '.KWGGGWWWWWWWWK.',
    '.KWWWWWWWBBBWWK.',
    '.KBBBBBBBBBBBBK.',
    '.KWWWWWWWWWWWWK.',
    '.KKKKKKKKKKKKKK.',
    '................',
    '................',
    '................'
) $keyPal 'item_key.png'

# Sparkle: a small four-point star.
$sparkPal = New-Object 'System.Collections.Generic.Dictionary[char,System.Drawing.Color]'
$sparkPal['W'] = Rgb 255 255 255
$sparkPal['w'] = Rgb 255 255 255 150
Draw-Sprite @(
    '...w....',
    '...W....',
    '..wWw...',
    'wWWWWWw.',
    '..wWw...',
    '...W....',
    '...w....',
    '........'
) $sparkPal 'sparkle.png'

# Aura: a soft round white glow, fading to nothing at the edge.
$aura = New-Object System.Drawing.Bitmap 64, 64, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
for ($y = 0; $y -lt 64; $y++) {
    for ($x = 0; $x -lt 64; $x++) {
        $d = [math]::Sqrt([math]::Pow($x - 31.5, 2) + [math]::Pow($y - 31.5, 2)) / 32.
        $a = [math]::Max(0., 1. - $d)
        $aura.SetPixel($x, $y, (Rgb 255 255 255 ([int](255 * [math]::Pow($a, 2.2)))))
    }
}
$aura.Save((Join-Path $out 'aura.png'), [System.Drawing.Imaging.ImageFormat]::Png)
$aura.Dispose()
"Prison item sprites written to $out"
