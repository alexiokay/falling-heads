$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
Set-Location $root
$env:BEVY_ASSET_ROOT=$root
$exe=Join-Path $root 'target/debug/rt-sequence-verification.exe'
Copy-Item -LiteralPath target/debug/falling-heads.exe -Destination $exe
$started=Get-Date
$probe=Start-Process $exe -ArgumentList @('--visual-smoke-test','--rt-flicker-test') -WindowStyle Hidden -PassThru -RedirectStandardOutput target/rt-sequence-out.log -RedirectStandardError target/rt-sequence-err.log
$null=$probe.Handle
"RT sequence probe PID $($probe.Id)"
if(-not $probe.WaitForExit(95000)){Stop-Process -Id $probe.Id;throw 'RT sequence timed out'}
if($probe.ExitCode){throw "RT sequence exit $($probe.ExitCode)"}
$log=Get-Content target/rt-sequence-err.log -Raw
if($log -match 'panicked at|Validation Error|(?m)^\d{4}-[^\r\n]*Z\s+ERROR\s' -or $log -notmatch '\[RT FLICKER\] completed' -or $log -notmatch '\[FG\] slShutdown result=0'){throw 'RT sequence did not finish cleanly'}
foreach($stage in 0..4){
 $frames=Get-ChildItem "target/visual-flicker-$stage-*.png" | Where-Object LastWriteTime -ge $started
 if($frames.Count -lt 5){throw "Insufficient frames in stage $stage"}
 "RT stage $stage : $($frames.Count) fresh frames"
}
