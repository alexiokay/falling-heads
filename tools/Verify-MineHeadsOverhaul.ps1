$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
Set-Location $root
if((Get-Content target/overhaul-tests.log -Raw) -notmatch 'test result: ok\.') {throw 'Unit tests must pass before rendered verification'}
& "$PSScriptRoot/Test-MineHeadsOverhaul.ps1"
& "$PSScriptRoot/Test-MineHeadsOverhaul.ps1" -SkipBuild -Scene launcher
& "$PSScriptRoot/Test-MineHeadsRtSequence.ps1"
'Final verification passed'
