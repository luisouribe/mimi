# Copies the system files Mimi needs next to its installer, so it opens on a PC that doesn't have them.
$project = "C:\Users\Luiso Uribe\Documents\Claude\DictatorApp"
$target = "$project\mimi\src-tauri\bundle-files"
$redist = Get-ChildItem "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Redist\MSVC\*\x64\Microsoft.VC143.CRT" | Select-Object -Last 1
foreach ($name in "vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll", "msvcp140_1.dll", "msvcp140_2.dll") {
    Copy-Item (Join-Path $redist.FullName $name) $target -Force
}
$sdk = Get-ChildItem "C:\VulkanSDK" -Directory | Select-Object -Last 1
$vulkan = Get-ChildItem $sdk.FullName -Recurse -Filter "vulkan-1.dll" -ErrorAction SilentlyContinue | Where-Object { $_.FullName -notmatch '32' } | Select-Object -First 1
if (-not $vulkan) { $vulkan = Get-Item "C:\Windows\System32\vulkan-1.dll" }
Copy-Item $vulkan.FullName $target -Force
$directml = Get-ChildItem "C:\mimi-build\app\release" -Filter "DirectML.dll" | Select-Object -First 1
# A previous build may have renamed it while Mimi was open. The copy already here is then kept.
if ($directml) { Copy-Item $directml.FullName $target -Force }
elseif (-not (Test-Path "$target\DirectML.dll")) { throw "DirectML.dll not found. Build Mimi once, then run this again." }
Get-ChildItem $target | Select-Object Name, @{ n = 'MB'; e = { [math]::Round($_.Length / 1MB, 2) } } | Format-Table -AutoSize
"vulkan from $($vulkan.FullName)"
