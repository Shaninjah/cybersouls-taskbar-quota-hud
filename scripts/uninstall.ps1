[CmdletBinding()]
param(
    [switch]$RemoveSettings,
    [switch]$Quiet
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Assert-NoReparsePoint {
    param([Parameter(Mandatory = $true)][string]$Path)
    $current = [IO.Path]::GetFullPath($Path)
    while ($current) {
        $item = Get-Item -LiteralPath $current -Force -ErrorAction SilentlyContinue
        if ($item) {
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
                throw 'Refusing to modify a path containing a symbolic link or junction.'
            }
        }
        $parent = Split-Path -Parent $current
        if ($parent -eq $current) { break }
        $current = $parent
    }
}

function Assert-SafeRemovalTree {
    param([Parameter(Mandatory = $true)][string]$Path)
    Assert-NoReparsePoint -Path $Path
    if (Test-Path -LiteralPath $Path -PathType Container) {
        foreach ($item in Get-ChildItem -LiteralPath $Path -Force) {
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
                throw 'Refusing recursive removal of a tree containing a symbolic link or junction.'
            }
            if ($item.PSIsContainer) { Assert-SafeRemovalTree -Path $item.FullName }
        }
    }
}

foreach ($root in @($env:LOCALAPPDATA, $env:APPDATA)) {
    if ([string]::IsNullOrWhiteSpace($root) -or -not [IO.Path]::IsPathRooted($root)) {
        throw 'A valid absolute per-user application data directory is required.'
    }
}

$InstallDirectory = Join-Path $env:LOCALAPPDATA 'Programs\CybersoulsTaskbarQuotaHUD'
$ExpectedInstallDirectory = [IO.Path]::GetFullPath($InstallDirectory).TrimEnd('\')
$TargetPath = Join-Path $ExpectedInstallDirectory 'cybersouls-taskbar-quota-hud.exe'
$ShortcutPath = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Cybersouls Taskbar Quota HUD.lnk'
$DesktopShortcutPath = Join-Path ([Environment]::GetFolderPath('Desktop')) 'Cybersouls Taskbar Quota HUD.lnk'
$UninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\CybersoulsTaskbarQuotaHUD'
$RunKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$SettingsDirectory = Join-Path $env:APPDATA 'CodexUsage'

$AllowedRoot = [IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'Programs')).TrimEnd('\')
if (-not $ExpectedInstallDirectory.StartsWith($AllowedRoot + '\', [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to remove unexpected install directory: $ExpectedInstallDirectory"
}
Assert-SafeRemovalTree -Path $ExpectedInstallDirectory
if ($RemoveSettings) { Assert-SafeRemovalTree -Path $SettingsDirectory }

Get-CimInstance Win32_Process -Filter "Name='cybersouls-taskbar-quota-hud.exe'" -ErrorAction SilentlyContinue |
    Where-Object { $_.ExecutablePath -eq $TargetPath } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force }

if (Test-Path -LiteralPath $RunKey) {
    # A portable/upstream copy may have taken ownership of the shared startup key.
    $StartupTarget = (Get-ItemProperty -Path $RunKey -ErrorAction SilentlyContinue).PSObject.Properties['CodexUsage']
    if ($StartupTarget -and $StartupTarget.Value.Trim('"') -eq $TargetPath) {
        Remove-ItemProperty -Path $RunKey -Name 'CodexUsage' -ErrorAction SilentlyContinue
    }
}

Remove-Item -LiteralPath $ShortcutPath -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $DesktopShortcutPath -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $UninstallKey -Recurse -Force -ErrorAction SilentlyContinue

if ($RemoveSettings) {
    $ExpectedSettingsDirectory = [IO.Path]::GetFullPath((Join-Path $env:APPDATA 'CodexUsage')).TrimEnd('\')
    $ResolvedSettingsDirectory = [IO.Path]::GetFullPath($SettingsDirectory).TrimEnd('\')
    if ($ResolvedSettingsDirectory -eq $ExpectedSettingsDirectory) {
        Remove-Item -LiteralPath $ResolvedSettingsDirectory -Recurse -Force -ErrorAction SilentlyContinue
    }
}

if (Test-Path -LiteralPath $ExpectedInstallDirectory -PathType Container) {
    Remove-Item -LiteralPath $ExpectedInstallDirectory -Recurse -Force
}

if (-not $Quiet) {
    Write-Output 'Cybersouls Taskbar Quota HUD was uninstalled.'
    if (-not $RemoveSettings) {
        Write-Output "Settings were preserved at $SettingsDirectory"
    }
}
