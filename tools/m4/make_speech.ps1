# Makes test dictations with Windows' offline voices. Nothing plays aloud: audio goes straight to files.
# Usage: make_speech.ps1 <cases.tsv> <output folder>
# Each line of cases.tsv is: id <tab> language <tab> voice <tab> text, where "|" marks a pause of 800 ms
# and {300} marks a pause of that many milliseconds.
param([string]$Cases, [string]$Out)
Add-Type -AssemblyName System.Speech
New-Item -ItemType Directory -Force $Out | Out-Null
$format = New-Object System.Speech.AudioFormat.SpeechAudioFormatInfo(16000, [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen, [System.Speech.AudioFormat.AudioChannel]::Mono)
$manifest = @()
foreach ($line in [IO.File]::ReadAllLines($Cases, [Text.Encoding]::UTF8)) {
    if (-not $line.Trim() -or $line.StartsWith("#")) { continue }
    $id, $language, $voice, $text = $line.Split("`t")
    $folder = Join-Path $Out $language
    New-Item -ItemType Directory -Force $folder | Out-Null
    $file = Join-Path $folder "$id.wav"
    $culture = if ($language -eq "es") { "es-MX" } else { "en-US" }
    $escaped = [Security.SecurityElement]::Escape($text).Replace("|", '<break time="800ms"/>') -replace '\{(\d+)\}', '<break time="$1ms"/>'
    $ssml = "<speak version=`"1.0`" xmlns=`"http://www.w3.org/2001/10/synthesis`" xml:lang=`"$culture`"><voice name=`"$voice`">$escaped</voice></speak>"
    $synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
    $synth.SetOutputToWaveFile($file, $format)
    $synth.SpeakSsml($ssml)
    $synth.Dispose()
    $seconds = ((Get-Item $file).Length - 44) / 32000.0
    $manifest += [pscustomobject]@{ lang = $language; file = "$language/$id.wav"; seconds = [math]::Round($seconds, 2); reference = ($text.Replace("|", "/") -replace '\{\d+\}', '/') }
}
[IO.File]::WriteAllText((Join-Path $Out "manifest.json"), (ConvertTo-Json @($manifest) -Depth 3), (New-Object Text.UTF8Encoding($false)))
"made $($manifest.Count) clips in $Out"
