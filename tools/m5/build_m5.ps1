# Rebuilds Mimi, runs every unit test, then checks the new screens in the real app with no visible window.
# It uses a temporary data folder, so real settings and dictations aren't touched.
$project = "C:\Users\Luiso Uribe\Documents\Claude\DictatorApp"
$log = "$env:TEMP\mimi-m5-build.log"
$release = "C:\mimi-build\app\release"
$app = "$release\mimi.exe"
$out = "$project\proof\M5"
$audio = "$project\test-audio-m4"
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

cmd /c "call `"$project\tools\benchmark\build_env.cmd`" && cd /d `"$project\mimi\src-tauri`" && cargo test --release --lib" *> "$env:TEMP\mimi-m5-tests.log"
"tests exit $LASTEXITCODE"
Select-String -Path "$env:TEMP\mimi-m5-tests.log" -Pattern 'test result|FAILED|panicked' | ForEach-Object { $_.Line.Trim() }

$env:MIMI_DATA_DIR = "$env:TEMP\mimi-m5-final"
if (Test-Path $env:MIMI_DATA_DIR) { Remove-Item $env:MIMI_DATA_DIR -Recurse -Force }
$env:MIMI_SELF_TEST_OUT = "$out\app-final.json"
if (Test-Path $env:MIMI_SELF_TEST_OUT) { Remove-Item $env:MIMI_SELF_TEST_OUT }
$env:MIMI_SELF_TEST_CLIPS = (@(
    "$audio\en\e01.wav|5|parakeet|",
    "$audio\es\s02.wav|4|whisper-turbo|es"
) -join ";")
$process = Start-Process -FilePath $app -PassThru
if (-not $process.WaitForExit(240000)) { "hidden check TIMED OUT"; Stop-Process -Id $process.Id -Force } else { "hidden check exit $($process.ExitCode)" }
Remove-Item Env:\MIMI_DATA_DIR, Env:\MIMI_SELF_TEST_OUT, Env:\MIMI_SELF_TEST_CLIPS
Get-Item $app | Select-Object LastWriteTime, @{ n = 'MB'; e = { [math]::Round($_.Length / 1MB, 1) } } | Format-List
