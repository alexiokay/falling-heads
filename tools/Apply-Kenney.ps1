# Copies Kenney CC0 art into the free pack following tools/kenney-map.csv.
# Download and unzip the Kenney packs first (one folder each):
#   target/kenney/<pack>/files/...   e.g. target/kenney/voxel-pack/files/PNG/Tiles/stone.png
# Images are resized to the size the game already uses for that path (the
# free-pack stand-in), optionally colourized (tint=RRGGBB) and faded
# (alpha=0..1), and saved in the game's format. Vertical strips repeat the
# picture per frame; a mapped texture also rebuilds its *_atlas strip.
# Sounds are copied as they are. Needs ffmpeg/ffprobe on PATH.
param([string]$Kenney = 'target/kenney')
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$free = Join-Path $root 'assets/packs/free'
$kenneyRoot = Join-Path $root $Kenney
$packs = @{ vox = 'voxel-pack'; fx = 'particle-pack'; ani = 'animal-pack-remastered'
    imp = 'impact-sounds'; sci = 'sci-fi-sounds'; rpg = 'rpg-audio'; ui = 'interface-sounds' }
# Preferred variant when a pack has the same file name in several folders.
$prefer = @{ fx = '\\PNG \(Transparent\)\\[^\\]+$'; ani = '\\PNG\\Square\\[^\\]+$' }
$index = @{}

function Find-Source([string]$spec) {
    $key, $name = $spec -split ':', 2
    if (-not $packs.ContainsKey($key)) { throw "Unknown pack key '$key' in $spec" }
    if (-not $index.ContainsKey($key)) {
        $dir = Join-Path $kenneyRoot "$($packs[$key])/files"
        if (-not (Test-Path -LiteralPath $dir)) { throw "Missing Kenney pack: $dir" }
        $index[$key] = @(Get-ChildItem -LiteralPath $dir -File -Recurse)
    }
    $hits = @($index[$key] | Where-Object Name -eq $name)
    if ($prefer.ContainsKey($key)) {
        $best = @($hits | Where-Object { $_.FullName -match $prefer[$key] })
        if ($best) { $hits = $best }
    }
    if (-not $hits) { throw "Not found in $($packs[$key]): $name" }
    $hits[0].FullName
}

function Size([string]$file) {
    $w, $h = ((& ffprobe -v error -select_streams v:0 -show_entries stream=width,height -of csv=p=0 $file) -split ',')
    [int]$w, [int]$h
}

function Make-Image([string]$from, [string]$to, [string]$key, [hashtable]$opt, [int]$w, [int]$h) {
    # Frames side by side (atlas) or stacked (animation strip) repeat the picture.
    $across = 1; $down = 1
    if ($w -ge 2 * $h -and $w % $h -eq 0) { $across = $w / $h }
    elseif ($h -ge 2 * $w -and $h % $w -eq 0) { $down = $h / $w }
    $fw = $w / $across; $fh = $h / $down
    $vf = "format=rgba,scale=${fw}:${fh}:flags=lanczos"
    if ($opt.tint) {
        # White particles take the tint as it is; textured tiles get brightened.
        $boost = if ($key -eq 'fx') { 1.0 } else { 1.35 }
        $r, $g, $b = 0, 2, 4 | ForEach-Object { [math]::Round([Convert]::ToInt32($opt.tint.Substring($_, 2), 16) / 255 * $boost, 3) }
        $vf += ",hue=s=0,colorchannelmixer=rr=${r}:gg=${g}:bb=${b}"
    }
    if ($opt.alpha) { $vf += ",colorchannelmixer=aa=$($opt.alpha)" }
    $frames = $across * $down
    if ($frames -gt 1) { $vf += ",loop=loop=$($frames - 1):size=1:start=0,tile=${across}x${down}" }
    $codec = switch ([IO.Path]::GetExtension($to).ToLowerInvariant()) {
        '.jpg' { @('-pix_fmt', 'yuvj420p', '-q:v', '3') }
        '.webp' { @('-c:v', 'libwebp', '-lossless', '1') }
        default { @() }
    }
    & ffmpeg -v error -y -i $from -vf $vf -frames:v 1 @codec $to
    if ($LASTEXITCODE -ne 0) { throw "ffmpeg failed for $to" }
}

$rows = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'kenney-map.csv') |
    Where-Object { $_ -and -not $_.StartsWith('#') } | ConvertFrom-Csv
$mapped = @{}; $images = 0; $sounds = 0; $atlases = 0
foreach ($row in $rows) { $mapped[$row.target] = $true }
foreach ($row in $rows) {
    $to = Join-Path $free $row.target
    if (-not (Test-Path -LiteralPath $to)) { throw "Game has no file at $($row.target) (run Make-FreePack.ps1 first)" }
    $from = Find-Source $row.source
    $key = ($row.source -split ':')[0]
    $opt = @{}
    foreach ($pair in "$($row.options)" -split '\s+' | Where-Object { $_ }) { $k, $v = $pair -split '=', 2; $opt[$k] = $v }
    if ($to -match '\.ogg$') {
        Copy-Item -LiteralPath $from -Destination $to -Force
        $sounds++
        continue
    }
    $w, $h = Size $to
    Make-Image $from $to $key $opt $w $h
    $images++
    $atlasRel = $row.target -replace '(\.[a-z]+)$', '_atlas$1'
    $atlas = Join-Path $free $atlasRel
    if (-not $mapped.ContainsKey($atlasRel) -and (Test-Path -LiteralPath $atlas)) {
        $aw, $ah = Size $atlas
        Make-Image $from $atlas $key $opt $aw $ah
        $atlases++
    }
}
"Kenney art applied: $images images, $atlases atlas strips, $sounds sounds"
