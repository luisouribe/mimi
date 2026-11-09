# Simulates weaker PCs by limiting the hidden test tool to fewer processor threads, with no graphics card.
# It opens no window and no microphone. Each run stops at its time limit, and finished clips still count.
$project = "C:\Users\Luiso Uribe\Documents\Claude\DictatorApp"
$out = "$project\proof\M8"
$check = "C:\mimi-build\app\release\pipeline_check.exe"
$audio = "$project\test-audio"
$cases = @(
    @("parakeet-12-threads", "parakeet", "en,es", "cpu", 4095, 150),
    @("parakeet-4-threads", "parakeet", "en,es", "cpu", 15, 150),
    @("parakeet-2-threads", "parakeet", "en,es", "cpu", 3, 150),
    @("small-4-threads", "whisper-small", "es", "cpu", 15, 150),
    @("small-2-threads", "whisper-small", "es", "cpu", 3, 150)
)
foreach ($case in $cases) {
    $name, $model, $languages, $device, $mask, $limit = $case
    $arguments = @("`"$audio`"", $languages, "1.0", "0", "`"$out\slow-$name.json`"", $model, "clip", "hint", $device)
    $process = Start-Process -FilePath $check -ArgumentList $arguments -NoNewWindow -PassThru -RedirectStandardOutput "$out\slow-$name.log" -RedirectStandardError "$out\slow-$name.err.log"
    $process.ProcessorAffinity = [IntPtr][int]$mask
    if (-not $process.WaitForExit($limit * 1000)) { Stop-Process -Id $process.Id -Force; "$name stopped at $limit s" } else { "$name finished" }
}
