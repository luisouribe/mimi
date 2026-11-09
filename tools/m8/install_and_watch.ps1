# Closes Mimi, installs it for this user, opens it, and watches its internet connections from the first moment.
# Run this only after agreeing it with the owner, because it adds a desktop icon and opens a window.
# Usage: install_and_watch.ps1 <seconds to watch>
param([int]$Seconds = 120)
$project = "C:\Users\Luiso Uribe\Documents\Claude\DictatorApp"
$setup = "C:\mimi-build\app\release\bundle\nsis\Mimi_0.1.0_x64-setup.exe"
$installed = "$env:LOCALAPPDATA\Mimi\mimi.exe"
$log = "$project\proof\M8\network-first-launch.txt"

function Count-Data {
    python -X utf8 -I -c "import os,sqlite3; p=os.path.join(os.environ['LOCALAPPDATA'],'Mimi','history.db').replace(chr(92),'/'); c=sqlite3.connect('file:/'+p+'?mode=ro',uri=True); print(c.execute('select count(*) from dictations').fetchone()[0], 'dictations,', c.execute('select count(*) from snippets').fetchone()[0], 'snippets,', c.execute('pragma quick_check').fetchone()[0])"
}
"before: $(Count-Data)"
Get-Process mimi -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.Id -Force; [void]$_.WaitForExit(10000) }
# The web view closes shortly after Mimi, and it must start fresh for this test to mean anything.
$deadline = (Get-Date).AddSeconds(20)
while ((Get-Date) -lt $deadline -and @(Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" | Where-Object { $_.CommandLine -like '*com.mimi.dictation*' }).Count -gt 0) { Start-Sleep -Milliseconds 300 }
"web view processes left: $(@(Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" | Where-Object { $_.CommandLine -like '*com.mimi.dictation*' }).Count)"
$started = Get-Date
$install = Start-Process -FilePath $setup -ArgumentList "/S" -PassThru
[void]$install.WaitForExit(300000)
"installer exit $($install.ExitCode) in $([math]::Round(((Get-Date) - $started).TotalSeconds)) s"
"installed: $(Test-Path $installed)"
"desktop icon: $(Test-Path "$([Environment]::GetFolderPath('Desktop'))\Mimi.lnk")"
"after install: $(Count-Data)"
if (-not (Test-Path $installed)) { exit 1 }
$app = Start-Process -FilePath $installed -WorkingDirectory (Split-Path $installed) -PassThru
$remote = @{}
$seen = @{}
$watchStart = Get-Date
while (((Get-Date) - $watchStart).TotalSeconds -lt $Seconds) {
    $all = Get-CimInstance Win32_Process -Property ProcessId, Name, CommandLine
    $family = @($all | Where-Object { ($_.Name -eq 'msedgewebview2.exe' -and $_.CommandLine -like '*com.mimi.dictation*') -or $_.Name -eq 'mimi.exe' })
    $family | ForEach-Object { $seen[[string]$_.ProcessId] = $_.Name }
    $ids = @($family | ForEach-Object { $_.ProcessId })
    if ($ids.Count) {
        Get-NetTCPConnection -OwningProcess $ids -ErrorAction SilentlyContinue |
            Where-Object { $_.RemoteAddress -notin '0.0.0.0', '::', '127.0.0.1', '::1' } |
            ForEach-Object {
                $key = "$($_.RemoteAddress):$($_.RemotePort)"
                if (-not $remote.ContainsKey($key)) { $remote[$key] = "first seen at $([math]::Round(((Get-Date) - $watchStart).TotalSeconds, 1)) s" }
            }
    }
    Start-Sleep -Milliseconds 200
}
$lines = @("Watched $Seconds s from launch. Processes: $($seen.Count). Non-local connections: $($remote.Count).")
$remote.GetEnumerator() | ForEach-Object { $lines += "$($_.Key) at $($_.Value) s" }
$lines | Out-File $log -Encoding utf8
$lines
"running: $(@(Get-Process mimi -ErrorAction SilentlyContinue | ForEach-Object { $_.Path }) -join ', ')"
