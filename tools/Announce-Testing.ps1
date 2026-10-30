# Speaks a short notice so you know automated MineHeads test runs are starting
# or finished (their windows are usually hidden). Uses the built-in Windows voice.
#   & .\tools\Announce-Testing.ps1 -Start   -> "Testing MineHeads"
#   & .\tools\Announce-Testing.ps1 -End     -> "Ending testing MineHeads"
param([switch]$Start, [switch]$End, [string]$Text)
$message = if ($Text) { $Text } elseif ($End) { 'Ending testing MineHeads' } else { 'Testing MineHeads' }
try {
    $voice = New-Object -ComObject SAPI.SpVoice
    $voice.Rate = 0
    $voice.Volume = 80
    [void]$voice.Speak($message)
} catch {
    Write-Warning "Could not speak '$message': $($_.Exception.Message)"
}
