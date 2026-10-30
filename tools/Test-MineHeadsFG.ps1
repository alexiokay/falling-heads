param([int]$WaitForBuild=0, [string]$RunTag="fg-run5", [switch]$Visible, [switch]$BackgroundMotion,[switch]$SkipBuild)
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
Set-Location $root
if ($WaitForBuild) { Wait-Process -Id $WaitForBuild -ErrorAction SilentlyContinue }
if(-not $SkipBuild){
$build = Start-Process (Get-Command cargo.exe).Source -ArgumentList @('build','--offline') -WindowStyle Hidden -PassThru -RedirectStandardOutput target/fg-build-out.log -RedirectStandardError target/fg-build6.log
$null = $build.Handle
$build.WaitForExit()
if ($build.ExitCode) { throw 'FG build failed; see target/fg-build6.log' }
}
$env:BEVY_ASSET_ROOT=$root
$probeArgs=@('--fg-smoke-test')
if ($BackgroundMotion) { $probeArgs+='--background-motion-test' }
$probeStyle='Hidden'
if ($Visible) { $probeArgs+='--fg-visible-test'; $probeStyle='Normal' }
$probe=Start-Process -FilePath (Join-Path $root 'target/debug/falling-heads.exe') -ArgumentList $probeArgs -WorkingDirectory $root -WindowStyle $probeStyle -PassThru -RedirectStandardOutput (Join-Path $root ("target/" + $RunTag + "-out.log")) -RedirectStandardError (Join-Path $root ("target/" + $RunTag + "-err.log"))
$null = $probe.Handle
Write-Output "FG probe PID $($probe.Id)"
if ($Visible) {
    # Explicitly authorized visible test. Never focus a different running game.
    $deadline = [DateTime]::UtcNow.AddSeconds(20)
    do { $probe.Refresh(); if ($probe.MainWindowHandle -ne 0) { break }; Start-Sleep -Milliseconds 200 } while ([DateTime]::UtcNow -lt $deadline -and -not $probe.HasExited)
    $games = @(Get-Process -Name 'falling-heads' -ErrorAction SilentlyContinue)
    if ($games.Count -eq 1 -and $games[0].Id -eq $probe.Id -and $probe.MainWindowHandle -ne 0) {
        & um win drive --proc falling-heads 'focus' 'untop' 'fg'
    } else { Write-Warning 'Skipped focus: could not uniquely identify the test window.' }
}

if (-not $probe.WaitForExit(100000)) {
    Stop-Process -Id $probe.Id
    throw 'FG probe exceeded 100 seconds; stopped only the test process.'
}
Write-Output "FG probe exit $($probe.ExitCode)"
if ($probe.ExitCode) { throw "FG runtime test failed; inspect target/$RunTag-err.log" }

$probeLog = Get-Content -LiteralPath (Join-Path $root ("target/" + $RunTag + "-err.log")) -Raw
if ($probeLog -notmatch '\[FG SMOKE\] transition 6' -or $probeLog -notmatch '\[FG\] slShutdown result=0' -or $probeLog -match 'Exception detected|panicked at|\[Streamline 2\]') {
    throw "FG runtime or shutdown error; inspect target/$RunTag-err.log"
}

if ($BackgroundMotion -and $probeLog -notmatch '\[BACKGROUND MOTION\] completed') {
    throw 'Moving background stability check did not complete.'
}
