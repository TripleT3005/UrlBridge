[CmdletBinding()]
param(
    [switch]$Zip
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Paths
$root = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $root 'target\release\UrlBridge.exe'
$licenses = Join-Path $root 'THIRD-PARTY-LICENSES.txt'

# Build
Push-Location $root
try {
    & cargo build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'The build failed.' }
}
finally {
    Pop-Location
}

# Package
$version = (Get-Item -LiteralPath $exe).VersionInfo.ProductVersion
$name = "UrlBridge-$version"
$dist = Join-Path $root 'dist'
$package = Join-Path $dist $name
if (Test-Path -LiteralPath $package) {
    Remove-Item -LiteralPath $package -Recurse -Force
}
New-Item -ItemType Directory -Path $package -Force | Out-Null
Copy-Item -LiteralPath $exe -Destination $package
Copy-Item -LiteralPath (Join-Path $root 'install\Install.ps1') -Destination $package
Copy-Item -LiteralPath (Join-Path $root 'install\Uninstall.ps1') -Destination $package
Copy-Item -LiteralPath (Join-Path $root 'config\UrlBridge.example.json') -Destination $package
Copy-Item -LiteralPath (Join-Path $root 'README.md') -Destination $package
Copy-Item -LiteralPath (Join-Path $root 'LICENSE') -Destination (Join-Path $package 'LICENSE.txt')
if (Test-Path -LiteralPath $licenses) {
    Copy-Item -LiteralPath $licenses -Destination $package
}
Write-Host "Package: $package"

# Archive
if ($Zip) {
    $archive = Join-Path $dist "$name-windows-x64.zip"
    if (Test-Path -LiteralPath $archive) {
        Remove-Item -LiteralPath $archive -Force
    }
    Compress-Archive -LiteralPath $package -DestinationPath $archive
    $hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    $checksum = "$hash *$(Split-Path -Leaf $archive)"
    [IO.File]::WriteAllText("$archive.sha256", $checksum, (New-Object Text.ASCIIEncoding))
    Write-Host "Archive: $archive"
}
