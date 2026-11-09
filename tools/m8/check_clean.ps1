# Runs Mimi hidden from a clean folder, the way the installer lays it out, and records what it uses.
# Nothing is installed. Real settings and dictations aren't touched, and no window appears.
$project = "C:\Users\Luiso Uribe\Documents\Claude\DictatorApp"
$out = "$project\proof\M8"
$release = "C:\mimi-build\app\release"
$clean = "$env:TEMP\mimi-clean-install"
New-Item -ItemType Directory -Force $out | Out-Null
if (Test-Path $clean) { Remove-Item $clean -Recurse -Force }
New-Item -ItemType Directory -Force "$clean\models\whisper", "$clean\models\parakeet-tdt-0.6b-v3-int8" | Out-Null
Copy-Item "$release\mimi.exe" $clean
Get-ChildItem -LiteralPath "$project\mimi\src-tauri\bundle-files" | ForEach-Object { Copy-Item -LiteralPath $_.FullName $clean }
Get-ChildItem "$project\models" -Recurse -File | ForEach-Object {
    $relative = $_.FullName.Substring("$project\models\".Length)
    New-Item -ItemType HardLink -Path "$clean\models\$relative" -Target $_.FullName | Out-Null
}
$app = "$clean\mimi.exe"
$savedPath = $env:PATH
$env:PATH = "C:\Windows\System32;C:\Windows"
Remove-Item Env:\VULKAN_SDK -ErrorAction SilentlyContinue

function Run-Hidden([string]$Name, [string]$Clips, [int]$Limit, [hashtable]$Extra) {
    $env:MIMI_DATA_DIR = "$env:TEMP\mimi-m8-$Name"
    if (Test-Path $env:MIMI_DATA_DIR) { Remove-Item $env:MIMI_DATA_DIR -Recurse -Force }
    $env:MIMI_SELF_TEST_OUT = "$out\app-$Name.json"
    if (Test-Path $env:MIMI_SELF_TEST_OUT) { Remove-Item $env:MIMI_SELF_TEST_OUT }
    $env:MIMI_SELF_TEST_CLIPS = $Clips
    foreach ($key in $Extra.Keys) { Set-Item "Env:\$key" $Extra[$key] }
    $process = Start-Process -FilePath $app -PassThru
    $started = Get-Date
    $memory = New-Object System.Collections.ArrayList
    $remote = @{}
    $modules = $null
    $family = @($process.Id)
    $seen = @{}
    while (-not $process.HasExited -and ((Get-Date) - $started).TotalSeconds -lt $Limit) {
        $seconds = [math]::Round(((Get-Date) - $started).TotalSeconds, 1)
        try {
            $process.Refresh()
            [void]$memory.Add([pscustomobject]@{ seconds = $seconds; mb = [math]::Round($process.WorkingSet64 / 1MB) })
            if (-not $modules -and $seconds -gt 4) { $modules = $process.Modules | ForEach-Object { $_.FileName } }
        } catch {}
        # The web view that draws the screens runs as child processes, so they count too.
        $all = Get-CimInstance Win32_Process -Property ProcessId, ParentProcessId, Name, CommandLine
        # Every copy of Mimi shares one web view, which may be running under another open Mimi.
        $family += @($all | Where-Object { $_.Name -eq 'msedgewebview2.exe' -and $_.CommandLine -like '*com.mimi.dictation*' } | ForEach-Object { $_.ProcessId })
        $family = @($family | Select-Object -Unique)
        do {
            $before = $family.Count
            $family = @($all | Where-Object { $_.ProcessId -in $family -or $_.ParentProcessId -in $family } | ForEach-Object { $_.ProcessId })
        } while ($family.Count -gt $before)
        $all | Where-Object { $_.ProcessId -in $family } | ForEach-Object { $seen[[string]$_.ProcessId] = $_.Name }
        Get-NetTCPConnection -OwningProcess $family -ErrorAction SilentlyContinue |
            Where-Object { $_.RemoteAddress -notin '0.0.0.0', '::', '127.0.0.1', '::1' } |
            ForEach-Object { $remote["$($_.RemoteAddress):$($_.RemotePort) ($($_.State))"] = $seconds }
        Start-Sleep -Milliseconds 300
    }
    if (-not $process.HasExited) { "$Name TIMED OUT"; Stop-Process -Id $process.Id -Force } else { "$Name exit $($process.ExitCode), $([math]::Round(((Get-Date) - $started).TotalSeconds)) s" }
    foreach ($key in $Extra.Keys) { Remove-Item "Env:\$key" -ErrorAction SilentlyContinue }
    Remove-Item Env:\MIMI_DATA_DIR, Env:\MIMI_SELF_TEST_OUT, Env:\MIMI_SELF_TEST_CLIPS
    @{ memory = $memory; remote = $remote; modules = $modules; seen = $seen }
}

$audio = "$project\test-audio-m6"
"--- clean folder, model released while idle"
$first = Run-Hidden "clean" "$audio\es\a2.wav|7|parakeet|;wait|20;$audio\en\b3.wav|4;wait|3" 120 @{ MIMI_IDLE_RELEASE_SECONDS = "8" }
$first.memory | ConvertTo-Csv -NoTypeInformation | Out-File "$out\memory.csv" -Encoding utf8
"processes watched: $($first.seen.Count)"
$first.seen.Values | Group-Object | ForEach-Object { "  $($_.Count) x $($_.Name)" }
"non-local connections: $($first.remote.Count)"
$first.remote.GetEnumerator() | ForEach-Object { "  $($_.Key) at $($_.Value) s" }
"libraries loaded from the clean folder:"
$first.modules | Where-Object { $_ -match 'vulkan-1|directml|vcruntime|msvcp' } | ForEach-Object { "  $_" }

"--- Whisper Turbo: released, then reloaded"
$turbo = Run-Hidden "reload-whisper" "$project\test-audio\ja\000.wav|4|whisper-turbo|ja;wait|15;$project\test-audio\ja\001.wav|4" 240 @{ MIMI_IDLE_RELEASE_SECONDS = "6" }
"non-local connections: $($turbo.remote.Count)"

"--- no graphics driver"
$none = Run-Hidden "no-driver" "$project\test-audio\ja\000.wav|4|whisper-small|ja" 240 @{ VK_DRIVER_FILES = "C:\no-such-driver.json"; VK_ICD_FILENAMES = "C:\no-such-driver.json" }
"non-local connections: $($none.remote.Count)"

$env:PATH = $savedPath
Remove-Item $clean -Recurse -Force -ErrorAction SilentlyContinue
foreach ($name in "clean", "reload-whisper", "no-driver") {
    $report = Get-Content "$out\app-$name.json" -Raw -Encoding UTF8 | ConvertFrom-Json
    "--- $name"
    foreach ($item in $report.dictations) {
        $d = $item.dictation
        "  [$([math]::Round($d.timings.stop_to_text_ms)) ms] $($d.text)"
    }
    "  engine: $($report.engine | ConvertTo-Json -Compress)"
    "  screen: $($report.screen | ConvertTo-Json -Compress)"
}
