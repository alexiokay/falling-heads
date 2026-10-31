# Builds the FallingHeads icon from the game's own textures:
# assets/packs/minecraft/icon/falling-heads.png (256x256) and falling-heads.ico (16-256 px).
# The hero head falls toward a row of blocks on a night-blue rounded tile.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Split-Path -Parent $PSScriptRoot
$tex = Join-Path $root 'assets/packs/minecraft/textures'
$out = Join-Path $root 'assets/packs/minecraft/icon'
New-Item -ItemType Directory -Force -Path $out | Out-Null

function Load($rel) { [System.Drawing.Image]::FromFile((Join-Path $tex $rel)) }

$size = 256
$bmp = New-Object System.Drawing.Bitmap $size, $size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = 'AntiAlias'
$g.Clear([System.Drawing.Color]::Transparent)

# Rounded tile clip
$radius = 44
$tile = New-Object System.Drawing.Drawing2D.GraphicsPath
$tile.AddArc(0, 0, $radius * 2, $radius * 2, 180, 90)
$tile.AddArc($size - $radius * 2, 0, $radius * 2, $radius * 2, 270, 90)
$tile.AddArc($size - $radius * 2, $size - $radius * 2, $radius * 2, $radius * 2, 0, 90)
$tile.AddArc(0, $size - $radius * 2, $radius * 2, $radius * 2, 90, 90)
$tile.CloseFigure()
$g.SetClip($tile)

# Night sky gradient
$sky = New-Object System.Drawing.Drawing2D.LinearGradientBrush (New-Object System.Drawing.Point 0, 0), (New-Object System.Drawing.Point 0, $size), ([System.Drawing.Color]::FromArgb(255, 38, 62, 112)), ([System.Drawing.Color]::FromArgb(255, 12, 18, 36))
$g.FillRectangle($sky, 0, 0, $size, $size)

# Pixel art from here on: no smoothing
$g.SmoothingMode = 'None'
$g.InterpolationMode = 'NearestNeighbor'
$g.PixelOffsetMode = 'Half'

# Row of blocks at the bottom (one cracked open in the middle)
$blocks = @('blocks/forest/species/grass_block_side.png', 'blocks/general/diamond_ore.png', 'blocks/general/stone.png', 'blocks/general/gold_ore.png')
for ($i = 0; $i -lt 4; $i++) {
    $img = Load $blocks[$i]
    $g.DrawImage($img, (New-Object System.Drawing.Rectangle ($i * 64), 192, 64, 64))
    $img.Dispose()
}
# Shading line on top of the blocks
$g.FillRectangle((New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(70, 0, 0, 0))), 0, 192, $size, 6)

# Motion streaks above the head (it is falling)
$g.SmoothingMode = 'AntiAlias'
foreach ($s in @(@(84, 10, 70), @(124, 0, 92), @(166, 14, 64))) {
    $streak = New-Object System.Drawing.Drawing2D.LinearGradientBrush (New-Object System.Drawing.Point 0, $s[1]), (New-Object System.Drawing.Point 0, ($s[1] + $s[2])), ([System.Drawing.Color]::FromArgb(0, 255, 255, 255)), ([System.Drawing.Color]::FromArgb(150, 210, 230, 255))
    $g.FillRectangle($streak, $s[0], $s[1], 8, $s[2])
}
$g.SmoothingMode = 'None'

# The hero head, slightly tilted, with a dark outline and a drop shadow
$head = Load 'entities/hero.jpg'
$state = $g.Save()
$g.TranslateTransform(128, 118)
$g.RotateTransform(-9)
$g.FillRectangle((New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(110, 0, 0, 0))), -50, -46, 112, 112)
$g.FillRectangle((New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 20, 14, 10))), -60, -60, 120, 120)
$g.DrawImage($head, (New-Object System.Drawing.Rectangle -56, -56, 112, 112))
$g.Restore($state)
$head.Dispose()

# Thin rim around the tile
$g.ResetClip()
$g.SmoothingMode = 'AntiAlias'
$g.DrawPath((New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(140, 160, 200, 255)), 3), $tile)
$g.Dispose()

$png = Join-Path $out 'falling-heads.png'
$bmp.Save($png, [System.Drawing.Imaging.ImageFormat]::Png)

# ICO with PNG-encoded entries
$sizes = @(16, 24, 32, 48, 64, 128, 256)
$entries = foreach ($s in $sizes) {
    $small = New-Object System.Drawing.Bitmap $s, $s, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $sg = [System.Drawing.Graphics]::FromImage($small)
    $sg.InterpolationMode = if ($s -le 48) { 'HighQualityBicubic' } else { 'NearestNeighbor' }
    $sg.PixelOffsetMode = 'Half'
    $sg.DrawImage($bmp, 0, 0, $s, $s)
    $sg.Dispose()
    $ms = New-Object System.IO.MemoryStream
    $small.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $small.Dispose()
    , @($s, $ms.ToArray())
}
$bmp.Dispose()
$fs = [System.IO.File]::Create((Join-Path $out 'falling-heads.ico'))
$w = New-Object System.IO.BinaryWriter $fs
$w.Write([UInt16]0); $w.Write([UInt16]1); $w.Write([UInt16]$entries.Count)
$offset = 6 + 16 * $entries.Count
foreach ($e in $entries) {
    $dim = if ($e[0] -ge 256) { 0 } else { $e[0] }
    $w.Write([Byte]$dim); $w.Write([Byte]$dim); $w.Write([Byte]0); $w.Write([Byte]0)
    $w.Write([UInt16]1); $w.Write([UInt16]32); $w.Write([UInt32]$e[1].Length); $w.Write([UInt32]$offset)
    $offset += $e[1].Length
}
foreach ($e in $entries) { $w.Write($e[1]) }
$w.Close()
"Icon written to $out"
