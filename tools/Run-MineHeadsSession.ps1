# Called only after launcher integrity checks; also used by diagnostics self-tests.
function Invoke-MineHeadsSession {
    param([string]$Executable,[string]$GameRoot,[string]$BuildVersion,[string]$BuildHash,
        [string[]]$GameArguments=@(),[string]$SessionDirectory)
    if (-not $SessionDirectory) {
        $SessionDirectory=Join-Path $GameRoot ('logs/session-'+(Get-Date -Format 'yyyyMMdd-HHmmss-fff')+'-'+[Guid]::NewGuid().ToString('N').Substring(0,8))
    }
    $SessionDirectory=[IO.Path]::GetFullPath($SessionDirectory)
    New-Item -ItemType Directory -Path $SessionDirectory -Force | Out-Null
    $report=[ordered]@{build=$BuildVersion;sha256=$BuildHash;executable=$Executable;startedUtc=[DateTime]::UtcNow.ToString('o');status='starting'}
    $reportPath=Join-Path $SessionDirectory 'launcher.json'
    $report | ConvertTo-Json | Set-Content -LiteralPath $reportPath -Encoding UTF8
    $oldSession=$env:MINEHEADS_SESSION_DIR
    $oldBacktrace=$env:RUST_BACKTRACE
    try {
        $env:MINEHEADS_SESSION_DIR=$SessionDirectory
        $env:RUST_BACKTRACE='full'
        $start=@{FilePath=$Executable;WorkingDirectory=$GameRoot;WindowStyle='Hidden';PassThru=$true;
            RedirectStandardOutput=(Join-Path $SessionDirectory 'stdout.log');RedirectStandardError=(Join-Path $SessionDirectory 'stderr.log')}
        if ($GameArguments.Count) {$start.ArgumentList=$GameArguments}
        $game=Start-Process @start
        $null=$game.Handle
        $report.pid=$game.Id
        $report.status='running'
        $report | ConvertTo-Json | Set-Content -LiteralPath $reportPath -Encoding UTF8
        # The invisible launcher remains as a tiny monitor. It never restarts or
        # terminates the game; even a native abort still leaves an exit record.
        do {
            # Sample memory headroom without changing the user's page file or
            # touching other processes. This survives a game allocation abort.
            try {
                $game.Refresh()
                $memory=Get-CimInstance Win32_OperatingSystem -ErrorAction Stop
                [ordered]@{utc=[DateTime]::UtcNow.ToString('o');privateBytes=$game.PrivateMemorySize64;
                    workingSetBytes=$game.WorkingSet64;freePhysicalKiB=$memory.FreePhysicalMemory;
                    freeVirtualKiB=$memory.FreeVirtualMemory;totalVirtualKiB=$memory.TotalVirtualMemorySize} |
                    ConvertTo-Json -Compress | Add-Content -LiteralPath (Join-Path $SessionDirectory 'memory.jsonl') -Encoding UTF8
            } catch { } # Diagnostics must never interrupt the running game.
        } while (-not $game.WaitForExit(10000))
        $game.WaitForExit() # Finish draining redirected stdout/stderr.
        $report.exitCode=$game.ExitCode
        $report.exitHex='0x'+[Convert]::ToString([int]$game.ExitCode,16).PadLeft(8,'0')
        $report.endedUtc=[DateTime]::UtcNow.ToString('o')
        $report.status=if($game.ExitCode -eq 0){'clean_exit'}else{'abnormal_exit'}
        $report | ConvertTo-Json | Set-Content -LiteralPath $reportPath -Encoding UTF8
        if($game.ExitCode -ne 0){
            Get-WinEvent -FilterHashtable @{LogName='Application';Id=1000,1001;StartTime=(Get-Date).AddMinutes(-10)} -ErrorAction SilentlyContinue |
                Where-Object {$_.Message -match [regex]::Escape($Executable)} |
                Select-Object -First 5 TimeCreated,Id,Message | Format-List | Out-File -LiteralPath (Join-Path $SessionDirectory 'windows-events.txt') -Encoding UTF8
        }
    } catch {
        $report.status='launcher_error';$report.error=$_.Exception.Message
        $report | ConvertTo-Json | Set-Content -LiteralPath $reportPath -Encoding UTF8
        throw
    } finally {
        $env:MINEHEADS_SESSION_DIR=$oldSession
        $env:RUST_BACKTRACE=$oldBacktrace
    }
}
