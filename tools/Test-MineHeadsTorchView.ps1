param([int]$WaitForTests=0)
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
Set-Location $root
if($WaitForTests){Wait-Process -Id $WaitForTests -ErrorAction SilentlyContinue}
if((Get-Content target/torch-view-tests-out.log -Raw) -notmatch 'test result: ok\.') {throw 'Unit tests did not pass'}
$build=Start-Process cargo.exe -ArgumentList @('build','--offline') -WindowStyle Hidden -PassThru -RedirectStandardOutput target/torch-view-build-out.log -RedirectStandardError target/torch-view-build-err.log
$null=$build.Handle
$build.WaitForExit()
if($build.ExitCode){throw 'Torch-view build failed'}
$env:BEVY_ASSET_ROOT=$root
$exe=Join-Path $root 'target/debug/torch-view-probe.exe'
Copy-Item -LiteralPath target/debug/falling-heads.exe -Destination $exe
$started=Get-Date
$probe=Start-Process $exe -ArgumentList @('--visual-smoke-test','--torch-view-test') -WindowStyle Hidden -PassThru -RedirectStandardOutput target/torch-view-out.log -RedirectStandardError target/torch-view-err.log
$null=$probe.Handle
"Torch-view probe PID $($probe.Id)"
if(-not $probe.WaitForExit(120000)){Stop-Process -Id $probe.Id;throw 'Torch-view timeout'}
if($probe.ExitCode){throw "Torch-view exit $($probe.ExitCode)"}
$log=Get-Content target/torch-view-err.log -Raw
if($log -match 'panicked at|Validation Error|(?m)^\d{4}-[^\r\n]*Z\s+ERROR\s' -or $log -notmatch '\[TORCH VIEW\] completed' -or $log -notmatch '\[FG\] slShutdown result=0'){throw 'Incomplete or failed torch-view probe'}
foreach($stage in @('wide-native','wide-moved','standard','dlss45','wide-rt','wide-rt-moved')) {
    $file=Get-Item "target/visual-torch-$stage.png"
    if($file.LastWriteTime -lt $started){throw "Stale capture: $stage"}
}
'Torch-view probe passed: six fresh captures'
