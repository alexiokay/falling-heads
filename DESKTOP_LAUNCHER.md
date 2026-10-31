# Falling Heads desktop builds

Use **Falling Heads - Play** on the desktop. It runs `tools/Launch-FallingHeads.ps1`, reads `builds/active.json`, verifies the executable hash and launches that release with its matching asset snapshot.

To publish a verified native build:

```powershell
& .\tools\Publish-FallingHeads.ps1 -Executable .\target\debug\falling-heads-weather-release.exe
```

Publishing creates a new version folder and atomically switches the active manifest. It does not modify a running executable or its assets, and does not start another game. A current session keeps its old version until it is closed. The next shortcut launch picks up the update automatically. Running another instance is blocked by the launcher while a normal Falling Heads session from this repository is open.

Read-only launcher check:

```powershell
& .\tools\Launch-FallingHeads.ps1 -ResolveOnly
```

Old releases are retained for recovery; no automatic deletion of running releases occurs. Disk usage grows with publications (binary plus approximately 75 MB of assets each). The launcher does not add world-save persistence to the game. Game settings currently initialise from code; persistence remains separate work.
