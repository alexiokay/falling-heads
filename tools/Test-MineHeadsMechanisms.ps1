param()
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
Set-Location $root
if(Get-Process -Name MineHeads,falling-heads -ErrorAction SilentlyContinue){throw 'Close the running game before the native probe.'}
$env:BEVY_ASSET_ROOT=$root
$exe=Join-Path $root 'target/debug/mechanism-verification.exe'
Copy-Item -LiteralPath 'target/optimized-candidate/falling-heads.exe' -Destination $exe
$started=Get-Date
$probe=Start-Process -FilePath $exe -ArgumentList @('--visual-smoke-test','--mechanism-test') -WindowStyle Hidden -PassThru -RedirectStandardOutput 'target/mechanism-run-out.log' -RedirectStandardError 'target/mechanism-run-err.log'
$null=$probe.Handle
"Mechanism probe PID $($probe.Id)"
if(-not $probe.WaitForExit(120000)){Stop-Process -Id $probe.Id;throw 'Mechanism probe timeout'}
if($probe.ExitCode){throw "Mechanism probe exit $($probe.ExitCode)"}
$log=Get-Content 'target/mechanism-run-err.log' -Raw
if($log -match 'panicked at|Validation Error|(?m)^\d{4}-[^\r\n]*Z\s+ERROR\s' -or $log -notmatch '\[MECHANISM TEST\] completed' -or $log -notmatch '\[FG\] slShutdown result=0'){throw 'Incomplete or failed mechanism probe'}
$captures=@(Get-ChildItem 'target/visual-mechanism-*.png' | Where-Object LastWriteTime -ge $started)
if($captures.Count -lt 5){throw 'Missing fresh mechanism captures'}
"Mechanisms passed: $($captures.Count) captures, clean shutdown"
