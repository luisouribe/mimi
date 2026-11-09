# Unit tests, full build, then every M1 check. Nothing here opens a visible window or a microphone.
$project = "C:\Users\Luiso Uribe\Documents\Claude\DictatorApp"
$env:Path = "$env:USERPROFILE\.cargo\bin;C:\Program Files\CMake\bin;C:\Program Files\LLVM\bin;" + $env:Path
$log = "$env:TEMP\mimi-m1-build.log"

function Show-Problems {
    Select-String -Path $log -Pattern '^error|error\[|error TS|FAILED|panicked|test result|^warning: unused|Built application' |
        Select-Object -First 30 |
        ForEach-Object { $_.Line.Trim().Substring(0, [Math]::Min(170, $_.Line.Trim().Length)) }
    Select-String -Path $log -Pattern '^error' -Context 0, 14 | Select-Object -First 3 | ForEach-Object {
        $_.Context.PostContext | ForEach-Object { $_.Substring(0, [Math]::Min(150, $_.Length)) }
    }
}

Set-Location "$project\mimi\src-tauri"
cargo test --lib *> $log
"unit tests exit $LASTEXITCODE"
Show-Problems
if ($LASTEXITCODE -ne 0) { exit 1 }

Set-Location "$project\mimi"
npm run tauri build -- --no-bundle *> $log
"app build exit $LASTEXITCODE"
Show-Problems
if ($LASTEXITCODE -ne 0) { exit 1 }

Set-Location "$project\mimi\src-tauri"
cargo build --release --features test-tools --bin pipeline_check *> $log
"test tool build exit $LASTEXITCODE"
Show-Problems
if ($LASTEXITCODE -ne 0) { exit 1 }

Get-ChildItem C:\mimi-build\app\release\*.exe, C:\mimi-build\app\release\*.dll |
    Select-Object Name, @{ n = 'MB'; e = { [math]::Round($_.Length / 1MB, 1) } } | Format-Table -AutoSize

Set-Location $project
powershell -ExecutionPolicy Bypass -NoProfile -File "$project\tools\benchmark\run_m1.ps1"
