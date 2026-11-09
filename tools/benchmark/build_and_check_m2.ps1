# Unit tests, full build, then the hidden app test. Nothing here opens a visible window,
# presses keys, touches the clipboard or uses a microphone.
$project = "C:\Users\Luiso Uribe\Documents\Claude\DictatorApp"
$env:Path = "$env:USERPROFILE\.cargo\bin;C:\Program Files\CMake\bin;C:\Program Files\LLVM\bin;" + $env:Path
$log = "$env:TEMP\mimi-m2-build.log"
$app = "C:\mimi-build\app\release\mimi.exe"
$out = "$project\proof\M2"
$variants = "$project\test-audio-variants"
New-Item -ItemType Directory -Force $out | Out-Null

function Show-Problems {
    Select-String -Path $log -Pattern '^error|error\[|error TS|FAILED|panicked|test result|^warning: unused|Built application' |
        Select-Object -First 30 |
        ForEach-Object { $_.Line.Trim().Substring(0, [Math]::Min(170, $_.Line.Trim().Length)) }
    Select-String -Path $log -Pattern '^error' -Context 0, 14 | Select-Object -First 4 | ForEach-Object {
        $_.Line.Substring(0, [Math]::Min(170, $_.Line.Length))
        $_.Context.PostContext | ForEach-Object { $_.Substring(0, [Math]::Min(150, $_.Length)) }
    }
}

$running = @(Get-Process mimi -ErrorAction SilentlyContinue)
if ($running.Count -gt 0) {
    # Windows can't overwrite a running program but can rename it, and the open copy keeps working.
    # Cargo keeps the same program under a second name in its deps folder.
    $release = "C:\mimi-build\app\release"
    $exe = Get-Item "$release\mimi.exe"
    $stamp = Get-Date -Format "yyyyMMddHHmmss"
    Get-ChildItem "$release\deps\mimi-*.exe" | Where-Object { $_.Length -eq $exe.Length -and $_.LastWriteTime -eq $exe.LastWriteTime } |
        ForEach-Object { Rename-Item $_.FullName "$($_.BaseName).$stamp.old" }
    Rename-Item $exe.FullName "mimi.$stamp.old"
    "Mimi is open, so its build copy was renamed. The open window is unaffected."
}

Set-Location "$project\mimi\src-tauri"
cargo test --lib *> $log
$code = $LASTEXITCODE
"unit tests exit $code"
Show-Problems
if ($code -ne 0) { exit 1 }

Set-Location "$project\mimi"
npm run tauri build -- --no-bundle *> $log
$code = $LASTEXITCODE
"app build exit $code"
Show-Problems
if ($code -ne 0) { exit 1 }

Get-ChildItem C:\mimi-build\app\release\*.exe, C:\mimi-build\app\release\*.dll |
    Select-Object Name, @{ n = 'MB'; e = { [math]::Round($_.Length / 1MB, 1) } } | Format-Table -AutoSize

$env:MIMI_SELF_TEST_OUT = "$out\app-self-test.json"
$env:MIMI_SELF_TEST_CLIPS = @(
    "$project\test-audio\en\000.wav|5.9",
    "$variants\48k-stereo\es\001.wav|8",
    "$variants\special\silence.wav|3"
) -join ";"
if (Test-Path $env:MIMI_SELF_TEST_OUT) { Remove-Item $env:MIMI_SELF_TEST_OUT }
$timings = "$env:LOCALAPPDATA\Mimi\timings.jsonl"
$before = if (Test-Path $timings) { @(Get-Content $timings).Count } else { 0 }
$process = Start-Process -FilePath $app -PassThru
if (-not $process.WaitForExit(120000)) {
    "self test timed out"
    Stop-Process -Id $process.Id -Force
}
"app self test exit $($process.ExitCode)"
if (Test-Path $timings) {
    $lines = @(Get-Content $timings)
    if ($lines.Count -gt $before) { $lines | Select-Object -First $before | Set-Content $timings -Encoding utf8 }
}
"real history database present: " + (Test-Path "$env:LOCALAPPDATA\Mimi\history.db")
