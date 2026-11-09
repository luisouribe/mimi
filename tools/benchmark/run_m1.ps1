# Runs the M1 checks. No window appears, and no microphone or desktop input is touched.
$project = "C:\Users\Luiso Uribe\Documents\Claude\DictatorApp"
$check = "C:\mimi-build\app\release\pipeline_check.exe"
$app = "C:\mimi-build\app\release\mimi.exe"
$variants = "$project\test-audio-variants"
$out = "$project\proof\M1"
New-Item -ItemType Directory -Force $out | Out-Null

function Run-Case([string]$Name, [string]$Audio, [string]$Langs, [string]$Gain, [string]$Lead) {
    & $check "$Audio" $Langs $Gain $Lead "$out\$Name.json" *> "$out\$Name.log"
    "$Name exit $LASTEXITCODE"
}

Run-Case "mic-48k-stereo" "$variants\48k-stereo" "en,es" "1.0" "0"
Run-Case "mic-44k-mono" "$variants\44k-mono" "en,es" "1.0" "0"
Run-Case "very-quiet-mic" "$variants\48k-stereo" "en,es" "0.05" "0"
Run-Case "silence-around-speech" "$variants\48k-stereo" "en,es" "1.0" "2.0"
Run-Case "long-dictation" "$variants\long" "en" "1.0" "0"
Run-Case "noisy-room" "$variants\noisy-room" "en,es" "1.0" "0"

python -X utf8 -I "$project\tools\benchmark\score.py" "$out\mic-48k-stereo.json" "$out\mic-44k-mono.json" "$out\very-quiet-mic.json" "$out\silence-around-speech.json" "$out\long-dictation.json" "$out\noisy-room.json" | Out-File "$out\results.md" -Encoding utf8
Get-Content "$out\results.md"

# The real app, hidden, driven through its normal screen-to-engine connection using WAV files.
$timings = "$env:LOCALAPPDATA\Mimi\timings.jsonl"
if (Test-Path $timings) { Remove-Item $timings }
$env:MIMI_SELF_TEST_OUT = "$out\app-self-test.json"
$env:MIMI_SELF_TEST_CLIPS = @(
    "$project\test-audio\en\000.wav|5.9",
    "$variants\48k-stereo\es\001.wav|8",
    "$variants\special\silence.wav|3",
    "$variants\special\room-noise.wav|3",
    "$variants\special\loud-noise.wav|3",
    "$variants\special\hum.wav|3"
) -join ";"
if (Test-Path $env:MIMI_SELF_TEST_OUT) { Remove-Item $env:MIMI_SELF_TEST_OUT }
$process = Start-Process -FilePath $app -PassThru
if (-not $process.WaitForExit(120000)) {
    "self test timed out"
    Stop-Process -Id $process.Id -Force
}
python -X utf8 -I "$project\tools\benchmark\compare_m1.py" "$out" "$project\proof\M0\parakeet-v3.json" | Tee-Object "$out\endings.txt"
"app self test exit $($process.ExitCode), window handle $($process.MainWindowHandle)"
if (Test-Path $timings) { Copy-Item $timings "$out\app-timings.jsonl"; Remove-Item $timings }
