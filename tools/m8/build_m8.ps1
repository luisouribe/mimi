# Builds Mimi's installer and runs every unit test. Installing is a separate step, agreed with the owner.
$project = "C:\Users\Luiso Uribe\Documents\Claude\DictatorApp"
$log = "$env:TEMP\mimi-m8-build.log"
$release = "C:\mimi-build\app\release"
$app = "$release\mimi.exe"
New-Item -ItemType Directory -Force "$project\proof\M8" | Out-Null
powershell -NoProfile -ExecutionPolicy Bypass -File "$project\tools\m8\collect_files.ps1"
if (@(Get-Process mimi -ErrorAction SilentlyContinue).Count -gt 0) {
    $stamp = Get-Date -Format "yyyyMMddHHmmss"
    if (Test-Path $app) {
        $exe = Get-Item $app
        Get-ChildItem "$release\deps\mimi-*.exe" | Where-Object { $_.Length -eq $exe.Length -and $_.LastWriteTime -eq $exe.LastWriteTime } |
            ForEach-Object { Rename-Item $_.FullName "$($_.BaseName).$stamp.old" }
        Rename-Item $exe.FullName "mimi.$stamp.old"
    }
    # An open Mimi also holds its libraries, which the build has to copy again.
    Get-ChildItem -LiteralPath "$project\mimi\src-tauri\bundle-files" -Filter "*.dll" | ForEach-Object {
        $held = Join-Path $release $_.Name
        if (Test-Path -LiteralPath $held) { Rename-Item -LiteralPath $held "$($_.Name).$stamp.old" }
    }
    "Mimi is open, so its build copies were renamed. The open window is unaffected."
}
cmd /c "call `"$project\tools\benchmark\build_env.cmd`" && cd /d `"$project\mimi`" && npm run tauri build" *> $log
"installer build exit $LASTEXITCODE"
Select-String -Path $log -Pattern '^error|error\[|error TS|Built application|Finished .* bundle|setup\.exe|makensis|mmap|Internal compiler' | Select-Object -First 15 |
    ForEach-Object { $_.Line.Trim().Substring(0, [Math]::Min(190, $_.Line.Trim().Length)) }
cmd /c "call `"$project\tools\benchmark\build_env.cmd`" && cd /d `"$project\mimi\src-tauri`" && cargo build --release --features test-tools --bin pipeline_check && cargo test --release --lib" *> "$env:TEMP\mimi-m8-tests.log"
$testsExit = $LASTEXITCODE
"tests exit $testsExit"
Select-String -Path "$env:TEMP\mimi-m8-tests.log" -Pattern 'test result|FAILED|panicked|^error' | ForEach-Object { $_.Line.Trim() }
Get-ChildItem "$release\bundle\nsis" -ErrorAction SilentlyContinue | Select-Object Name, @{ n = 'MB'; e = { [math]::Round($_.Length / 1MB, 1) } } | Format-Table -AutoSize
