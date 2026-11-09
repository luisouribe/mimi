# Pictures of the built screens with simulated data. Headless Edge draws them with no window,
# so nothing on the desktop is touched. Usage: screenshots.ps1 <output folder>
param([string]$Out)
$project = "C:\Users\Luiso Uribe\Documents\Claude\DictatorApp"
$edge = "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
if (-not (Test-Path $edge)) { $edge = "C:\Program Files\Microsoft\Edge\Application\msedge.exe" }
New-Item -ItemType Directory -Force $Out | Out-Null
$server = Start-Process -FilePath python -ArgumentList @("-X", "utf8", "-I", "`"$project\tools\m5\serve_screens.py`"", "`"$project\mimi\dist`"") -WindowStyle Hidden -PassThru
Start-Sleep -Seconds 1
$profile = "$env:TEMP\mimi-m5-edge"
$shots = @(
    @("home", "", 1280, 720),
    @("listening", "?stage=listening&level=0.5", 1280, 720),
    @("settings", "?view=settings", 1280, 720),
    @("settings-whisper", "?view=settings&model=whisper-turbo", 1280, 720),
    @("empty", "?empty=1", 1280, 720),
    @("still", "?animate=0", 1280, 720),
    @("smallest", "", 1100, 600),
    @("smallest-settings", "?view=settings", 1100, 600),
    @("large", "", 1920, 1040),
    @("snippets", "?view=snippets", 1280, 720),
    @("snippets-empty", "?view=snippets&empty=1", 1280, 720),
    @("snippets-new", "?view=snippets&edit=1", 1280, 720),
    @("snippets-smallest", "?view=snippets&edit=1", 1100, 600),
    @("search", "?search=juan", 1280, 720),
    @("search-none", "?search=presupuesto", 1280, 720),
    @("delete-confirm", "?confirm=1", 1280, 720),
    @("smallest-home", "", 1100, 600)
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
