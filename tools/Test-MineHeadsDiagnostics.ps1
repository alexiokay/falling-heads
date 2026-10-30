param([string]$Executable)
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
if(-not $Executable){$Executable=Join-Path $root 'target/debug/falling-heads.exe'}
$Executable=(Resolve-Path -LiteralPath $Executable).Path
. (Join-Path $PSScriptRoot 'Run-MineHeadsSession.ps1')
$testRoot=Join-Path $root ('target/diagnostics-verification-'+(Get-Date -Format 'yyyyMMdd-HHmmss-fff'))
foreach($mode in @('clean','panic')) {
    $directory=Join-Path $testRoot $mode
    Invoke-MineHeadsSession -Executable $Executable -GameRoot $root -BuildVersion 'diagnostics-self-test' -BuildHash (Get-FileHash -LiteralPath $Executable).Hash -GameArguments @("--diagnostics-test-$mode") -SessionDirectory $directory
    $report=Get-Content -LiteralPath (Join-Path $directory 'launcher.json') -Raw | ConvertFrom-Json
    $log=Get-Content -LiteralPath (Join-Path $directory 'runtime.log') -Raw
    if($log -notmatch 'Diagnostics self-test log entry'){throw 'File tracing is missing'}
    if($mode -eq 'clean'){
        if($report.status -ne 'clean_exit' -or $report.exitCode -ne 0){throw 'Clean exit was misreported'}
        if(-not (Test-Path -LiteralPath (Join-Path $directory 'app-exit.json'))){throw 'Missing application exit record'}
        if(Test-Path -LiteralPath (Join-Path $directory 'panic.txt')){throw 'Clean exit recorded a false panic'}
    } else {
        if($report.status -ne 'abnormal_exit' -or $report.exitCode -eq 0){throw 'Panic exit was not captured'}
        $panic=Get-Content -LiteralPath (Join-Path $directory 'panic.txt') -Raw
        if($panic -notmatch 'Intentional diagnostics self-test panic' -or $panic -notmatch 'falling_heads::main|falling_heads::diagnostics'){throw 'Missing panic/backtrace'}
        if((Get-Content -LiteralPath (Join-Path $directory 'stderr.log') -Raw) -notmatch 'Intentional diagnostics self-test panic'){throw 'stderr is not captured'}
    }
    "$mode passed: exit=$($report.exitCode), logs=$directory"
}
