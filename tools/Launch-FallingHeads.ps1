param([switch]$ResolveOnly)
$ErrorActionPreference = 'Stop'
$gameRoot = Split-Path -Parent $PSScriptRoot
try {
    $manifest = Get-Content -LiteralPath (Join-Path $gameRoot 'builds/active.json') -Raw | ConvertFrom-Json
    $buildRoot = [IO.Path]::GetFullPath((Join-Path $gameRoot ('builds/' + $manifest.version)))
    $allowedRoot = [IO.Path]::GetFullPath((Join-Path $gameRoot 'builds')) + [IO.Path]::DirectorySeparatorChar
    if (-not $buildRoot.StartsWith($allowedRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Invalid build path.' }
    $gameExe = Join-Path $buildRoot 'FallingHeads.exe'
    # Releases from before the rename ship MineHeads.exe.
    if (-not (Test-Path -LiteralPath $gameExe)) { $gameExe = Join-Path $buildRoot 'MineHeads.exe' }
    if (-not (Test-Path -LiteralPath $gameExe)) { throw 'The active Falling Heads build is missing.' }
    if ((Get-FileHash -LiteralPath $gameExe -Algorithm SHA256).Hash -ne $manifest.sha256) { throw 'The active build failed its integrity check.' }
    $allowedRuntimes=@('nvngx_dlss.dll','nvngx_dlssd.dll','sl.interposer.dll','sl.common.dll','sl.dlss_g.dll','sl.reflex.dll','sl.pcl.dll','nvngx_dlssg.dll','NvLowLatencyVk.dll')
    if (-not $manifest.runtimeDlls.'nvngx_dlss.dll' -or -not $manifest.runtimeDlls.'nvngx_dlssd.dll') {throw 'Missing core runtime hashes.'}
    foreach ($name in $manifest.runtimeDlls.PSObject.Properties.Name) {
        if ($name -notin $allowedRuntimes) {throw 'Invalid runtime name.'}
        $runtime = Join-Path $buildRoot $name
        if (-not (Test-Path -LiteralPath $runtime) -or
            (Get-FileHash -LiteralPath $runtime -Algorithm SHA256).Hash -ne $manifest.runtimeDlls.$name) {
            throw "The active NVIDIA runtime failed its integrity check: $name"
        }
    }
    if ($ResolveOnly) { $gameExe; exit 0 }
    $alreadyRunning = Get-CimInstance Win32_Process | Where-Object {
        $_.ExecutablePath -and $_.ExecutablePath.StartsWith($gameRoot + '\', [StringComparison]::OrdinalIgnoreCase) -and
        ($_.Name -eq 'FallingHeads.exe' -or $_.Name -eq 'falling-heads.exe')
    }
    if ($alreadyRunning) {
        Add-Type -AssemblyName System.Windows.Forms
        [Windows.Forms.MessageBox]::Show('FallingHeads is already running. Your current session stays open. Quit it and use this shortcut again to play the latest installed version.', 'FallingHeads') | Out-Null
        exit 0
    }
    # Assets are an immutable snapshot belonging to this binary. A later update
    # cannot hot-reload incompatible shaders into an already running session.
    $env:BEVY_ASSET_ROOT = $buildRoot
    . (Join-Path $PSScriptRoot 'Run-FallingHeadsSession.ps1')
    Invoke-FallingHeadsSession -Executable $gameExe -GameRoot $gameRoot -BuildVersion $manifest.version -BuildHash $manifest.sha256
} catch {
    if ($ResolveOnly) { throw }
    Add-Type -AssemblyName System.Windows.Forms
    [Windows.Forms.MessageBox]::Show($_.Exception.Message, 'FallingHeads launcher') | Out-Null
    exit 1
}
