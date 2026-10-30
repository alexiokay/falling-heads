param([Parameter(Mandatory=$true)][string]$Destination)
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
$sdk=Join-Path $root 'vendor/Streamline-2.14.16'
$runtime=Join-Path $sdk 'bin/x64'
$destinationPath=[IO.Path]::GetFullPath($Destination)
New-Item -ItemType Directory -Path $destinationPath -Force | Out-Null
# Production files only. Existing SR/RR runtimes are staged by Build-MineHeads.
$names=@('sl.interposer.dll','sl.common.dll','sl.dlss_g.dll','sl.reflex.dll','sl.pcl.dll','nvngx_dlssg.dll','NvLowLatencyVk.dll')
foreach($name in $names) {
    $source=Join-Path $runtime $name
    $target=Join-Path $destinationPath $name
    Copy-Item -LiteralPath $source -Destination $target
    if ((Get-FileHash -LiteralPath $source).Hash -ne (Get-FileHash -LiteralPath $target).Hash) {throw "Streamline copy mismatch: $name"}
}
$licenses=Join-Path $destinationPath 'licenses'
New-Item -ItemType Directory -Path $licenses -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $sdk 'license.txt') -Destination (Join-Path $licenses 'Streamline.txt')
Copy-Item -LiteralPath (Join-Path $sdk '3rd-party-licenses.md') -Destination (Join-Path $licenses 'Streamline-third-party.md')
Copy-Item -LiteralPath (Join-Path $runtime 'nvngx_dlss.license.txt') -Destination (Join-Path $licenses 'NVIDIA-DLSS.txt')
Copy-Item -LiteralPath (Join-Path $runtime 'reflex.license.txt') -Destination (Join-Path $licenses 'NVIDIA-Reflex.txt')
