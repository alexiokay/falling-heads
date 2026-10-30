# Fills assets/packs/free with original stand-ins for every texture, sound and
# icon another pack has and the free pack lacks, so the game runs from the free
# pack alone (the public repository has only that pack).
#   Images keep their size, format and average colour: a plain noisy tile, or
#   a soft blob when the original has transparency.
#   Sounds are short synthesized blips chosen by name (explosion, step, mob, note).
# Files already in the free pack (own work, CC0 replacements) are never touched.
# Needs ffmpeg/ffprobe on PATH.  Usage: pwsh tools/Make-FreePack.ps1 [-Source minecraft] [-DryRun]
param([string]$Source = 'minecraft', [switch]$DryRun)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$src = Join-Path $root "assets/packs/$Source"
$dst = Join-Path $root 'assets/packs/free'
if (-not (Test-Path -LiteralPath $src)) { throw "Pack not found: $src" }
$skip = '(\.(json|txt|bat|ini|ico|mp3)$)|(^licenses[\\/])'
$tmp = New-TemporaryFile

function Probe([string]$file, [string]$entries, [string]$stream) {
    $opts = @('-v', 'error')
    if ($stream) { $opts += @('-select_streams', $stream) }
    (& ffprobe @opts -show_entries $entries -of csv=p=0 $file | Select-Object -First 1).Trim()
}

# Stable pseudo-random number per file name, so reruns give the same sounds.
function NameHash([string]$name) {
    $h = 17
    foreach ($c in $name.ToCharArray()) { $h = ($h * 31 + [int]$c) % 1000003 }
    $h
}

function Make-Image([string]$from, [string]$to) {
    $w, $h = (Probe $from 'stream=width,height' 'v:0') -split ','
    & ffmpeg -v error -y -i $from -vf 'format=rgba,scale=1:1:flags=area' -f rawvideo $tmp.FullName
    $px = [IO.File]::ReadAllBytes($tmp.FullName)
    $hex = '{0:X2}{1:X2}{2:X2}' -f $px[0], $px[1], $px[2]
    $ext = [IO.Path]::GetExtension($to).ToLowerInvariant()
    if ($px[3] -lt 250 -and $ext -ne '.jpg') {
        # Transparent original (particles, overlays, mob parts): soft round blob.
        $vf = "color=c=0x${hex}:s=${w}x${h},format=rgba,geq=r='r(X,Y)':g='g(X,Y)':b='b(X,Y)':a='255*clip(1.25-1.25*hypot((X-W/2)/(W/2),(Y-H/2)/(H/2)),0,1)'"
    } else {
        $vf = "color=c=0x${hex}:s=${w}x${h},format=rgb24,noise=alls=22:allf=u,drawbox=x=0:y=0:w=iw:h=ih:color=black@0.3:t=1"
    }
    $codec = switch ($ext) {
        '.jpg' { @('-pix_fmt', 'yuvj420p', '-q:v', '3') }
        '.webp' { @('-c:v', 'libwebp', '-lossless', '1') }
        default { @() }
    }
    & ffmpeg -v error -y -f lavfi -i $vf -frames:v 1 @codec $to
}

function Make-Sound([string]$from, [string]$to) {
    $name = [IO.Path]::GetFileNameWithoutExtension($from).ToLowerInvariant()
    $path = $from.ToLowerInvariant()
    $d = [math]::Round([math]::Min(1.5, [math]::Max(0.12, [double](Probe $from 'format=duration' $null))), 2)
    $f = 180 + (NameHash $name) % 520
    $af = if ($path -match 'explode|fuse|thunder|whirlpool|bubble|wind|portal|ignite') {
        "anoisesrc=d=${d}:c=brown:a=0.6,lowpass=f=700,afade=t=out:st=0:d=$d"
    } elseif ($path -match 'step|break|dig|place|carve|click|slide|fall|stone|snow|sand|leaves|wood|glass|pop') {
        $d = 0.14
        "anoisesrc=d=${d}:c=pink:a=0.5,highpass=f=900,afade=t=out:st=0:d=$d"
    } elseif ($path -match 'note|bell|harp|chime|bass|snare|resonate|repair') {
        "sine=f=${f}:d=${d},afade=t=out:st=0.03:d=$([math]::Max(0.05, $d - 0.03)),volume=0.5"
    } else {
        # Mobs and everything else: a short wobbling tone, pitched per sound.
        $d = [math]::Min($d, 0.6)
        "sine=f=${f}:d=${d},vibrato=f=9:d=0.5,afade=t=out:st=0:d=$d,volume=0.45"
    }
    & ffmpeg -v error -y -f lavfi -i $af -ac 1 -ar 44100 -c:a libvorbis -q:a 3 $to
}

$made = 0; $kept = 0
Get-ChildItem -LiteralPath $src -File -Recurse | ForEach-Object {
    $rel = $_.FullName.Substring($src.Length + 1)
    if ($rel -match $skip) { return }
    $to = Join-Path $dst $rel
    if (Test-Path -LiteralPath $to) { $kept++; return }
    $ext = $_.Extension.ToLowerInvariant()
    if ($ext -notin '.png', '.jpg', '.webp', '.ogg') { Write-Warning "No stand-in for $rel"; return }
    if ($DryRun) { "would make $rel"; $made++; return }
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $to) | Out-Null
    if ($ext -eq '.ogg') { Make-Sound $_.FullName $to } else { Make-Image $_.FullName $to }
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $to)) { throw "ffmpeg failed for $rel" }
    $made++
}
Remove-Item -LiteralPath $tmp
"Free pack: $made stand-ins made, $kept files already there"
