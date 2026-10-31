$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
Set-Location $root
if((Get-Content target/overhaul-tests.log -Raw) -notmatch 'test result: ok\.') {throw 'Unit tests must pass before rendered verification'}
& "$PSScriptRoot/Test-FallingHeadsOverhaul.ps1"
& "$PSScriptRoot/Test-FallingHeadsOverhaul.ps1" -SkipBuild -Scene launcher
& "$PSScriptRoot/Test-FallingHeadsRtSequence.ps1"
'Final verification passed'
