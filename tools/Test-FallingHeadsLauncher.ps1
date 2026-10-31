param([int]$WaitForTests=0)
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
Set-Location $root
if($WaitForTests){Wait-Process -Id $WaitForTests -ErrorAction SilentlyContinue}
if((Get-Content target/launcher-tests-out.log -Raw) -notmatch 'test result: ok\.') {throw 'Launcher unit tests did not pass'}
$build=Start-Process cargo.exe -ArgumentList @('build','--offline') -WindowStyle Hidden -PassThru -RedirectStandardOutput target/launcher-build-out.log -RedirectStandardError target/launcher-build-err.log
$null=$build.Handle
$build.WaitForExit()
if($build.ExitCode){throw 'Launcher build failed'}
$env:BEVY_ASSET_ROOT=$root
$exe=Join-Path $root 'target/debug/launcher-probe.exe'
Copy-Item -LiteralPath target/debug/falling-heads.exe -Destination $exe
$started=Get-Date
$probe=Start-Process $exe -ArgumentList @('--visual-smoke-test','--launcher-test') -WindowStyle Hidden -PassThru -RedirectStandardOutput target/launcher-probe-out.log -RedirectStandardError target/launcher-probe-err.log
$null=$probe.Handle
"Launcher probe PID $($probe.Id)"
if(-not $probe.WaitForExit(125000)){Stop-Process -Id $probe.Id;throw 'Launcher probe timeout'}
if($probe.ExitCode){throw "Launcher probe exit $($probe.ExitCode)"}
$log=Get-Content target/launcher-probe-err.log -Raw
if($log -match 'panicked at|Validation Error|(?m)^\d{4}-[^\r\n]*Z\s+ERROR\s' -or $log -notmatch '\[LAUNCHER TEST\] completed' -or $log -notmatch '\[FG\] slShutdown result=0'){throw 'Incomplete or failed launcher probe'}
foreach($request in @('preset=13 quality=Quality','preset=13 quality=Performance','preset=11 quality=Quality','preset=11 quality=Performance')){if(-not $log.Contains($request)){throw "Missing DLSS context: $request"}}
foreach($stage in @('warmup','flight','crash','left-warmup','left-flight','left-crash','dlss45-performance','rt','settings')) {
    $file=Get-Item "target/visual-launcher-$stage.png"
    if($file.LastWriteTime -lt $started){throw "Stale capture: $stage"}
}
'Launcher probe passed: nine fresh captures and all SR model/quality contexts'
