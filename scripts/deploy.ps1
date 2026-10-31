#Requires -Version 5.1
<#
.SYNOPSIS
    Installs the freshly built CLAP and VST3 plugins into every standard Windows plugin folder.

.DESCRIPTION
    A host loads whichever copy it finds first, and `%LOCALAPPDATA%\Programs\Common` is scanned
    before `C:\Program Files\Common Files` in most hosts. Leaving an old build in either place is
    how a "fix" appears not to work: the DAW keeps loading the stale binary. This script writes the
    same build to all of them, then verifies that every copy hashes to the same bytes as the build
    output.

    The per-user folders are writable without elevation. The machine-wide folders are not, so a
    denied copy is reported with the exact command to run in an elevated shell instead of failing
    the whole deploy.

.PARAMETER Source
    Folder holding Masteraid.clap and Masteraid.vst3. Defaults to target\bundled.

.EXAMPLE
    .\scripts\deploy.ps1
#>
[CmdletBinding()]
param(
    [string]$Source = "",
    [switch]$ClapOnly,
    [switch]$Vst3Only
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($Source)) {
    $Source = Join-Path $root "target\bundled"
}

$srcClap = Join-Path $Source "Masteraid.clap"
$srcVst3 = Join-Path $Source "Masteraid.vst3"
if (Test-Path $srcVst3 -PathType Container) {
    $innerVst3 = Join-Path $srcVst3 "Contents\x86_64-win\Masteraid.vst3"
    if (Test-Path $innerVst3 -PathType Leaf) {
        $srcVst3 = $innerVst3
    }
}
foreach ($file in @($srcClap, $srcVst3)) {
    if (-not (Test-Path $file -PathType Leaf)) {
        throw "Missing build artifact: $file. Run scripts\package.ps1 first."
    }
}

$localCommon = Join-Path $env:LOCALAPPDATA "Programs\Common"

# The per-user VST3 is normally a bundle directory. Keep that layout if it is already there,
# otherwise write a flat single-file .vst3, which Windows also accepts.
$userVst3Bundle = Join-Path $localCommon "VST3\masteraid.vst3"
$userVst3Target = if (Test-Path $userVst3Bundle -PathType Container) {
    Join-Path $userVst3Bundle "Contents\x86_64-win\masteraid.vst3"
} else {
    Join-Path $localCommon "VST3\masteraid.vst3"
}

$targets = @(
    [pscustomobject]@{ Name = "VST3 (machine)";  Path = "C:\Program Files\Common Files\VST3\Masteraid.vst3"; Source = $srcVst3; Elevated = $true  },
    [pscustomobject]@{ Name = "VST3 (per-user)"; Path = $userVst3Target;                                          Source = $srcVst3; Elevated = $false },
    [pscustomobject]@{ Name = "CLAP (machine)";  Path = "C:\Program Files\Common Files\CLAP\Masteraid.clap";     Source = $srcClap; Elevated = $true  },
    [pscustomobject]@{ Name = "CLAP (per-user)"; Path = (Join-Path $localCommon "CLAP\masteraid.clap");          Source = $srcClap; Elevated = $false }
)

if ($ClapOnly) {
    $targets = @($targets | Where-Object { $_.Name -like "CLAP*" })
} elseif ($Vst3Only) {
    $targets = @($targets | Where-Object { $_.Name -like "VST3*" })
}

$sourceHashes = @{
    $srcVst3 = (Get-FileHash $srcVst3 -Algorithm SHA256).Hash
    $srcClap = (Get-FileHash $srcClap -Algorithm SHA256).Hash
}

$denied = New-Object System.Collections.Generic.List[object]

foreach ($target in $targets) {
    $dir = Split-Path -Parent $target.Path
    if (-not (Test-Path $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }
    try {
        Copy-Item -LiteralPath $target.Source -Destination $target.Path -Force -ErrorAction Stop
        $target | Add-Member -NotePropertyName Status -NotePropertyValue "installed"
    } catch [System.UnauthorizedAccessException] {
        $target | Add-Member -NotePropertyName Status -NotePropertyValue "needs elevation"
        $denied.Add($target)
    } catch {
        $target | Add-Member -NotePropertyName Status -NotePropertyValue "failed: $($_.Exception.Message)"
    }
}

Write-Host ""
Write-Host "Deployed build:"
Write-Host ("  CLAP  {0}" -f $sourceHashes[$srcClap])
Write-Host ("  VST3  {0}" -f $sourceHashes[$srcVst3])
Write-Host ""

$rows = foreach ($target in $targets) {
    $hash = if (Test-Path $target.Path -PathType Leaf) {
        (Get-FileHash $target.Path -Algorithm SHA256).Hash
    } else {
        ""
    }
    $matches = $hash -eq $sourceHashes[$target.Source]
    [pscustomobject]@{
        Format = $target.Name
        Status = $target.Status
        Current = if ($hash) { if ($matches) { "yes" } else { "NO - stale" } } else { "-" }
        Path = $target.Path
    }
}

$rows | Format-Table -AutoSize | Out-String -Width 200 | Write-Host

if ($denied.Count -gt 0) {
    Write-Host "These copies need an elevated shell (one command each):" -ForegroundColor Yellow
    foreach ($target in $denied) {
        Write-Host ("  Copy-Item '{0}' '{1}' -Force" -f $target.Source, $target.Path) -ForegroundColor Yellow
    }
    Write-Host ""
}

$stale = $rows | Where-Object { $_.Current -eq "NO - stale" }
if ($stale) {
    Write-Warning "At least one installed copy does not match the build. A host may still load it."
    exit 1
}
