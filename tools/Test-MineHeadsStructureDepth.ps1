param()
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
Set-Location $root
if(Get-Process -Name MineHeads,falling-heads,overhaul-verification,depth-verification -ErrorAction SilentlyContinue){throw 'Close MineHeads before the native structure gallery.'}
$settings=Join-Path $root 'graphics-settings.json'
$saved=if(Test-Path -LiteralPath $settings){[IO.File]::ReadAllBytes($settings)}else{$null}
try {
    & ./tools/Test-MineHeadsOverhaul.ps1 -SkipBuild -Scene overhaul
    $log=Get-Content 'target/overhaul-run-err.log' -Raw
    if($log -notmatch '\[STRUCTURE DEPTH\].*rt=false' -or $log -notmatch '\[STRUCTURE DEPTH\].*rt=true'){throw 'Missing raster/RT solid wall verification'}
} finally {
    if($null -ne $saved){[IO.File]::WriteAllBytes($settings,$saved)}
    elseif(Test-Path -LiteralPath $settings){Remove-Item -LiteralPath $settings}
}
