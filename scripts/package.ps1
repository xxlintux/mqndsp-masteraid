#Requires -Version 5.1
<#
.SYNOPSIS
    Builds the Masteraid CLAP and VST3 plugins and packages them together.

.DESCRIPTION
    Flattens the VST3 build output so `target/bundled/` holds Masteraid.clap and a
    single-file Masteraid.vst3 side by side, then rewrites `target/bundled/bundled.zip`
    from those two files.

    Also produces `dist/MQN-Masteraid-<version>-win64/` and a matching zip containing
    Masteraid.clap and Masteraid.vst3 at the top level.

.PARAMETER SkipBuild
    Package the existing `target/bundled` artifacts without rebuilding them.

.PARAMETER Deploy
    After packaging, install both plugins into the machine-wide and per-user plugin folders
    with `scripts\deploy.ps1`.

.EXAMPLE
    .\scripts\package.ps1 -Deploy
#>
[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [switch]$Deploy
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    if (-not $SkipBuild) {
        # The previous run flattened the VST3 into a file, but xtask needs that path free to
        # create its bundle directory.
        $previousVst3 = Join-Path $root "target\bundled\Masteraid.vst3"
        if (Test-Path $previousVst3 -PathType Leaf) {
            Remove-Item $previousVst3 -Force
        }

        Write-Host "Building CLAP and VST3 bundles..."
        & cargo xtask bundle masteraid --release
        if ($LASTEXITCODE -ne 0) {
            throw "cargo xtask bundle failed with exit code $LASTEXITCODE."
        }
    }

    $cargoToml = Get-Content (Join-Path $root "Cargo.toml") -Raw
    if ($cargoToml -notmatch '(?m)^version\s*=\s*"([^"]+)"') {
        throw "Could not read the package version from Cargo.toml."
    }
    $version = $Matches[1]

    $bundled = Join-Path $root "target\bundled"
    $clapSource = Join-Path $bundled "Masteraid.clap"
    $vst3Source = Join-Path $bundled "Masteraid.vst3"

    # Older builds used a lowercase CLAP name; drop that leftover only.
    Get-ChildItem $bundled -File -Force -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -ceq "masteraid.clap" } |
        Remove-Item -Force

    # cargo xtask always emits a VST3 bundle directory named after the package
    # (Contents/<arch>/<name>.vst3). Windows hosts load the DLL directly, so move
    # it up into target\bundled and delete the wrapper directory.
    $vst3BundleDir = Get-ChildItem $bundled -Directory -Force -ErrorAction SilentlyContinue |
        Where-Object { $_.Extension -eq ".vst3" } |
        Select-Object -First 1

    if ($vst3BundleDir) {
        $nestedVst3 = Get-ChildItem (Join-Path $vst3BundleDir.FullName "Contents") -Recurse -File -Filter *.vst3 -ErrorAction SilentlyContinue |
            Select-Object -First 1
        if ($nestedVst3) {
            $temp = Join-Path $bundled "Masteraid.vst3.tmp"
            Remove-Item $temp -Force -ErrorAction SilentlyContinue
            if (Test-Path $vst3Source -PathType Leaf) {
                Remove-Item $vst3Source -Force
            }
            Move-Item $nestedVst3.FullName $temp
            Remove-Item $vst3BundleDir.FullName -Recurse -Force
            Move-Item $temp $vst3Source
        }
    }

    foreach ($source in @($clapSource, $vst3Source)) {
        if (-not (Test-Path $source -PathType Leaf)) {
            throw "Missing build artifact: $source. Run without -SkipBuild first."
        }
    }

    # Replace xtask's nested bundled.zip with one built from the flat files.
    $bundleZip = Join-Path $bundled "bundled.zip"
    Remove-Item $bundleZip -Force -ErrorAction SilentlyContinue
    Compress-Archive -Path $clapSource, $vst3Source -DestinationPath $bundleZip -CompressionLevel Optimal

    $packageName = "MQN-Masteraid-$version-win64"
    $distRoot = Join-Path $root "dist"
    $stage = Join-Path $distRoot $packageName
    $zip = Join-Path $distRoot "$packageName.zip"

    if (Test-Path $stage) {
        Remove-Item $stage -Recurse -Force
    }
    if (Test-Path $zip) {
        Remove-Item $zip -Force
    }
    New-Item -ItemType Directory -Path $stage -Force | Out-Null

    Copy-Item $clapSource (Join-Path $stage "Masteraid.clap")
    Copy-Item $vst3Source (Join-Path $stage "Masteraid.vst3")

    $readme = @"
MQN Masteraid $version (Windows x64)

Contents
  install.ps1     - one-click installer for both formats
  Masteraid.clap  - CLAP plugin
  Masteraid.vst3  - VST3 plugin (single-file Windows DLL)

Install
  Easiest: right-click install.ps1 -> Run with PowerShell. It installs both
  formats for all users and asks for administrator rights once.

  Manually:
  1. Copy Masteraid.clap to your CLAP folder, usually:
     C:\Program Files\Common Files\CLAP
  2. Copy Masteraid.vst3 to your VST3 folder, usually:
     C:\Program Files\Common Files\VST3
  3. Rescan plugins in your host.

Install both formats. If your host also scans per-user folders, use these
instead (or as well), and make sure no older build is left behind:
  %LOCALAPPDATA%\Programs\Common\CLAP
  %LOCALAPPDATA%\Programs\Common\VST3

The VST3 is shipped as a single .vst3 file instead of the nested
Masteraid.vst3\Contents\x86_64-win\ bundle layout. Both forms load on Windows.
"@
    Set-Content -Path (Join-Path $stage "README.txt") -Value $readme -Encoding UTF8

    # A tester should not have to know about CLAP vs VST3 folders. This copies both, asking for
    # administrator rights once, and refreshes any per-user copy that would otherwise shadow the
    # new build in the host's scan order.
    $installer = @'
#Requires -Version 5.1
# Installs MQN Masteraid (CLAP + VST3) for all users.
# Right-click -> Run with PowerShell, or:
#   powershell -NoProfile -ExecutionPolicy Bypass -File .\install.ps1
$ErrorActionPreference = "Stop"

$here = Split-Path -Parent $PSCommandPath
$clap = Join-Path $here "Masteraid.clap"
$vst3 = Join-Path $here "Masteraid.vst3"
foreach ($file in @($clap, $vst3)) {
    if (-not (Test-Path $file -PathType Leaf)) { throw "Missing $file" }
}

$principal = New-Object Security.Principal.WindowsPrincipal(
    [Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host "Requesting administrator rights..."
    Start-Process powershell.exe -Verb RunAs -ArgumentList @(
        '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$PSCommandPath`"")
    exit
}

$clapDir = "C:\Program Files\Common Files\CLAP"
$vst3Dir = "C:\Program Files\Common Files\VST3"
New-Item -ItemType Directory -Path $clapDir -Force | Out-Null
New-Item -ItemType Directory -Path $vst3Dir -Force | Out-Null
Copy-Item $clap (Join-Path $clapDir "Masteraid.clap") -Force
Copy-Item $vst3 (Join-Path $vst3Dir "Masteraid.vst3") -Force
Write-Host "Installed:"
Write-Host "  $(Join-Path $clapDir 'Masteraid.clap')"
Write-Host "  $(Join-Path $vst3Dir 'Masteraid.vst3')"

# Hosts usually scan the per-user folders first. If an older copy is sitting there, it would keep
# loading instead of the one just installed, so refresh it too.
$userCommon = Join-Path $env:LOCALAPPDATA "Programs\Common"
$userClap = Join-Path $userCommon "CLAP\masteraid.clap"
if (Test-Path $userClap) {
    Copy-Item $clap $userClap -Force
    Write-Host "Refreshed $userClap"
}
$userVst3 = Join-Path $userCommon "VST3\masteraid.vst3"
if (Test-Path $userVst3 -PathType Container) {
    $inner = Join-Path $userVst3 "Contents\x86_64-win\masteraid.vst3"
    Copy-Item $vst3 $inner -Force
    Write-Host "Refreshed $inner"
}

Write-Host ""
Write-Host "Done. Rescan plugins in your host."
Read-Host "Press Enter to close"
'@
    Set-Content -Path (Join-Path $stage "install.ps1") -Value $installer -Encoding UTF8

    Write-Host "Packaging $packageName.zip..."
    Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $zip -CompressionLevel Optimal

    Write-Host ""
    Write-Host "Package folder: $stage"
    Write-Host "Package zip:    $zip"

    if ($Deploy) {
        & (Join-Path $PSScriptRoot "deploy.ps1") -Source $bundled
    }
}
finally {
    Pop-Location
}
