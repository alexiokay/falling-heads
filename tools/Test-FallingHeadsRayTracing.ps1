param([int]$WaitForBuild=0,[string]$Tag='rt-sequence',[switch]$Negative,[switch]$Calibration)
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
Set-Location $root
if($WaitForBuild){Wait-Process -Id $WaitForBuild -ErrorAction SilentlyContinue}
$build=Start-Process cargo.exe -ArgumentList @('build','--offline') -WindowStyle Hidden -PassThru -RedirectStandardOutput "target/$Tag-build-out.log" -RedirectStandardError "target/$Tag-build-err.log"
$null=$build.Handle
$build.WaitForExit()
if($build.ExitCode){throw 'Ray-tracing probe build failed'}
$env:BEVY_ASSET_ROOT=$root
$exe=Join-Path $root "target/debug/$Tag.exe"
Copy-Item -LiteralPath target/debug/falling-heads.exe -Destination $exe
$probeArgs=@('--visual-smoke-test','--rt-flicker-test')
if($Calibration){$probeArgs+='--rt-bias-check'}else{$probeArgs+='--rt-validation-test'}
if($Negative){$probeArgs+='--rt-negative-test'}
$started=Get-Date
$probe=Start-Process $exe -ArgumentList $probeArgs -WindowStyle Hidden -PassThru -RedirectStandardOutput "target/$Tag-out.log" -RedirectStandardError "target/$Tag-err.log"
$null=$probe.Handle
"Ray-tracing sequence PID $($probe.Id)"
if(-not $probe.WaitForExit(100000)){Stop-Process -Id $probe.Id;throw 'Ray-tracing sequence timeout'}
if($probe.ExitCode){throw "Ray-tracing sequence exit $($probe.ExitCode)"}
$log=Get-Content "target/$Tag-err.log" -Raw
if($log -match 'panicked at|Validation Error|(?m)^\d{4}-[^\r\n]*Z\s+ERROR\s' -or $log -notmatch '\[RT FLICKER\] completed' -or $log -notmatch '\[FG\] slShutdown result=0'){throw 'Incomplete or failed ray-tracing sequence'}
$destination=Join-Path $root "target/$Tag"
New-Item -ItemType Directory -Path $destination -Force | Out-Null
foreach($stage in 0..4){
    $frames=@(Get-ChildItem "target/visual-flicker-$stage-*.png" | Where-Object {$_.LastWriteTime -ge $started})
    if($frames.Count -lt 20){throw "Too few fresh frames in stage $stage"}
    $frames | Copy-Item -Destination $destination
    "Stage $stage : $($frames.Count) fresh frames"
}
"Ray-tracing sequence passed: $destination"
