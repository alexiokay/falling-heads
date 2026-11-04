# Draws original 16x16 pixel-art block textures (at 4x) for the voxel block
# rocket shared by the LAUNCHER power and the space prison escape, into the free
# pack (other packs fall back to it):
#   textures/powers/rocket_hull.png    riveted iron plate
#   textures/powers/rocket_paint.png   red painted plate
#   textures/powers/rocket_glass.png   glass block: light frame, see-through pane
#   textures/powers/rocket_window.png  porthole: iron plate round a glass pane
#   textures/powers/rocket_nozzle.png  dark engine nozzle rings
#   textures/powers/rocket_flame.png   pixel flame block (used emissive)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Split-Path -Parent $PSScriptRoot
$out = Join-Path $root 'assets/packs/free/textures/powers'
New-Item -ItemType Directory -Force $out | Out-Null

function Rgb([int]$r, [int]$g, [int]$b, [int]$a = 255) { [System.Drawing.Color]::FromArgb($a, $r, $g, $b) }

# Palettes are case-sensitive (PowerShell hash literals are not).
function New-Pal { New-Object 'System.Collections.Generic.Dictionary[char,System.Drawing.Color]' }
function Draw-Sprite([string[]]$rows, $pal, [string]$name, [int]$scale = 4) {
    $h = $rows.Count; $w = $rows[0].Length
    $b = New-Object System.Drawing.Bitmap ($w * $scale), ($h * $scale), ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($b)
    $g.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceCopy
    for ($y = 0; $y -lt $h; $y++) {
        if ($rows[$y].Length -ne $w) { throw "$name row $y is $($rows[$y].Length) wide, expected $w" }
        for ($x = 0; $x -lt $w; $x++) {
            $c = $rows[$y][$x]
            if ($c -eq '.') { continue }
            if (-not $pal.ContainsKey($c)) { throw "$name has no colour for '$c'" }
            $brush = New-Object System.Drawing.SolidBrush $pal[$c]
            $g.FillRectangle($brush, $x * $scale, $y * $scale, $scale, $scale)
            $brush.Dispose()
        }
    }
    $g.Dispose()
    $b.Save((Join-Path $out $name), [System.Drawing.Imaging.ImageFormat]::Png)
    $b.Dispose()
}

$plate = @(
    'KKKKKKKKKKKKKKKK',
    'KHHHHHHHHHHHHHMK',
    'KHRLLLLLLLLLLRDK',
    'KHLLLLMLLLLLLLDK',
    'KHLLLLLLLLLMLLDK',
    'KHLLMLLLLLLLLLDK',
    'KHLLLLLLLMLLLLDK',
    'KHLLLLLLLLLLLLDK',
    'KHLLLLLMLLLLLLDK',
    'KHLLLLLLLLLLMLDK',
    'KHLMLLLLLLLLLLDK',
    'KHLLLLLLMLLLLLDK',
    'KHLLLLLLLLLLLLDK',
    'KHRLLLLLLLLLLRDK',
    'KMDDDDDDDDDDDDDK',
    'KKKKKKKKKKKKKKKK'
)

$iron = New-Pal
$iron['K'] = Rgb 72 76 86
$iron['H'] = Rgb 244 246 248
$iron['L'] = Rgb 216 220 226
$iron['M'] = Rgb 192 196 204
$iron['D'] = Rgb 152 157 166
$iron['R'] = Rgb 112 116 126
Draw-Sprite $plate $iron 'rocket_hull.png'

$paint = New-Pal
$paint['K'] = Rgb 92 20 18
$paint['H'] = Rgb 255 112 92
$paint['L'] = Rgb 216 50 38
$paint['M'] = Rgb 192 40 30
$paint['D'] = Rgb 150 28 22
$paint['R'] = Rgb 120 24 20
Draw-Sprite $plate $paint 'rocket_paint.png'

$glass = New-Pal
$glass['K'] = Rgb 90 120 140
$glass['F'] = Rgb 205 230 245
$glass['G'] = Rgb 170 215 245 60
$glass['W'] = Rgb 255 255 255 200
Draw-Sprite @(
    'KKKKKKKKKKKKKKKK',
    'KFFFFFFFFFFFFFFK',
    'KFGGGGGGGGGGGGFK',
    'KFGWWGGGGGGGGGFK',
    'KFGWGGGGGGGGGGFK',
    'KFGGGGGGGGGGGGFK',
    'KFGGGGGGGGGGGGFK',
    'KFGGGGGGGGGGGGFK',
    'KFGGGGGGGGGGGGFK',
    'KFGGGGGGGGGGGGFK',
    'KFGGGGGGGGGGGGFK',
    'KFGGGGGGGGGGGGFK',
    'KFGGGGGGGGGGGWFK',
    'KFGGGGGGGGGGWWFK',
    'KFFFFFFFFFFFFFFK',
    'KKKKKKKKKKKKKKKK'
) $glass 'rocket_glass.png'

$window = New-Pal
foreach ($k in $iron.Keys) { $window[$k] = $iron[$k] }
$window['F'] = Rgb 60 64 74
$window['G'] = Rgb 150 205 245 70
$window['W'] = Rgb 255 255 255 210
Draw-Sprite @(
    'KKKKKKKKKKKKKKKK',
    'KHHHHHHHHHHHHHMK',
    'KHRLLFFFFFFLLRDK',
    'KHLLFGGGGGGFLLDK',
    'KHLFGWWGGGGGFLDK',
    'KHFGWGGGGGGGGFDK',
    'KHFGGGGGGGGGGFDK',
    'KHFGGGGGGGGGGFDK',
    'KHFGGGGGGGGGGFDK',
    'KHFGGGGGGGGGGFDK',
    'KHFGGGGGGGGWGFDK',
    'KHLFGGGGGGWWFLDK',
    'KHLLFGGGGGGFLLDK',
    'KHRLLFFFFFFLLRDK',
    'KMDDDDDDDDDDDDDK',
    'KKKKKKKKKKKKKKKK'
) $window 'rocket_window.png'

$nozzle = New-Pal
$nozzle['K'] = Rgb 28 28 32
$nozzle['D'] = Rgb 58 60 66
$nozzle['M'] = Rgb 94 97 105
$nozzle['L'] = Rgb 132 136 144
$nozzle['O'] = Rgb 255 150 40
Draw-Sprite @(
    'KKKKKKKKKKKKKKKK',
    'KLLLLLLLLLLLLLLK',
    'KLMMMMMMMMMMMMLK',
    'KLMDDDDDDDDDDMLK',
    'KLMDKKKKKKKKDMLK',
    'KLMDKDDDDDDKDMLK',
    'KLMDKDMMMMDKDMLK',
    'KLMDKDMOOMDKDMLK',
    'KLMDKDMOOMDKDMLK',
    'KLMDKDMMMMDKDMLK',
    'KLMDKDDDDDDKDMLK',
    'KLMDKKKKKKKKDMLK',
    'KLMDDDDDDDDDDMLK',
    'KLMMMMMMMMMMMMLK',
    'KLLLLLLLLLLLLLLK',
    'KKKKKKKKKKKKKKKK'
) $nozzle 'rocket_nozzle.png'

$flame = New-Pal
$flame['R'] = Rgb 225 60 20
$flame['O'] = Rgb 255 140 25
$flame['Y'] = Rgb 255 225 80
$flame['W'] = Rgb 255 250 215
Draw-Sprite @(
    'RRORRRRORRRRRORR',
    'ROOOORROOOROOOOR',
    'ROOYOOOOOOOOYOOR',
    'OOYYYOOYYOOYYYOO',
    'ROOYYYYYYYYYYOOR',
    'ROYYYWYYYYWYYYOR',
    'OOYYWWWYYWWWYYOO',
    'ROYYWWWWWWWWYYOR',
    'ROYYWWWWWWWWYYOR',
    'OOYYWWWYYWWWYYOO',
    'ROYYYWYYYYWYYYOR',
    'ROOYYYYYYYYYYOOR',
    'OOYYYOOYYOOYYYOO',
    'ROOYOOOOOOOOYOOR',
    'ROOOORROOOROOOOR',
    'RRORRRRORRRRRORR'
) $flame 'rocket_flame.png'
"Rocket block textures written to $out"
