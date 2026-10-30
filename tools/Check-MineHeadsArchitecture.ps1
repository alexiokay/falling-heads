param([switch]$UpdateBaseline,[switch]$SkipTests,[switch]$Build,[switch]$Native)
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
Set-Location $root
$expected=(Get-Content tools/build-environment.json -Raw | ConvertFrom-Json).rustc
$compiler=(& rustc --version)
if($compiler -notmatch ('^rustc '+[regex]::Escape($expected)+' ')){throw "Expected rustc $expected; found $compiler. Review the compiler/patch upgrade before updating build-environment.json."}
$formatFiles=Get-Content tools/architecture-format-files.txt
& rustfmt --check --edition 2024 --config skip_children=true $formatFiles
if($LASTEXITCODE){throw 'Architecture formatting failed'}
& cargo clippy --locked --offline -j 1 --bin falling-heads --tests --message-format=json > target/architecture-lint.jsonl 2> target/architecture-lint-stderr.log
if($LASTEXITCODE){Get-Content target/architecture-lint-stderr.log;throw 'Clippy failed; see target/architecture-lint.jsonl'}
$warnings=New-Object 'System.Collections.Generic.HashSet[string]'
foreach($line in Get-Content target/architecture-lint.jsonl){
 try{$item=$line | ConvertFrom-Json}catch{continue}
 if($item.reason -ne 'compiler-message' -or $item.target.name -ne 'falling-heads' -or $item.message.level -ne 'warning'){continue}
 $primary=$item.message.spans | Where-Object is_primary | Select-Object -First 1
 $key=$item.message.code.code+' | '+$primary.file_name.Replace('\','/')+' | '+$item.message.message
 $null=$warnings.Add($key)
}
$current=@($warnings | Sort-Object)
$baselinePath=Join-Path $root 'tools/architecture-warning-baseline.json'
if($UpdateBaseline){ConvertTo-Json -InputObject $current -Depth 3 | Set-Content -LiteralPath $baselinePath -Encoding UTF8}
if(-not(Test-Path $baselinePath)){throw 'Warning baseline missing'}
$baseline=@(Get-Content $baselinePath -Raw | ConvertFrom-Json)
$new=@($current | Where-Object {$_ -notin $baseline})
if($new.Count){$new | Write-Output;throw "$($new.Count) new lint diagnostics; fix or explicitly review the baseline"}
"Lint baseline passed ($($current.Count) known diagnostics, no new warnings)"
if(-not $SkipTests){
 & cargo test --locked --offline -j 1 --bin falling-heads > target/architecture-gate-tests.log 2>&1
 if($LASTEXITCODE){Get-Content target/architecture-gate-tests.log -Tail 40;throw 'Tests failed'}
 Get-Content target/architecture-gate-tests.log -Tail 3
}
if($Build -or $Native){
 & cargo build --locked --offline -j 1 > target/architecture-gate-build.log 2>&1
 if($LASTEXITCODE){throw 'Build failed'}
}
if($Native){
 & ./tools/Test-MineHeadsOverhaul.ps1 -SkipBuild -Scene water
 & ./tools/Test-MineHeadsOverhaul.ps1 -SkipBuild -Scene snowghast
 & ./tools/Test-MineHeadsOverhaul.ps1 -SkipBuild -Scene crowd -FullSize -Dense -Named
}
