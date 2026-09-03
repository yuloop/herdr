[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [string]$HerdrExe,
    [string]$IconPath,
    [string]$StartingDirectory = [Environment]::GetFolderPath("UserProfile"),
    [string]$SettingsPath,
    [string]$ProfileName = "Herdr",
    [string]$ProfileGuid = "{e507d62a-bb8f-44b0-9c69-22625383acf3}",
    [switch]$Elevate,
    [switch]$NoElevate,
    [switch]$SetDefault
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$scriptDirectory = $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($scriptDirectory)) {
    $scriptDirectory = Split-Path -Parent $MyInvocation.MyCommand.Path
}
if ([string]::IsNullOrWhiteSpace($scriptDirectory)) {
    throw "Could not determine the terminal profile installer directory"
}
if ([string]::IsNullOrWhiteSpace($HerdrExe)) {
    $HerdrExe = Join-Path $scriptDirectory "herdr.exe"
}
if ([string]::IsNullOrWhiteSpace($IconPath)) {
    $IconPath = Join-Path $scriptDirectory "assets\herdr.png"
}
if ($Elevate -and $NoElevate) {
    throw "-Elevate and -NoElevate cannot be used together"
}
# The packaged Windows profile is elevated by default. Windows Terminal can
# keep opening elevated profiles as tabs when the owning window is already
# elevated; top-level window reuse is controlled separately by
# windowingBehavior below. -Elevate remains an explicit, idempotent opt-in and
# -NoElevate is the supported override.
$profileElevated = -not [bool]$NoElevate

function Get-JsonProperty {
    param(
        [Parameter(Mandatory = $true)]
        [object]$Object,
        [Parameter(Mandatory = $true)]
        [string]$Name
    )

    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $null
    }
    return $property.Value
}

function Set-JsonProperty {
    param(
        [Parameter(Mandatory = $true)]
        [object]$Object,
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [AllowNull()]
        [object]$Value
    )

    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        $Object | Add-Member -MemberType NoteProperty -Name $Name -Value $Value
    } else {
        $property.Value = $Value
    }
}

function Get-FileSha256 {
    param([Parameter(Mandatory = $true)][string]$Path)

    $stream = [System.IO.File]::OpenRead($Path)
    $algorithm = [System.Security.Cryptography.SHA256]::Create()
    try {
        return ([BitConverter]::ToString($algorithm.ComputeHash($stream))).Replace("-", "")
    } finally {
        $algorithm.Dispose()
        $stream.Dispose()
    }
}

function Find-WindowsTerminalSettings {
    $localAppData = [Environment]::GetFolderPath("LocalApplicationData")
    $delegationGuid = $null
    try {
        $delegationGuid = (Get-ItemProperty -LiteralPath "HKCU:\Console\%%Startup" -ErrorAction Stop).DelegationTerminal
    } catch {
        # Fall through to installed-package preference when no explicit delegation exists.
    }

    $packages = @(Get-AppxPackage -Name "Microsoft.WindowsTerminal*" -ErrorAction SilentlyContinue)
    if ($null -ne $delegationGuid) {
        foreach ($package in $packages) {
            $manifest = Join-Path $package.InstallLocation "AppxManifest.xml"
            if ((Test-Path -LiteralPath $manifest -PathType Leaf) -and
                ([System.IO.File]::ReadAllText($manifest)).IndexOf(
                    [string]$delegationGuid,
                    [StringComparison]::OrdinalIgnoreCase
                ) -ge 0) {
                $candidate = Join-Path $localAppData "Packages\$($package.PackageFamilyName)\LocalState\settings.json"
                if (Test-Path -LiteralPath $candidate -PathType Leaf) {
                    return $candidate
                }
            }
        }
    }

    foreach ($family in @(
        "Microsoft.WindowsTerminalPreview_8wekyb3d8bbwe",
        "Microsoft.WindowsTerminal_8wekyb3d8bbwe"
    )) {
        $candidate = Join-Path $localAppData "Packages\$family\LocalState\settings.json"
        if (Test-Path -LiteralPath $candidate -PathType Leaf) {
            return $candidate
        }
    }

    $unpackaged = Join-Path $localAppData "Microsoft\Windows Terminal\settings.json"
    if (Test-Path -LiteralPath $unpackaged -PathType Leaf) {
        return $unpackaged
    }

    throw "Could not find Windows Terminal settings.json"
}

if ([string]::IsNullOrWhiteSpace($SettingsPath)) {
    $SettingsPath = Find-WindowsTerminalSettings
}

foreach ($path in @($HerdrExe, $IconPath, $StartingDirectory, $SettingsPath)) {
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Required path does not exist: $path"
    }
}
if (-not (Test-Path -LiteralPath $HerdrExe -PathType Leaf)) {
    throw "Herdr executable is not a file: $HerdrExe"
}
if (-not (Test-Path -LiteralPath $IconPath -PathType Leaf)) {
    throw "Herdr icon is not a file: $IconPath"
}
if (-not (Test-Path -LiteralPath $StartingDirectory -PathType Container)) {
    throw "Starting directory is not a directory: $StartingDirectory"
}
if (-not (Test-Path -LiteralPath $SettingsPath -PathType Leaf)) {
    throw "Windows Terminal settings path is not a file: $SettingsPath"
}

$HerdrExe = (Resolve-Path -LiteralPath $HerdrExe).Path
$IconPath = (Resolve-Path -LiteralPath $IconPath).Path
$StartingDirectory = (Resolve-Path -LiteralPath $StartingDirectory).Path
$SettingsPath = (Resolve-Path -LiteralPath $SettingsPath).Path

$originalHash = Get-FileSha256 -Path $SettingsPath
$rawSettings = [System.IO.File]::ReadAllText($SettingsPath)
try {
    $settings = $rawSettings | ConvertFrom-Json
} catch {
    throw "Windows Terminal settings are not valid JSON; refusing to rewrite $SettingsPath`: $($_.Exception.Message)"
}
if ($null -eq $settings) {
    throw "Windows Terminal settings are empty: $SettingsPath"
}

$profiles = Get-JsonProperty -Object $settings -Name "profiles"
if ($null -eq $profiles) {
    $profiles = [pscustomobject][ordered]@{
        defaults = [pscustomobject]@{}
        list = @()
    }
    Set-JsonProperty -Object $settings -Name "profiles" -Value $profiles
}

$profileList = @(Get-JsonProperty -Object $profiles -Name "list")
$keptProfiles = New-Object System.Collections.ArrayList
$herdrProfile = $null
foreach ($profile in $profileList) {
    $guid = Get-JsonProperty -Object $profile -Name "guid"
    $name = Get-JsonProperty -Object $profile -Name "name"
    $isHerdr = ([string]$guid).Equals($ProfileGuid, [StringComparison]::OrdinalIgnoreCase) -or
        ([string]$name).Equals($ProfileName, [StringComparison]::OrdinalIgnoreCase)
    if ($isHerdr) {
        if ($null -eq $herdrProfile) {
            $herdrProfile = $profile
        }
        continue
    }
    [void]$keptProfiles.Add($profile)
}

if ($null -eq $herdrProfile) {
    $herdrProfile = [pscustomobject][ordered]@{}
}
Set-JsonProperty -Object $herdrProfile -Name "guid" -Value $ProfileGuid
Set-JsonProperty -Object $herdrProfile -Name "name" -Value $ProfileName
Set-JsonProperty -Object $herdrProfile -Name "commandline" -Value ('"' + $HerdrExe + '"')
Set-JsonProperty -Object $herdrProfile -Name "startingDirectory" -Value $StartingDirectory
Set-JsonProperty -Object $herdrProfile -Name "icon" -Value $IconPath
Set-JsonProperty -Object $herdrProfile -Name "hidden" -Value $false
Set-JsonProperty -Object $herdrProfile -Name "elevate" -Value $profileElevated
Set-JsonProperty -Object $herdrProfile -Name "tabTitle" -Value $ProfileName
Set-JsonProperty -Object $herdrProfile -Name "suppressApplicationTitle" -Value $true
[void]$keptProfiles.Add($herdrProfile)
Set-JsonProperty -Object $profiles -Name "list" -Value @($keptProfiles.ToArray())

if ($SetDefault) {
    Set-JsonProperty -Object $settings -Name "defaultProfile" -Value $ProfileGuid
}
# Windows Terminal's persisted-layout modes restore every saved Terminal
# window. Herdr restores only its owning window, so keep Terminal itself on the
# single-default-profile startup path.
Set-JsonProperty -Object $settings -Name "firstWindowPreference" -Value "defaultProfile"
# Without this property Windows Terminal defaults to useNew, so starting Herdr
# again creates another top-level window even when an eligible Terminal window
# already exists. Reuse the most recently used window across virtual desktops.
Set-JsonProperty -Object $settings -Name "windowingBehavior" -Value "useAnyExisting"

$updatedJson = ($settings | ConvertTo-Json -Depth 100) + [Environment]::NewLine
$null = $updatedJson | ConvertFrom-Json

$settingsDirectory = Split-Path -Parent $SettingsPath
$temporaryPath = Join-Path $settingsDirectory ("settings.json.herdr.tmp.{0}" -f $PID)
try {
    if ($PSCmdlet.ShouldProcess($SettingsPath, "Install the $ProfileName Windows Terminal profile")) {
        $currentHash = Get-FileSha256 -Path $SettingsPath
        if ($currentHash -ne $originalHash) {
            throw "Windows Terminal settings changed while the profile was being prepared; retry the command"
        }
        [System.IO.File]::WriteAllText(
            $temporaryPath,
            $updatedJson,
            (New-Object System.Text.UTF8Encoding($false))
        )
        Move-Item -LiteralPath $temporaryPath -Destination $SettingsPath -Force
    }
} finally {
    Remove-Item -LiteralPath $temporaryPath -Force -ErrorAction SilentlyContinue
}

[pscustomobject][ordered]@{
    settings_path = $SettingsPath
    profile_name = $ProfileName
    profile_guid = $ProfileGuid
    commandline = '"' + $HerdrExe + '"'
    starting_directory = $StartingDirectory
    icon = $IconPath
    elevate = $profileElevated
    terminal_all_window_restore = $false
    terminal_windowing_behavior = "useAnyExisting"
    default_profile = [bool]$SetDefault
} | ConvertTo-Json -Compress
