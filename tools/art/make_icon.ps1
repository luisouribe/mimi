# Makes Mimi's app icon: her smiling face on the coral background, with the design's angled corners.
# Usage: make_icon.ps1 <output.png>
param([string]$Out)
Add-Type -AssemblyName System.Drawing
$project = "C:\Users\Luiso Uribe\Documents\Claude\DictatorApp"
$source = [System.Drawing.Bitmap]::FromFile("$project\design\assets\mimi.png")

# Drawn at twice the size and reduced at the end, which keeps the angled edges smooth.
$size = 2048
$chamfer = 360
$big = New-Object System.Drawing.Bitmap $size, $size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$canvas = [System.Drawing.Graphics]::FromImage($big)
$canvas.Clear([System.Drawing.Color]::Transparent)
$canvas.SmoothingMode = 'HighQuality'
$canvas.InterpolationMode = 'HighQualityBicubic'
$canvas.PixelOffsetMode = 'HighQuality'

$shape = New-Object System.Drawing.Drawing2D.GraphicsPath
$shape.AddPolygon([System.Drawing.Point[]]@(
    (New-Object System.Drawing.Point 0, 0),
    (New-Object System.Drawing.Point ($size - $chamfer), 0),
    (New-Object System.Drawing.Point $size, $chamfer),
    (New-Object System.Drawing.Point $size, $size),
    (New-Object System.Drawing.Point $chamfer, $size),
    (New-Object System.Drawing.Point 0, ($size - $chamfer))
))
$coral = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 255, 143, 150))
$canvas.FillPath($coral, $shape)

# Her face, close enough to still read at the small sizes Windows uses.
$canvas.SetClip($shape)
$face = New-Object System.Drawing.Rectangle 395, 95, 460, 460
$canvas.DrawImage($source, (New-Object System.Drawing.Rectangle 0, 0, $size, $size), $face, [System.Drawing.GraphicsUnit]::Pixel)
$canvas.Dispose()

$final = New-Object System.Drawing.Bitmap 1024, 1024, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$reduce = [System.Drawing.Graphics]::FromImage($final)
$reduce.Clear([System.Drawing.Color]::Transparent)
$reduce.InterpolationMode = 'HighQualityBicubic'
$reduce.PixelOffsetMode = 'HighQuality'
$reduce.DrawImage($big, 0, 0, 1024, 1024)
$reduce.Dispose()
$final.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$final.Dispose()
$big.Dispose()
$source.Dispose()
"icon saved to $Out"
