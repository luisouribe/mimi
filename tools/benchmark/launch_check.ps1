# Opens the built Mimi app, measures how fast its window appears and how much it uses, then closes it.
# It opens a window on the desktop for about 15 seconds, so run it only when that is agreed.
# It never takes focus, presses keys or captures the screen.
param([string]$Exe)

$timer = [Diagnostics.Stopwatch]::StartNew()
$process = Start-Process -FilePath $Exe -PassThru
while ($process.MainWindowHandle -eq 0 -and $timer.Elapsed.TotalSeconds -lt 30) {
  Start-Sleep -Milliseconds 20
  $process.Refresh()
}
"Window appeared after: {0:N2} seconds" -f $timer.Elapsed.TotalSeconds

Start-Sleep -Seconds 5

$all = Get-CimInstance Win32_Process
function Get-Tree([int]$Id) {
  $Id
  $all | Where-Object { $_.ParentProcessId -eq $Id } | ForEach-Object { Get-Tree $_.ProcessId }
}
$ids = Get-Tree $process.Id
$group = Get-Process -Id $ids -ErrorAction SilentlyContinue
$before = ($group | Measure-Object -Property CPU -Sum).Sum
Start-Sleep -Seconds 10
$group = Get-Process -Id $ids -ErrorAction SilentlyContinue
$after = ($group | Measure-Object -Property CPU -Sum).Sum
$memory = ($group | Measure-Object -Property PrivateMemorySize64 -Sum).Sum / 1MB
"Processes (app plus its web view): $($group.Count)"
"Private memory: {0:N0} MB" -f $memory
"Processor use: {0:N2}% of the whole PC" -f (100 * ($after - $before) / 10 / [Environment]::ProcessorCount)

Stop-Process -Id $process.Id -Force
"Closed."
