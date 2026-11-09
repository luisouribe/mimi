# Rebuilds Mimi, runs every unit test, then checks snippets in the real app with no visible window.
# It uses a temporary data folder, so real settings, snippets and dictations aren't touched.
$project = "C:\Users\Luiso Uribe\Documents\Claude\DictatorApp"
$log = "$env:TEMP\mimi-m6-build.log"
$release = "C:\mimi-build\app\release"
$app = "$release\mimi.exe"
$out = "$project\proof\M6"
$audio = "$project\test-audio-m6"
New-Item -ItemType Directory -Force $out | Out-Null
powershell -NoProfile -ExecutionPolicy Bypass -File "$project\tools\m4\make_speech.ps1" -Cases "$project\tools\m6\cases.tsv" -Out $audio
if (@(Get-Process mimi -ErrorAction SilentlyContinue).Count -gt 0 -and (Test-Path $app)) {
    $exe = Get-Item $app
    $stamp = Get-Date -Format "yyyyMMddHHmmss"
    Get-ChildItem "$release\deps\mimi-*.exe" | Where-Object { $_.Length -eq $exe.Length -and $_.LastWriteTime -eq $exe.LastWriteTime } |
        ForEach-Object { Rename-Item $_.FullName "$($_.BaseName).$stamp.old" }
    Rename-Item $exe.FullName "mimi.$stamp.old"
    "Mimi is open, so its build copy was renamed. The open window is unaffected."
}
cmd /c "`"$project\tools\benchmark\build_all.cmd`"" *> $log
"build exit $LASTEXITCODE"
Select-String -Path $log -Pattern '^error|error\[|error TS|Built application' | Select-Object -First 10 |
    ForEach-Object { $_.Line.Trim().Substring(0, [Math]::Min(170, $_.Line.Trim().Length)) }
if (-not (Test-Path $app)) { exit 1 }

cmd /c "call `"$project\tools\benchmark\build_env.cmd`" && cd /d `"$project\mimi\src-tauri`" && cargo test --release --lib && cargo build --release --features test-tools --bin tidy_check" *> "$env:TEMP\mimi-m6-tests.log"
"tests exit $LASTEXITCODE"
Select-String -Path "$env:TEMP\mimi-m6-tests.log" -Pattern 'test result|FAILED|panicked|^error' | ForEach-Object { $_.Line.Trim() }
if ($LASTEXITCODE -ne 0) { Get-Content "$env:TEMP\mimi-m6-tests.log" -Tail 25 }

# Accents are written as codes, because Windows PowerShell can misread them in a script file.
$clips = @("snippet|mi direcci$([char]0xF3)n|Calle Luna 5,\nCiudad de M$([char]0xE9)xico", "snippet|mi enlace|https://example.com/mimi", "snippet|my email|luis@example.com")
foreach ($model in @(@("parakeet", $false), @("whisper-turbo", $true))) {
    $first = $true
    foreach ($id in "es\a1", "es\a2", "es\a3", "en\a4", "en\a5", "en\a6", "es\b1", "es\b2", "en\b3", "en\b4", "es\n1", "es\n2", "es\n3", "en\n4", "en\n5", "en\n6") {
        $language = if ($model[1]) { $id.Substring(0, 2) } else { "" }
        $switch = if ($model[1] -or $first) { "|$($model[0])|$language" } else { "" }
        $seconds = [math]::Ceiling(((Get-Item "$audio\$id.wav").Length - 44) / 32000.0)
        $clips += "$audio\$id.wav|$seconds$switch"
        $first = $false
    }
}
$env:MIMI_DATA_DIR = "$env:TEMP\mimi-m6-final"
if (Test-Path $env:MIMI_DATA_DIR) { Remove-Item $env:MIMI_DATA_DIR -Recurse -Force }
$env:MIMI_SELF_TEST_OUT = "$out\app-final.json"
if (Test-Path $env:MIMI_SELF_TEST_OUT) { Remove-Item $env:MIMI_SELF_TEST_OUT }
$env:MIMI_SELF_TEST_CLIPS = $clips -join ";"
$process = Start-Process -FilePath $app -PassThru
if (-not $process.WaitForExit(420000)) { "hidden check TIMED OUT"; Stop-Process -Id $process.Id -Force } else { "hidden check exit $($process.ExitCode)" }
Remove-Item Env:\MIMI_DATA_DIR, Env:\MIMI_SELF_TEST_OUT, Env:\MIMI_SELF_TEST_CLIPS
Get-Item $app | Select-Object LastWriteTime, @{ n = 'MB'; e = { [math]::Round($_.Length / 1MB, 1) } } | Format-List
