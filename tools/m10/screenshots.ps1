# Pictures of the built screens with simulated data. Headless Edge draws them with no window,
# so nothing on the desktop is touched. Usage: m10screenshots.ps1 <output folder>
param([string]$Out)
$project = "C:\Users\Luiso Uribe\Documents\Claude\DictatorApp"
$edge = "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
if (-not (Test-Path $edge)) { $edge = "C:\Program Files\Microsoft\Edge\Application\msedge.exe" }
New-Item -ItemType Directory -Force $Out | Out-Null
$server = Start-Process -FilePath python -ArgumentList @("-X", "utf8", "-I", "`"$project\tools\m5\serve_screens.py`"", "`"$project\mimi\dist`"") -WindowStyle Hidden -PassThru
Start-Sleep -Seconds 1
$profile = "$env:TEMP\mimi-m10-edge"
$shots = @(
    # The first picture after starting often misses the view change, so it is only a warm-up.
    @("warm-up", "?view=settings", 1280, 720),
    @("settings", "?view=settings", 1280, 720),
    @("settings-smallest", "?view=settings", 1100, 600)
)
foreach ($shot in $shots) {
    $name, $address, $width, $height = $shot
    $arguments = @("--headless=new", "--disable-gpu-sandbox", "--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--hide-scrollbars",
        "--user-data-dir=`"$profile`"", "--window-size=$width,$height", "--virtual-time-budget=3000",
        "--screenshot=`"$Out\$name.png`"", "`"http://127.0.0.1:1421/$address`"")
    $edgeRun = Start-Process -FilePath $edge -ArgumentList $arguments -WindowStyle Hidden -PassThru
    if (-not $edgeRun.WaitForExit(45000)) { "$name TIMED OUT"; Stop-Process -Id $edgeRun.Id -Force }
    if (Test-Path "$Out\$name.png") { "$name ok" } else { "$name MISSING" }
}
Stop-Process -Id $server.Id -Force
Remove-Item $profile -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item "$Out\warm-up.png" -ErrorAction SilentlyContinue
