param([int]$WaitForBuild=0,[string]$Tag='visual-baseline',[switch]$ShadowOnly,[switch]$FireImpact,[switch]$BiomeAtmosphere,[switch]$ForestTrees,[switch]$ForestWeatherRt)
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
Set-Location $root
if($WaitForBuild){Wait-Process -Id $WaitForBuild -ErrorAction SilentlyContinue}
$build=Start-Process (Get-Command cargo.exe).Source -ArgumentList @('build','--offline') -WindowStyle Hidden -PassThru -RedirectStandardOutput target/visual-build-out.log -RedirectStandardError target/visual-build-err.log
$null=$build.Handle
$build.WaitForExit()
if($build.ExitCode){throw 'Visual probe build failed'}
$env:BEVY_ASSET_ROOT=$root
$testExe=Join-Path $root "target/debug/$Tag.exe"
Copy-Item -LiteralPath target/debug/falling-heads.exe -Destination $testExe
$started=Get-Date
$probeArgs=@('--visual-smoke-test')
if($ShadowOnly){$probeArgs+='--shadow-check'}
if($FireImpact){$probeArgs+='--fire-impact-test'}
if($BiomeAtmosphere){$probeArgs+='--biome-atmosphere-test'}
if($ForestTrees){$probeArgs+='--forest-trees-test'}
if($ForestWeatherRt){$probeArgs+='--forest-weather-rt-test'}
$probe=Start-Process $testExe -ArgumentList $probeArgs -WindowStyle Hidden -PassThru -RedirectStandardOutput "target/$Tag-out.log" -RedirectStandardError "target/$Tag-err.log"
$null=$probe.Handle
Write-Output "Visual probe PID $($probe.Id)"
if(-not $probe.WaitForExit(150000)){Stop-Process -Id $probe.Id;throw 'Visual probe timed out'}
Write-Output "Visual probe exit $($probe.ExitCode)"
if($probe.ExitCode){throw 'Visual probe failed'}
$log=Get-Content -LiteralPath "target/$Tag-err.log" -Raw
if($log -match '(?m)^\d{4}-[^\r\n]*Z\s+ERROR\s|panicked at|Validation Error' -or ($log -notmatch '\[VISUAL PROFILE\] snow' -and $log -notmatch '\[VISUAL SHADOW\] completed' -and $log -notmatch '\[FIRE IMPACT\] completed' -and $log -notmatch '\[BIOME ATMOSPHERE\] completed' -and $log -notmatch '\[FOREST TREES\] completed' -and $log -notmatch '\[FOREST WEATHER RT\] completed') -or $log -notmatch '\[FG\] slShutdown result=0'){throw 'Visual probe reported a rendering error or ended before completing'}
$names=@('steam','sr','no-ao','no-shadows','native')
if(-not $ShadowOnly){$names+=@('snow','dragon','dragon-fire','dragon-turn-quarter','dragon-turn-middle','dragon-turn-right')}
if($FireImpact){$names=@('fire-impact','fire-release','fire-off','fire-settled')}
if($BiomeAtmosphere){$names=@('nether-day','nether-night','forest-mist','forest-clear','forest-rt')}
if($ForestTrees){$names=@('trees-natural','trees-species','trees-rt')}
if($ForestWeatherRt){$names=@('forest-density-clear','forest-density-snow','forest-density-rain','end-rt-still','end-rt-moving','end-rt-moving-later','end-rt-no-torches','end-raster-shadows','end-raster-no-shadows','end-rt-quality','end-rt-quality-fg-input')}
foreach($name in $names){if((Get-Item -LiteralPath "target/visual-$name.png").LastWriteTime -lt $started){throw "Stale capture: $name"};Copy-Item -LiteralPath "target/visual-$name.png" -Destination "target/$Tag-$name.png"}

