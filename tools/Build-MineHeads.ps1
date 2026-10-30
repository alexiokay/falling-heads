param()
$ErrorActionPreference='Stop'
$gameRoot=Split-Path -Parent $PSScriptRoot
Push-Location $gameRoot
try {
    & cargo build --locked
    if ($LASTEXITCODE) { throw 'MineHeads build failed.' }
    $sdk=Join-Path $gameRoot '../../south-balcony-twin/vendor/DLSS'
    foreach ($name in @('nvngx_dlss.dll','nvngx_dlssd.dll')) {
        Copy-Item -LiteralPath (Join-Path $sdk "lib/Windows_x86_64/rel/$name") -Destination (Join-Path $gameRoot "target/debug/$name")
    }
    & (Join-Path $PSScriptRoot "Stage-Streamline.ps1") -Destination (Join-Path $gameRoot "target/debug")
} finally { Pop-Location }
