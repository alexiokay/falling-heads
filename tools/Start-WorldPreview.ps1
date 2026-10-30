param([switch]$Open)
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
if(-not (Test-Path (Join-Path $root 'target/debug/falling-heads.exe'))){throw 'Build the game first: cargo build --offline'}
$url='http://127.0.0.1:48119'
try {$null=Invoke-RestMethod "$url/api/manifest" -TimeoutSec 2; $running=$true} catch {$running=$false}
if(-not $running){
 $python=Join-Path $env:USERPROFILE '.cache/codex-runtimes/codex-primary-runtime/dependencies/python/python.exe'
 if(-not(Test-Path $python)){$python=(Get-Command python.exe -ErrorAction Stop).Source}
 $server=Join-Path $PSScriptRoot 'world-preview/server.py'
 $process=Start-Process -FilePath $python -ArgumentList @('"'+$server+'"') -WorkingDirectory $root -WindowStyle Hidden -PassThru -RedirectStandardOutput (Join-Path $root 'target/world-preview-server.log') -RedirectStandardError (Join-Path $root 'target/world-preview-server-error.log')
 $process.Id | Set-Content (Join-Path $root 'target/world-preview-server.pid')
}
if($Open){Start-Process $url}
Write-Output $url
