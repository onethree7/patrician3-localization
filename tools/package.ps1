# Builds the two release archives into dist\.
#   patrician3-localization-<version>.zip               DLL + localization.ini (all languages)
#   patrician3-localization-<version>-resources-de.zip  German menu images and textures.ini
# Run from the repository root:  powershell -File tools\package.ps1
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$version = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value
$target = 'i686-pc-windows-msvc'
cargo build --release --locked --target $target
if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }

$dll = Join-Path $root "target\$target\release\patrician3_localization.dll"
$dist = Join-Path $root 'dist'
New-Item -ItemType Directory -Force -Path $dist | Out-Null

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

function New-Zip([string]$zipPath, [hashtable]$entries) {
    if (Test-Path $zipPath) { Remove-Item $zipPath }
    $zip = [System.IO.Compression.ZipFile]::Open($zipPath, 'Create')
    try {
        foreach ($name in ($entries.Keys | Sort-Object)) {
            [void][System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $entries[$name], $name, 'Optimal')
        }
    } finally { $zip.Dispose() }
}

$core = @{
    'mods/patrician3_localization.dll' = $dll
    'mods/localization.ini'            = Join-Path $root 'tools\localization.ini'
}
New-Zip (Join-Path $dist "patrician3-localization-$version.zip") $core

$res = @{}
$resRoot = Join-Path $root 'resources\de'
Get-ChildItem -Path $resRoot -Recurse -File | ForEach-Object {
    $rel = $_.FullName.Substring($resRoot.Length + 1).Replace('\', '/')
    $res[$rel] = $_.FullName
}
New-Zip (Join-Path $dist "patrician3-localization-$version-resources-de.zip") $res

Get-ChildItem $dist -Filter *.zip | Select-Object Name, Length
