# Rebuilds Mimi, then checks model switching in the real app with no visible window.
# It uses a temporary data folder, so real settings and dictations aren't touched.
$project = "C:\Users\Luiso Uribe\Documents\Claude\DictatorApp"
$log = "$env:TEMP\mimi-m3-build.log"
$out = "$project\proof\M3"
$audio = "$project\test-audio"
$variants = "$project\test-audio-variants"
$release = "C:\mimi-build\app\release"
$app = "$release\mimi.exe"
$check = "$release\pipeline_check.exe"

if (@(Get-Process mimi -ErrorAction SilentlyContinue).Count -gt 0 -and (Test-Path $app)) {
    $exe = Get-Item $app
    $stamp = Get-Date -Format "yyyyMMddHHmmss"
    Get-ChildItem "$release\deps\mimi-*.exe" | Where-Object { $_.Length -eq $exe.Length -and $_.LastWriteTime -eq $exe.LastWriteTime } |
        ForEach-Object { Rename-Item $_.FullName "$($_.BaseName).$stamp.old" }
    Rename-Item $exe.FullName "mimi.$stamp.old"
    "Mimi is open, so its build copy was renamed. The open window is unaffected."
}

cmd /c "`"$project\tools\benchmark\build_all.cmd`" clean-whisper" *> $log
"build exit $LASTEXITCODE"
Select-String -Path $log -Pattern '^error|error\[|error TS|Built application' | Select-Object -First 10 |
    ForEach-Object { $_.Line.Trim().Substring(0, [Math]::Min(170, $_.Line.Trim().Length)) }
if (-not (Test-Path $app)) { "app was not built"; exit 1 }
Get-ChildItem "$release\*.exe", "$release\*.dll" |
    Select-Object Name, @{ n = 'MB'; e = { [math]::Round($_.Length / 1MB, 1) } } | Format-Table -AutoSize

function Run-Hidden([string]$Name, [string]$Clips, [int]$TimeoutSeconds) {
    $data = "$env:TEMP\mimi-m3-$Name"
    return @{ Data = $data; Name = $Name; Clips = $Clips; Timeout = $TimeoutSeconds }
}

function Start-Hidden($Plan, [switch]$KeepData, [switch]$ExistingData) {
    if (-not $ExistingData -and (Test-Path $Plan.Data)) { Remove-Item $Plan.Data -Recurse -Force }
    New-Item -ItemType Directory -Force $Plan.Data | Out-Null
    $env:MIMI_DATA_DIR = $Plan.Data
    $env:MIMI_SELF_TEST_OUT = "$out\app-$($Plan.Name).json"
    $env:MIMI_SELF_TEST_CLIPS = $Plan.Clips
    if (Test-Path $env:MIMI_SELF_TEST_OUT) { Remove-Item $env:MIMI_SELF_TEST_OUT }
    $process = Start-Process -FilePath $app -PassThru
    if (-not $process.WaitForExit($Plan.Timeout * 1000)) {
        "$($Plan.Name) TIMED OUT after $($Plan.Timeout) s"
        Stop-Process -Id $process.Id -Force
    } else {
        "$($Plan.Name) exit $($process.ExitCode)"
    }
}

$en = "$audio\en\000.wav"
$es = "$variants\48k-stereo\es\001.wav"
$ja = "$audio\ja\000.wav"
$ja2 = "$audio\ja\001.wav"
$silence = "$variants\special\silence.wav"
$noise = "$variants\special\loud-noise.wav"

$switching = Run-Hidden "switching" (@(
    "$en|5.9",
    "$ja|3.2|whisper-turbo|ja",
    "$ja2|3.5",
    "$silence|3",
    "$noise|3",
    "$es|8|whisper-turbo|es",
    "$ja|3.2|whisper-small|ja",
    "$en|5.9|parakeet|"
) -join ";") 240
Start-Hidden $switching

$remembered = Run-Hidden "remembered" "$en|5.9" 120
if (Test-Path $remembered.Data) { Remove-Item $remembered.Data -Recurse -Force }
New-Item -ItemType Directory -Force $remembered.Data | Out-Null
'{"model":"whisper-turbo","language":"ja"}' | Out-File "$($remembered.Data)\settings.json" -Encoding ascii
$remembered.Clips = "$ja|3.2"
Start-Hidden $remembered -ExistingData

$crash = Run-Hidden "after-graphics-crash" "$ja|3.2" 300
if (Test-Path $crash.Data) { Remove-Item $crash.Data -Recurse -Force }
New-Item -ItemType Directory -Force $crash.Data | Out-Null
'{"model":"whisper-small","language":"ja"}' | Out-File "$($crash.Data)\settings.json" -Encoding ascii
New-Item -ItemType File "$($crash.Data)\gpu-check" | Out-Null
Start-Hidden $crash -ExistingData
"settings after crash test: " + (Get-Content "$($crash.Data)\settings.json" -Raw)

Remove-Item Env:\MIMI_DATA_DIR, Env:\MIMI_SELF_TEST_OUT, Env:\MIMI_SELF_TEST_CLIPS

# Every model again with the final build. Processor-only runs are stopped if they take too long.
foreach ($case in @(
        @("parakeet", "parakeet", "en,es", "gpu"),
        @("turbo-hint", "whisper-turbo", "en,es,ja", "gpu"),
        @("small-hint", "whisper-small", "en,es,ja", "gpu"),
        @("small-processor-only", "whisper-small", "en,es,ja", "cpu"),
        @("turbo-processor-only", "whisper-turbo", "en,es,ja", "cpu"))) {
    $arguments = @("`"$audio`"", $case[2], "1.0", "0", "`"$out\$($case[0]).json`"", $case[1], "clip", "hint", $case[3])
    $process = Start-Process -FilePath $check -ArgumentList $arguments -NoNewWindow -PassThru -RedirectStandardOutput "$out\$($case[0]).log" -RedirectStandardError "$out\$($case[0]).err.log"
    if (-not $process.WaitForExit(420000)) {
        "$($case[0]) TIMED OUT after 7 minutes"
        Stop-Process -Id $process.Id -Force
    } else {
        "$($case[0]) exit $($process.ExitCode)"
    }
}

$names = "parakeet", "turbo-no-hint", "turbo-hint", "turbo-auto-language", "small-hint", "small-processor-only", "turbo-processor-only"
$files = $names | ForEach-Object { "$out\$_.json" } | Where-Object { Test-Path $_ }
python -X utf8 -I "$project\tools\benchmark\score.py" $files | Out-File "$out\results.md" -Encoding utf8
Get-Content "$out\results.md" | Select-String 'processor'

$dumpbin = Get-ChildItem "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\*\bin\Hostx64\x64\dumpbin.exe" | Select-Object -First 1
if ($dumpbin) {
    "--- DLLs Mimi needs when it opens"
    & $dumpbin.FullName /dependents $app | Select-String '\.dll' | ForEach-Object { $_.Line.Trim() } | Where-Object { $_ -match 'DirectML|onnx|vulkan|VCRUNTIME|MSVCP' }
}
