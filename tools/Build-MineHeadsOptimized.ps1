param([switch]$SkipChecks)
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
Set-Location $root
if(-not $SkipChecks){& ./tools/Check-MineHeadsArchitecture.ps1}
# Dependencies already use opt-level=3. Optimize only the application while
# retaining the tested development ABI, assertions and full Windows PDBs.
# This avoids rebuilding the engine merely to benchmark application optimization.
& cargo rustc --locked --offline -j 1 --bin falling-heads -- -C opt-level=3
if($LASTEXITCODE){throw 'Optimized application build failed'}
$destination=Join-Path $root 'target/optimized-candidate'
New-Item -ItemType Directory -Path $destination -Force | Out-Null
Copy-Item -LiteralPath target/debug/falling-heads.exe -Destination $destination
Copy-Item -LiteralPath target/debug/falling_heads.pdb -Destination $destination
foreach($dll in @('nvngx_dlss.dll','nvngx_dlssd.dll','sl.interposer.dll','sl.common.dll','sl.dlss_g.dll','sl.reflex.dll','sl.pcl.dll','nvngx_dlssg.dll','NvLowLatencyVk.dll')) {
    Copy-Item -LiteralPath (Join-Path 'target/debug' $dll) -Destination $destination
}
Copy-Item -LiteralPath target/debug/licenses -Destination $destination -Recurse -Force
Get-FileHash (Join-Path $destination 'falling-heads.exe') -Algorithm SHA256
