# Memory regression check: the game plays itself for -Minutes (as on an unattended
# stream) and fails if private bytes, image assets or glyph atlases keep growing after
# a one-minute warm-up. Music is off (probe runs are quiet). Announces start and end.
#   pwsh -File tools/Test-FallingHeadsMemory.ps1                       # target/debug build, 5 min
#   pwsh -File tools/Test-FallingHeadsMemory.ps1 -Executable target/optimized-candidate/falling-heads.exe -Pack free
param([string]$Executable, [double]$Minutes = 5, [int]$LimitMiB = 1024, [string]$Pack, [switch]$Quiet)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not $Executable) { $Executable = Join-Path $root 'target/debug/falling-heads.exe' }
$Executable = (Resolve-Path -LiteralPath $Executable).Path
if (-not $Quiet) { & (Join-Path $PSScriptRoot 'Announce-Testing.ps1') -Start }
$log = Join-Path $root ('target/memory-probe-' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '.log')
$psi = [Diagnostics.ProcessStartInfo]::new($Executable)
$psi.ArgumentList.Add('--memory-probe')
$psi.WorkingDirectory = $root
$psi.UseShellExecute = $false
$psi.RedirectStandardError = $true
$psi.RedirectStandardOutput = $true
$psi.Environment['BEVY_ASSET_ROOT'] = $root
$psi.Environment['FALLING_HEADS_QUIET'] = '1'
$psi.Environment['FALLING_HEADS_MEMORY_MINUTES'] = "$Minutes"
$psi.Environment['FALLING_HEADS_MEMORY_LIMIT_MIB'] = "$LimitMiB"
if ($Pack) { $psi.Environment['FALLING_HEADS_PACK'] = $Pack }
try {
    $process = [Diagnostics.Process]::Start($psi)
    $stdout = $process.StandardOutput.ReadToEndAsync()
    $lines = [Collections.Generic.List[string]]::new()
    while ($null -ne ($line = $process.StandardError.ReadLine())) {
        $lines.Add($line)
        if ($line -like '`[MEMORY`]*') { $line }
    }
    # The probe exits by itself; allow two extra minutes before treating it as hung.
    if (-not $process.WaitForExit([int](($Minutes + 2) * 60000))) { $process.Kill(); throw 'Memory probe hung' }
    $null = $stdout.Result
    $lines | Set-Content -LiteralPath $log
    $result = $lines | Where-Object { $_ -match '^\[MEMORY\] (PASS|FAIL)' }
    if ($process.ExitCode -ne 0 -or -not ($result -match 'PASS')) {
        throw "Memory probe failed (exit $($process.ExitCode)); log: $log"
    }
    "Memory probe passed; log: $log"
} finally {
    if (-not $Quiet) { & (Join-Path $PSScriptRoot 'Announce-Testing.ps1') -End }
}
