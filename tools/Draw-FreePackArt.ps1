# Draws original art for the free pack where a stand-in blob looks wrong:
#   textures/biome_content/pointed_dripstone_{up,down}_tip[_atlas].png  16x16 pixel spikes at 4x.
#   textures/effects/rain.png, snow.png  weather sheets (256x1024): streaks, flakes.
#     Each weather particle is a 192x768 quad showing one whole sheet.
#   textures/effects/heart/{full,half,container}.png  9x9 pixel hearts at 4x (36x36).
#   textures/effects/exp/*.png  XP bar background and fill.
# Fixed random seeds, so reruns give the same pictures.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Split-Path -Parent $PSScriptRoot
$out = Join-Path $root 'assets/packs/free/textures/effects'
$w = 256; $h = 1024

function New-Sheet { $b = New-Object System.Drawing.Bitmap $w, $h, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb); $b }

# Rain: streaks fade in from the top, brightest at the leading (lower) end.
$rain = New-Sheet
$g = [System.Drawing.Graphics]::FromImage($rain)
$g.SmoothingMode = 'AntiAlias'
$rng = New-Object System.Random 7
for ($i = 0; $i -lt 22; $i++) {
    $x = $rng.Next(6, $w - 6); $y = $rng.Next(0, $h - 120); $len = $rng.Next(50, 110)
    $top = [System.Drawing.Point]::new($x, $y); $bottom = [System.Drawing.Point]::new($x - 3, $y + $len)
    $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush $top, $bottom,
        ([System.Drawing.Color]::FromArgb(0, 170, 205, 255)), ([System.Drawing.Color]::FromArgb(210, 200, 225, 255))
    $pen = New-Object System.Drawing.Pen $brush, ([single]($rng.Next(2, 4)))
    $g.DrawLine($pen, $top, $bottom)
    $pen.Dispose(); $brush.Dispose()
}
$g.Dispose()
$rain.Save((Join-Path $out 'rain.png'), [System.Drawing.Imaging.ImageFormat]::Png)
$rain.Dispose()

# Snow: round flakes with a soft edge, a few larger ones.
$snow = New-Sheet
$g = [System.Drawing.Graphics]::FromImage($snow)
$g.SmoothingMode = 'AntiAlias'
$rng = New-Object System.Random 11
for ($i = 0; $i -lt 34; $i++) {
    $r = if ($rng.Next(0, 6) -eq 0) { $rng.Next(6, 9) } else { $rng.Next(3, 6) }
    $x = $rng.Next($r, $w - $r); $y = $rng.Next($r, $h - $r)
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $path.AddEllipse($x - $r, $y - $r, 2 * $r, 2 * $r)
    $brush = New-Object System.Drawing.Drawing2D.PathGradientBrush $path
    $brush.CenterColor = [System.Drawing.Color]::FromArgb(240, 255, 255, 255)
    $brush.SurroundColors = @([System.Drawing.Color]::FromArgb(0, 235, 245, 255))
    $g.FillPath($brush, $path)
    $brush.Dispose(); $path.Dispose()
}
$g.Dispose()
$snow.Save((Join-Path $out 'snow.png'), [System.Drawing.Imaging.ImageFormat]::Png)
$snow.Dispose()
"Weather sheets written to $out"

# Hearts: 9x9 pixel art drawn at 4x. '#' fill, 'o' outline, '+' highlight.
$heart = @(
    '.oo...oo.',
    'o+#o.o##o',
    'o+##o###o',
    'o#######o',
    '.o#####o.',
    '..o###o..',
    '...o#o...',
    '....o....',
    '.........'
)
function Save-Heart([string]$name, [scriptblock]$fillColor) {
    $b = New-Object System.Drawing.Bitmap 36, 36, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($b)
    for ($y = 0; $y -lt 9; $y++) {
        for ($x = 0; $x -lt 9; $x++) {
            $c = switch ($heart[$y][$x]) {
                'o' { [System.Drawing.Color]::FromArgb(255, 40, 8, 10) }
                '#' { & $fillColor $x }
                '+' { if ((& $fillColor $x).R -gt 150) { [System.Drawing.Color]::FromArgb(255, 255, 190, 190) } else { & $fillColor $x } }
                default { $null }
            }
            if ($c) {
                $brush = New-Object System.Drawing.SolidBrush $c
                $g.FillRectangle($brush, $x * 4, $y * 4, 4, 4)
                $brush.Dispose()
            }
        }
    }
    $g.Dispose()
    $b.Save((Join-Path $out "heart/$name.png"), [System.Drawing.Imaging.ImageFormat]::Png)
    $b.Dispose()
}
$red = [System.Drawing.Color]::FromArgb(255, 222, 38, 44)
$empty = [System.Drawing.Color]::FromArgb(200, 60, 30, 32)
Save-Heart 'full' { param($x) $red }
Save-Heart 'half' { param($x) if ($x -le 4) { $red } else { $empty } }
Save-Heart 'container' { param($x) $empty }

# XP bar: dark rounded track and a bright green fill, both with a thin outline.
function Save-Bar([string]$name, [int]$width, [System.Drawing.Color]$top, [System.Drawing.Color]$bottom) {
    $b = New-Object System.Drawing.Bitmap $width, 20, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($b)
    $rect = [System.Drawing.Rectangle]::new(0, 0, $width, 20)
    $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush $rect, $top, $bottom, 90.0
    $g.FillRectangle($brush, 1, 1, $width - 2, 18)
    $pen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 10, 14, 10)), 2
    $g.DrawRectangle($pen, 1, 1, $width - 2, 18)
    $pen.Dispose(); $brush.Dispose(); $g.Dispose()
    $b.Save((Join-Path $out "exp/$name.png"), [System.Drawing.Imaging.ImageFormat]::Png)
    $b.Dispose()
}
Save-Bar 'experience_bar_background' 728 ([System.Drawing.Color]::FromArgb(230, 52, 56, 52)) ([System.Drawing.Color]::FromArgb(230, 28, 30, 28))
Save-Bar 'experience_bar_progress' 728 ([System.Drawing.Color]::FromArgb(255, 150, 255, 90)) ([System.Drawing.Color]::FromArgb(255, 70, 190, 40))
Save-Bar 'experience_bar_current' 408 ([System.Drawing.Color]::FromArgb(255, 150, 255, 90)) ([System.Drawing.Color]::FromArgb(255, 70, 190, 40))
"Hearts and XP bar written to $out"

# Pointed dripstone: a brown stone spike, 16x16 pixels at 4x, lit from the left.
# The atlas holds four 64x64 frames; each frame gets its own speckle pattern.
$tiles = Join-Path $root 'assets/packs/free/textures/biome_content'
$halfWidths = @(0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5)
function Draw-Spike([System.Drawing.Graphics]$g, [int]$ox, [bool]$down, [int]$seed) {
    $rng = New-Object System.Random $seed
    for ($row = 0; $row -lt 16; $row++) {
        $hw = $halfWidths[$row]
        $y = if ($down) { 15 - $row } else { $row }
        for ($x = 7 - $hw; $x -le 8 + $hw; $x++) {
            $edge = ($x -eq 7 - $hw) -or ($x -eq 8 + $hw) -or $row -eq 0
            $shade = if ($edge) { 0.5 } elseif ($x -lt 7) { 1.18 } elseif ($x -gt 9) { 0.82 } else { 1.0 }
            $shade *= 0.92 + $rng.NextDouble() * 0.16
            $c = [System.Drawing.Color]::FromArgb(255,
                [math]::Min(255, [int](138 * $shade)), [math]::Min(255, [int](102 * $shade)), [math]::Min(255, [int](76 * $shade)))
            $brush = New-Object System.Drawing.SolidBrush $c
            $g.FillRectangle($brush, $ox + $x * 4, $y * 4, 4, 4)
            $brush.Dispose()
        }
    }
}
foreach ($dir in 'up', 'down') {
    $down = $dir -eq 'down'
    $single = New-Object System.Drawing.Bitmap 64, 64, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($single)
    Draw-Spike $g 0 $down 31
    $g.Dispose()
    $single.Save((Join-Path $tiles "pointed_dripstone_${dir}_tip.png"), [System.Drawing.Imaging.ImageFormat]::Png)
    $single.Dispose()
    $atlas = New-Object System.Drawing.Bitmap 256, 64, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($atlas)
    for ($f = 0; $f -lt 4; $f++) { Draw-Spike $g ($f * 64) $down (31 + $f) }
    $g.Dispose()
    $atlas.Save((Join-Path $tiles "pointed_dripstone_${dir}_tip_atlas.png"), [System.Drawing.Imaging.ImageFormat]::Png)
    $atlas.Dispose()
}
"Dripstone spikes written to $tiles"
