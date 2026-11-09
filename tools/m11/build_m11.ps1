# Builds the installer and runs every unit test, then checks dictation still works in the real app with no window.
# The app check uses a temporary data folder, so real settings, snippets and dictations aren't touched.
$project = "C:\Users\Luiso Uribe\Documents\Claude\DictatorApp"
$release = "C:\mimi-build\app\release"
$out = "$project\proof\M11"
$audio = "$project\test-audio-m9"
New-Item -ItemType Directory -Force $out | Out-Null
powershell -NoProfile -ExecutionPolicy Bypass -File "$project\tools\m8\build_m8.ps1"
if (-not (Test-Path "$release\mimi.exe")) { "no app was built"; exit 1 }

$clips = @("snippet|mi enlace|https://example.com/mimi", "$audio\es\f01.wav|6|parakeet|", "$audio\es\f03.wav|6", "$audio\es\f05.wav|5","$audio\es\k01.wav|5", "history-check|libro", "clear-check")
$env:MIMI_DATA_DIR = "$env:TEMP\mimi-m11-final"
if (Test-Path $env:MIMI_DATA_DIR) { Remove-Item $env:MIMI_DATA_DIR -Recurse -Force }
$env:MIMI_SELF_TEST_OUT = "$out\app-final.json"
if (Test-Path $env:MIMI_SELF_TEST_OUT) { Remove-Item $env:MIMI_SELF_TEST_OUT }
$env:MIMI_SELF_TEST_CLIPS = $clips -join ";"
$process = Start-Process -FilePath "$release\mimi.exe" -PassThru
if (-not $process.WaitForExit(120000)) { "hidden check TIMED OUT"; Stop-Process -Id $process.Id -Force } else { "hidden check exit $($process.ExitCode)" }
Remove-Item Env:\MIMI_DATA_DIR, Env:\MIMI_SELF_TEST_OUT, Env:\MIMI_SELF_TEST_CLIPS
# The hidden test must never lower the owner's apps, so it must leave no volume memory behind.
if (Test-Path "$env:TEMP\mimi-m11-final\other-apps-volume.json") { "VOLUME MEMORY WAS WRITTEN" } else { "no volume memory written, as expected" }
