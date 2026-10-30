param([Parameter(Mandatory=$true)][string]$Executable)
$ErrorActionPreference = 'Stop'
$gameRoot = Split-Path -Parent $PSScriptRoot
$sourceExe = (Resolve-Path -LiteralPath $Executable).Path
$hash = (Get-FileHash -LiteralPath $sourceExe -Algorithm SHA256).Hash
$version = (Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + $hash.Substring(0,10).ToLowerInvariant()
$builds = Join-Path $gameRoot 'builds'
$release = Join-Path $builds $version
if (Test-Path -LiteralPath $release) { throw 'Release already exists; publication never overwrites an installed release.' }
New-Item -ItemType Directory -Path $release -Force | Out-Null
Copy-Item -LiteralPath $sourceExe -Destination (Join-Path $release 'MineHeads.exe')
# Keep matching symbols beside this immutable binary. Otherwise the next build
# replaces target/debug's PDB and future panic stacks lose their source lines.
$symbols = Join-Path (Split-Path -Parent $sourceExe) 'falling_heads.pdb'
if (Test-Path -LiteralPath $symbols) {
    Copy-Item -LiteralPath $symbols -Destination (Join-Path $release 'falling_heads.pdb')
}
Copy-Item -LiteralPath (Join-Path $gameRoot 'assets') -Destination (Join-Path $release 'assets') -Recurse
if ((Get-FileHash -LiteralPath (Join-Path $release 'MineHeads.exe') -Algorithm SHA256).Hash -ne $hash) { throw 'Copied executable failed integrity verification.' }
$runtimeDlls = [ordered]@{}
foreach ($dll in @('nvngx_dlss.dll','nvngx_dlssd.dll','sl.interposer.dll','sl.common.dll','sl.dlss_g.dll','sl.reflex.dll','sl.pcl.dll','nvngx_dlssg.dll','NvLowLatencyVk.dll')) {
    $sourceDll = Join-Path (Split-Path -Parent $sourceExe) $dll
    $copiedDll = Join-Path $release $dll
    Copy-Item -LiteralPath $sourceDll -Destination $copiedDll
    $dllHash = (Get-FileHash -LiteralPath $sourceDll -Algorithm SHA256).Hash
    if ((Get-FileHash -LiteralPath $copiedDll -Algorithm SHA256).Hash -ne $dllHash) { throw "Runtime DLL copy mismatch: $dll" }
    $runtimeDlls[$dll] = $dllHash
}
Copy-Item -LiteralPath (Join-Path (Split-Path -Parent $sourceExe) 'licenses') -Destination (Join-Path $release 'licenses') -Recurse
foreach ($license in @('MIT','APACHE')) {
    Copy-Item -LiteralPath (Join-Path $gameRoot "vendor/bevy_solari/LICENSE-$license") -Destination (Join-Path $release "licenses/Bevy-Solari-$license.txt")
    Copy-Item -LiteralPath (Join-Path $gameRoot "vendor/bevy_anti_alias/LICENSE-$license") -Destination (Join-Path $release "licenses/Bevy-Anti-Alias-$license.txt")
}
$assetSource = Join-Path $gameRoot 'assets'
$assetCount = 0
Get-ChildItem -LiteralPath $assetSource -File -Recurse | ForEach-Object {
    $relative = $_.FullName.Substring($assetSource.Length + 1)
    $copied = Join-Path (Join-Path $release 'assets') $relative
    if ((Get-FileHash -LiteralPath $_.FullName).Hash -ne (Get-FileHash -LiteralPath $copied).Hash) { throw "Asset copy mismatch: $relative" }
    $assetCount++
}
$manifest = [ordered]@{version=$version;sha256=$hash;runtimeDlls=$runtimeDlls;assetCount=$assetCount;publishedUtc=[DateTime]::UtcNow.ToString('o')}
$staged = Join-Path $builds ('active-' + [Guid]::NewGuid().ToString('N') + '.json')
$manifest | ConvertTo-Json | Set-Content -LiteralPath $staged -Encoding UTF8
$active = Join-Path $builds 'active.json'
if (Test-Path -LiteralPath $active) {
    # A real backup path works in both Windows PowerShell and PowerShell 7;
    # some .NET bindings coerce a null path into an invalid empty string.
    $backup = Join-Path $builds ('active-previous-' + [Guid]::NewGuid().ToString('N') + '.json')
    [IO.File]::Replace($staged,$active,$backup)
    Remove-Item -LiteralPath $backup
} else { [IO.File]::Move($staged,$active) }
$desktop = [Environment]::GetFolderPath('Desktop')
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut((Join-Path $desktop 'MineHeads - Play.lnk'))
$shortcut.TargetPath = Join-Path $env:SystemRoot 'System32/WindowsPowerShell/v1.0/powershell.exe'
$shortcut.Arguments = '-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File "' + (Join-Path $PSScriptRoot 'Launch-MineHeads.ps1') + '"'
$shortcut.WorkingDirectory = $gameRoot
# Game icon (tools/Make-Icon.ps1); the assets copy above already contains it.
$icon = Join-Path $release 'assets/icon/mineheads.ico'
$shortcut.IconLocation = if (Test-Path -LiteralPath $icon) { $icon + ',0' } else { (Join-Path $release 'MineHeads.exe') + ',0' }
$shortcut.Description = 'Play the latest verified MineHeads build'
$shortcut.Save()
$manifest | ConvertTo-Json
