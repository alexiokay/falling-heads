param([switch]$SkipBuild,[ValidateSet('overhaul','launcher','refinement','style','polish','crowd','ecology','headsize','water','snowghast','content','rift')][string]$Scene='overhaul',[switch]$RayTracing,[switch]$FullSize,[switch]$Dense,[switch]$Named,[ValidateRange(0,5000)][int]$Count=0)
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
Set-Location $root
if(-not $SkipBuild){
 $build=Start-Process cargo.exe -ArgumentList @('build','--offline') -WindowStyle Hidden -PassThru -RedirectStandardOutput target/overhaul-build-out.log -RedirectStandardError target/overhaul-build-err.log
 $null=$build.Handle
 $build.WaitForExit()
 if($build.ExitCode){throw 'Overhaul build failed'}
}
$env:BEVY_ASSET_ROOT=$root
$exe=Join-Path $root "target/debug/$Scene-verification.exe"
Copy-Item -LiteralPath target/debug/falling-heads.exe -Destination $exe
$started=Get-Date
$probeArgs=@('--visual-smoke-test',"--$Scene-test")
if($RayTracing){$probeArgs+='--rift-rt'}
if($FullSize){$probeArgs+='--crowd-fullsize'}
if($Dense){$probeArgs+='--crowd-dense'}
if($Named){$probeArgs+='--crowd-named'}
if($Count -gt 0){if(-not $Dense){throw 'Explicit crowd count requires -Dense'};$probeArgs+="--crowd-count=$Count"}
$probe=Start-Process $exe -ArgumentList $probeArgs -WindowStyle Hidden -PassThru -RedirectStandardOutput "target/$Scene-run-out.log" -RedirectStandardError "target/$Scene-run-err.log"
$null=$probe.Handle
"$Scene probe PID $($probe.Id)"
if(-not $probe.WaitForExit(150000)){Stop-Process -Id $probe.Id;throw "$Scene probe timeout"}
if($probe.ExitCode){throw "$Scene probe exit $($probe.ExitCode)"}
$log=Get-Content "target/$Scene-run-err.log" -Raw
if($log -match 'panicked at|Validation Error|(?m)^\d{4}-[^\r\n]*Z\s+ERROR\s' -or $log -notmatch "\[$($Scene.ToUpper()) TEST\] completed" -or $log -notmatch '\[FG\] slShutdown result=0'){throw "Incomplete or failed $Scene probe"}
$files=Get-ChildItem "target/visual-$Scene-*.png" | Where-Object LastWriteTime -ge $started
$required=if($Scene -eq 'content' -or $Scene -eq 'rift'){4}elseif($Scene -eq 'snowghast'){27}elseif($Scene -eq 'ecology' -or $Scene -eq 'headsize'){3}elseif($Scene -eq 'crowd'){4}elseif($Scene -eq 'water'){5}elseif($Scene -eq 'refinement'){5}elseif($Scene -eq 'style'){8}elseif($Scene -eq 'polish'){6}else{9}
if($files.Count -lt $required){throw 'Missing fresh captures'}
"$Scene passed: $($files.Count) fresh captures, clean shutdown"
