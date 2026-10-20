$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$repo = 'Open-Stream-Deck/Open-Stream-Deck-SDK'
$version = if ($env:OPENSD_VERSION) { $env:OPENSD_VERSION } else { 'latest' }
$dir = if ($env:OPENSD_INSTALL_DIR) { $env:OPENSD_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'opensd\bin' }

$arch = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64' -or $env:PROCESSOR_ARCHITEW6432 -eq 'ARM64') { 'aarch64' } else { 'x86_64' }
$archive = "opensd-$arch-pc-windows-msvc.zip"
$base = if ($version -eq 'latest') {
    "https://github.com/$repo/releases/latest/download"
} else {
    "https://github.com/$repo/releases/download/v$($version.TrimStart('v'))"
}

[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$work = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $work | Out-Null

try {
    Write-Host "Downloading $archive"
    Invoke-WebRequest -UseBasicParsing -Uri "$base/$archive" -OutFile (Join-Path $work $archive)
    Invoke-WebRequest -UseBasicParsing -Uri "$base/SHA256SUMS" -OutFile (Join-Path $work 'SHA256SUMS')

    $line = Get-Content (Join-Path $work 'SHA256SUMS') | Where-Object { $_ -match " $([Regex]::Escape($archive))$" }
    if (-not $line) { throw "$archive is not listed in SHA256SUMS" }
    $expected = ($line -split ' ')[0]
    $actual = (Get-FileHash -Algorithm SHA256 (Join-Path $work $archive)).Hash.ToLower()
    if ($expected -ne $actual) { throw "Checksum mismatch for $archive" }

    Expand-Archive -Path (Join-Path $work $archive) -DestinationPath (Join-Path $work 'out')
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    Copy-Item -Force (Join-Path $work 'out\opensd.exe') (Join-Path $dir 'opensd.exe')
} finally {
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
$entries = if ($userPath) { $userPath -split ';' } else { @() }
if ($entries -notcontains $dir) {
    $updated = if ($userPath) { "$userPath;$dir" } else { $dir }
    [Environment]::SetEnvironmentVariable('Path', $updated, 'User')
    Write-Host "Added $dir to the user PATH; open a new terminal to use opensd"
}
$env:Path = "$env:Path;$dir"

Write-Host "Installed $(& (Join-Path $dir 'opensd.exe') --version) to $dir"
