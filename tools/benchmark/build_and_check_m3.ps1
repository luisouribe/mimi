# Unit tests, full build, then speech checks on every model. No window, microphone,
# clipboard or key press is involved. Mimi can stay open while this runs.
$project = "C:\Users\Luiso Uribe\Documents\Claude\DictatorApp"
$log = "$env:TEMP\mimi-m3-build.log"
$out = "$project\proof\M3"
$check = "C:\mimi-build\app\release\pipeline_check.exe"
$audio = "$project\test-audio"
New-Item -ItemType Directory -Force $out | Out-Null

function Show-Problems {
    Select-String -Path $log -Pattern '^error|error\[|error TS|FAILED|panicked|test result|^warning: unused|Built application|Finished' |
        Select-Object -First 30 |
        ForEach-Object { $_.Line.Trim().Substring(0, [Math]::Min(170, $_.Line.Trim().Length)) }
    Select-String -Path $log -Pattern '^error' -Context 0, 14 | Select-Object -First 6 | ForEach-Object {
        $_.Line.Substring(0, [Math]::Min(170, $_.Line.Length))
        $_.Context.PostContext | ForEach-Object { $_.Substring(0, [Math]::Min(150, $_.Length)) }
    }
}

$release = "C:\mimi-build\app\release"
if (@(Get-Process mimi -ErrorAction SilentlyContinue).Count -gt 0 -and (Test-Path "$release\mimi.exe")) {
    # Windows can't overwrite a running program but can rename it, and the open copy keeps working.
    $exe = Get-Item "$release\mimi.exe"
    $stamp = Get-Date -Format "yyyyMMddHHmmss"
    Get-ChildItem "$release\deps\mimi-*.exe" | Where-Object { $_.Length -eq $exe.Length -and $_.LastWriteTime -eq $exe.LastWriteTime } |
        ForEach-Object { Rename-Item $_.FullName "$($_.BaseName).$stamp.old" }
    Rename-Item $exe.FullName "mimi.$stamp.old"
    "Mimi is open, so its build copy was renamed. The open window is unaffected."
}

cmd /c "`"$project\tools\benchmark\test_all.cmd`"" *> $log
$code = $LASTEXITCODE
"unit tests exit $code"
Show-Problems
if ($code -ne 0) { exit 1 }

cmd /c "`"$project\tools\benchmark\build_all.cmd`"" *> $log
$code = $LASTEXITCODE
"build exit $code"
Show-Problems
if ($code -ne 0) { exit 1 }

Get-ChildItem "$release\*.exe", "$release\*.dll" |
    Select-Object Name, @{ n = 'MB'; e = { [math]::Round($_.Length / 1MB, 1) } } | Format-Table -AutoSize

function Run-Case([string]$Name, [string]$Langs, [string]$Model, [string]$Language, [string]$Hint, [string]$Device) {
    & $check "$audio" $Langs "1.0" "0" "$out\$Name.json" $Model $Language $Hint $Device *> "$out\$Name.log"
    "$Name exit $LASTEXITCODE"
}

Run-Case "parakeet" "en,es" "parakeet" "clip" "hint" "gpu"
Run-Case "turbo-no-hint" "en,es,ja" "whisper-turbo" "clip" "no-hint" "gpu"
Run-Case "turbo-hint" "en,es,ja" "whisper-turbo" "clip" "hint" "gpu"
Run-Case "turbo-auto-language" "en,es,ja" "whisper-turbo" "auto" "no-hint" "gpu"
Run-Case "small-hint" "en,es,ja" "whisper-small" "clip" "hint" "gpu"
Run-Case "turbo-processor-only" "ja" "whisper-turbo" "clip" "hint" "cpu"

$names = "parakeet", "turbo-no-hint", "turbo-hint", "turbo-auto-language", "small-hint", "turbo-processor-only"
python -X utf8 -I "$project\tools\benchmark\score.py" ($names | ForEach-Object { "$out\$_.json" }) | Out-File "$out\results.md" -Encoding utf8
Get-Content "$out\results.md"
python -X utf8 -I "$project\tools\benchmark\compare_m3.py" "$out" | Tee-Object "$out\punctuation.txt"
