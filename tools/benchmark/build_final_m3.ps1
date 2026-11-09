# Rebuilds the app with the latest screens, then runs a short hidden check with a temporary data folder.
$project = "C:\Users\Luiso Uribe\Documents\Claude\DictatorApp"
$log = "$env:TEMP\mimi-m3-build.log"
$release = "C:\mimi-build\app\release"
$app = "$release\mimi.exe"
$out = "$project\proof\M3"
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

$env:MIMI_DATA_DIR = "$env:TEMP\mimi-m3-final"
if (Test-Path $env:MIMI_DATA_DIR) { Remove-Item $env:MIMI_DATA_DIR -Recurse -Force }
$env:MIMI_SELF_TEST_OUT = "$out\app-final.json"
$env:MIMI_SELF_TEST_CLIPS = "$project\test-audio\ja\000.wav|3.2|whisper-turbo|ja;$project\test-audio\en\000.wav|5.9|parakeet|"
$process = Start-Process -FilePath $app -PassThru
if (-not $process.WaitForExit(180000)) { "final check TIMED OUT"; Stop-Process -Id $process.Id -Force } else { "final check exit $($process.ExitCode)" }
Get-Item $app | Select-Object LastWriteTime, @{ n = 'MB'; e = { [math]::Round($_.Length / 1MB, 1) } } | Format-List
