# Draws original art for the free pack where a stand-in blob looks wrong:
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
