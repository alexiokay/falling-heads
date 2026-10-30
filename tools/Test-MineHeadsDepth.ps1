param()
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
Set-Location $root
$running=Get-Process -Name MineHeads,falling-heads,depth-verification,mechanism-verification -ErrorAction SilentlyContinue
if($running){throw 'Close the running game before the depth benchmark.'}
$settings=Join-Path $root 'graphics-settings.json'
$saved=if(Test-Path -LiteralPath $settings){[IO.File]::ReadAllBytes($settings)}else{$null}
$exe=Join-Path $root 'target/debug/depth-verification.exe'
Copy-Item -LiteralPath 'target/optimized-candidate/falling-heads.exe' -Destination $exe
$env:BEVY_ASSET_ROOT=$root
$probe=$null
$monitor=$null
try {
    & nvidia-smi --query-gpu=name,driver_version,memory.total --format=csv > target/depth-hardware.csv
    Get-FileHash -LiteralPath $exe -Algorithm SHA256 | ConvertTo-Json > target/depth-executable.json
    $monitor=Start-Process -FilePath (Get-Command nvidia-smi).Source -ArgumentList @('--query-gpu=timestamp,utilization.gpu,clocks.current.graphics,temperature.gpu,power.draw','--format=csv','-l','1') -WindowStyle Hidden -PassThru -RedirectStandardOutput 'target/depth-telemetry.csv' -RedirectStandardError 'target/depth-telemetry-error.log'
    $probe=Start-Process -FilePath $exe -ArgumentList @('--visual-smoke-test','--depth-benchmark') -WindowStyle Hidden -PassThru -RedirectStandardOutput 'target/depth-run-out.log' -RedirectStandardError 'target/depth-run-err.log'
    $null=$probe.Handle
    "Depth probe PID $($probe.Id)"
    if(-not $probe.WaitForExit(450000)){throw 'Depth benchmark timeout'}
    if($probe.ExitCode){throw "Depth benchmark exit $($probe.ExitCode)"}
    $log=Get-Content 'target/depth-run-err.log' -Raw
    if($log -notmatch '\[DEPTH\] completed' -or $log -notmatch '\[FG\] slShutdown result=0' -or $log -match 'panicked at|Validation Error'){throw 'Incomplete or failed depth benchmark'}
    'Depth A/B benchmark passed, clean shutdown'
} finally {
    if($probe -and -not $probe.HasExited){Stop-Process -Id $probe.Id}
    if($monitor -and -not $monitor.HasExited){Stop-Process -Id $monitor.Id}
    if($null -ne $saved){[IO.File]::WriteAllBytes($settings,$saved)}
    elseif(Test-Path -LiteralPath $settings){Remove-Item -LiteralPath $settings}
}
