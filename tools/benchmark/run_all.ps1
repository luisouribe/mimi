# Runs every speech model over the test recordings and saves the raw results.
$project = "C:\Users\Luiso Uribe\Documents\Claude\DictatorApp"
$bench = "C:\mimi-build\bench\release\mimi-bench.exe"
$audio = "$project\test-audio"
$models = "$project\models"
$out = "$project\proof\M0"
$log = "$out\benchmark-log.txt"
New-Item -ItemType Directory -Force $out | Out-Null
"started $(Get-Date)" | Out-File $log -Encoding utf8

function Run-Test([string]$Name, [string]$Kind, [string]$Model, [string]$Langs, [int64]$Affinity = 0) {
    "=== $Name" | Out-File $log -Append -Encoding utf8
    $outLog = "$out\$Name.stdout.txt"
    $errLog = "$out\$Name.stderr.txt"
    $arguments = @($Kind, "`"$Model`"", "`"$audio`"", $Langs, "`"$out\$Name.json`"")
    $process = Start-Process -FilePath $bench -ArgumentList $arguments -NoNewWindow -PassThru `
        -RedirectStandardOutput $outLog -RedirectStandardError $errLog
    if ($Affinity -ne 0) { $process.ProcessorAffinity = [IntPtr]$Affinity }
    $process.WaitForExit()
    "$Name exit $($process.ExitCode)" | Out-File $log -Append -Encoding utf8
}

Run-Test "parakeet-v3" "parakeet" "$models\parakeet-tdt-0.6b-v3-int8" "en,es"
# Four logical processors, equal to a basic dual-core laptop.
Run-Test "parakeet-v3-weak-pc" "parakeet" "$models\parakeet-tdt-0.6b-v3-int8" "en,es" 15
Run-Test "whisper-turbo-gpu" "whisper" "$models\whisper\ggml-large-v3-turbo-q5_0.bin" "en,es,ja"
Run-Test "whisper-small-gpu" "whisper" "$models\whisper\ggml-small-q5_1.bin" "en,es,ja"

"finished $(Get-Date)" | Out-File $log -Append -Encoding utf8
Get-Content $log
python -I "$project\tools\benchmark\score.py" "$out\parakeet-v3.json" "$out\parakeet-v3-weak-pc.json" "$out\whisper-turbo-gpu.json" "$out\whisper-small-gpu.json" | Tee-Object "$out\results.md"
